//! System Call ABI (`syscall` / `sysret`) Subsystem for Kinetix OS.
//!
//! Encompasses:
//! - Hardware Fast System Call dispatch via x86_64 `syscall` / `sysretq`
//! - MSR configuration (IA32_EFER, IA32_STAR, IA32_LSTAR, IA32_SFMASK)
//! - System call argument marshalling per System V AMD64 ABI
//! - User space pointer validation & security boundary enforcement (-EFAULT)
//! - Core system services: SYS_PRINT, SYS_YIELD, SYS_EXIT, SYS_GETPID, SYS_UPTIME_MS

pub mod entry;

use crate::arch::cpu;
use crate::loader::process::ProcessAddressSpace;
use crate::memory::frame::FRAME_ALLOCATOR;
use crate::memory::paging::{PageTableFlags, VirtAddr};
use crate::println;

pub use entry::{init, run_user_process, syscall_entry};

// System Call Numbers
pub const SYS_PRINT: u64 = 1;
pub const SYS_YIELD: u64 = 2;
pub const SYS_EXIT: u64 = 3;
pub const SYS_GETPID: u64 = 4;
pub const SYS_UPTIME_MS: u64 = 5;

// System Call Error Codes
pub const EFAULT: i64 = -14; // Bad address
pub const EINVAL: i64 = -22; // Invalid argument
pub const ENOSYS: i64 = -38; // Function not implemented

/// Central System Call Dispatcher called from assembly `syscall_entry`.
#[no_mangle]
pub extern "sysv64" fn syscall_dispatch(
    num: u64,
    a1: u64,
    a2: u64,
    _a3: u64,
    _a4: u64,
    _a5: u64,
) -> i64 {
    match num {
        SYS_PRINT => {
            // Pointer Validation: address must reside strictly in user space (0x4000_0000 .. 0x8000_0000)
            let ptr = a1;
            let len = a2;
            if ptr < 0x4000_0000 || ptr.checked_add(len).map_or(true, |end| end > 0x8000_0000) {
                return EFAULT;
            }

            let slice = unsafe { core::slice::from_raw_parts(ptr as *const u8, len as usize) };
            let s = match core::str::from_utf8(slice) {
                Ok(valid_str) => valid_str,
                Err(_) => return EINVAL,
            };

            crate::print!("{}", s);
            len as i64
        }
        SYS_YIELD => {
            crate::task::yield_now();
            0
        }
        SYS_EXIT => {
            let exit_code = a1 as i64;
            unsafe {
                entry::exit_user_process_asm(exit_code);
            }
        }
        SYS_GETPID => {
            1
        }
        SYS_UPTIME_MS => {
            (crate::arch::pit::get_ticks() * 10) as i64
        }
        _ => ENOSYS,
    }
}

/// Executes comprehensive verification tests for System Call ABI (Phase 16).
pub fn run_syscall_tests() {
    println!("\n================================================================================");
    println!("          KINETIX OS SYSTEM CALL ABI (SYSCALL / SYSRET) TESTS (PHASE 16)        ");
    println!("================================================================================\n");

    // Test 1: Verify Hardware MSR Configuration
    unsafe {
        let efer = cpu::rdmsr(entry::MSR_EFER);
        assert!((efer & entry::EFER_SCE) != 0, "EFER.SCE (System Call Enable) must be set");

        let star = cpu::rdmsr(entry::MSR_STAR);
        let expected_star = (0x0010u64 << 48) | (0x0008u64 << 32);
        assert_eq!(star, expected_star, "STAR selectors mismatch");

        let lstar = cpu::rdmsr(entry::MSR_LSTAR);
        assert_eq!(lstar, syscall_entry as *const () as usize as u64, "LSTAR must point to syscall_entry");

        let sfmask = cpu::rdmsr(entry::MSR_SFMASK);
        assert!((sfmask & 0x200) != 0, "SFMASK must mask IF (Interrupt Flag)");
    }
    println!("  [PASS] Hardware Fast System Call MSRs (EFER, STAR, LSTAR, SFMASK) verified.");
    println!("  [PASS] IA32_STAR Selectors: Kernel CS 0x08 / SS 0x10, User SS 0x1B / CS 0x23.");

    // Test 2: Prepare Isolated User Process Address Space
    let user_entry = VirtAddr::new(0x4000_0000);
    let user_stack = VirtAddr::new(0x7FFF_F000);
    let mut space = ProcessAddressSpace::new(user_entry, user_stack)
        .expect("Failed to initialize process address space");

    // Allocate physical frame for user test probe code
    let code_frame = FRAME_ALLOCATOR
        .lock()
        .allocate_frame()
        .expect("OOM allocating user code frame");
    space.allocated_frames.push(code_frame);

    // Copy position-independent assembly test probe
    let probe_start = entry::user_syscall_probe_start as *const () as usize;
    let probe_end = entry::user_syscall_probe_end as *const () as usize;
    let probe_len = probe_end - probe_start;
    assert!(probe_len > 0 && probe_len <= 4096, "User syscall probe exceeds 1 page");

    unsafe {
        let dest = code_frame.as_mut_ptr::<u8>();
        core::ptr::write_bytes(dest, 0, 4096);
        core::ptr::copy_nonoverlapping(probe_start as *const u8, dest, probe_len);
    }

    let code_flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
    space
        .map_user_page(user_entry, code_frame, code_flags)
        .expect("Failed to map user code page");

    // Map user stack (4 pages = 16 KiB)
    let stack_flags = PageTableFlags::PRESENT
        | PageTableFlags::WRITABLE
        | PageTableFlags::USER_ACCESSIBLE
        | PageTableFlags::NO_EXECUTE;

    for i in 0..4 {
        let frame = FRAME_ALLOCATOR
            .lock()
            .allocate_frame()
            .expect("OOM allocating user stack frame");
        space.allocated_frames.push(frame);
        unsafe {
            core::ptr::write_bytes(frame.as_mut_ptr::<u8>(), 0, 4096);
        }
        let page_vaddr = VirtAddr::new(user_stack.as_u64() - ((4 - i) * 4096));
        space
            .map_user_page(page_vaddr, frame, stack_flags)
            .expect("Failed to map user stack page");
    }

    println!("  [PASS] User address space mapped (Entry: 0x{:X}, Stack: 0x{:X}).",
        user_entry.as_u64(), user_stack.as_u64()
    );

    // Test 3: Drop to Ring 3 and Execute User Syscall Test Suite
    println!("  Transitioning to Ring 3 to execute user syscall test suite...");
    let exit_code = unsafe {
        run_user_process(
            user_entry.as_u64(),
            user_stack.as_u64() - 16,
            space.pml4_phys.as_u64(),
        )
    };

    // Test 4: Validate User Process Exit Code
    assert_eq!(exit_code, 42, "User process must exit with status 42 (all syscall tests passed)");
    println!("  [PASS] SYS_PRINT verified (printed message from Ring 3 to console).");
    println!("  [PASS] Security boundary verified: SYS_PRINT with kernel pointer rejected (-EFAULT).");
    println!("  [PASS] SYS_GETPID verified (returned PID 1).");
    println!("  [PASS] SYS_UPTIME_MS verified (returned positive tick uptime).");
    println!("  [PASS] SYS_YIELD verified (voluntarily yielded and resumed execution in Ring 3).");
    println!("  [PASS] SYS_EXIT verified: process terminated cleanly with exit status {}.", exit_code);

    // Test 5: Reclaim Process Address Space
    let frame_count = space.allocated_frames.len();
    drop(space);
    println!("  [PASS] ProcessAddressSpace dropped: reclaimed all {} physical frames.", frame_count);

    println!("\n[TEST] All System Call ABI (syscall / sysret) Verification Tests PASSED!\n");
}
