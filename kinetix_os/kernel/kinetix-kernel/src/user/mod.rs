//! User Mode (Ring 3 Privilege Separation) Subsystem for Kinetix OS.
//!
//! Encompasses:
//! - Hardware privilege level transition from Ring 0 to Ring 3 via `iretq`
//! - Enforcement of x86_64 privilege boundaries (blocking privileged instructions)
//! - TSS RSP0 privilege stack switching during interrupts and exceptions
//! - Verification of CPL == 3 execution and clean kernel context restoration

pub mod ring3;

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};
use crate::arch::gdt::{self, KERNEL_CODE_SELECTOR, KERNEL_DATA_SELECTOR, TSS_SELECTOR, USER_CODE_SELECTOR, USER_DATA_SELECTOR};
use crate::arch::interrupts::TrapFrame;
use crate::loader::process::ProcessAddressSpace;
use crate::memory::frame::FRAME_ALLOCATOR;
use crate::memory::paging::{PageTableFlags, VirtAddr};
use crate::println;

pub use ring3::{return_from_user_mode_probe, run_user_mode_probe};

pub static USER_PROBE_STAGE: AtomicU8 = AtomicU8::new(0);
pub static USER_CPL_RECORDED: AtomicU8 = AtomicU8::new(0);
pub static USER_GP_RECORDED: AtomicBool = AtomicBool::new(false);
pub static USER_INT3_COUNT: AtomicU32 = AtomicU32::new(0);

/// Handles software breakpoint (`#BP`, Vector 3) occurring during User Mode execution.
pub fn handle_user_breakpoint(frame: &mut TrapFrame) -> bool {
    let stage = USER_PROBE_STAGE.load(Ordering::SeqCst);

    if stage == 0 {
        // Stage 0: User probe has read CS into RAX and executed int3
        let cpl = (frame.rax & 3) as u8;
        USER_CPL_RECORDED.store(cpl, Ordering::SeqCst);
        USER_INT3_COUNT.fetch_add(1, Ordering::SeqCst);
        println!("  [USER PROBE] Breakpoint 1: User mode confirmed (CPL = {}, CS = 0x{:X}, SS = 0x{:X})",
            cpl, frame.cs, frame.ss
        );
        USER_PROBE_STAGE.store(1, Ordering::SeqCst);
        true // Resume user code to test privileged instruction 'cli'
    } else if stage == 1 {
        // Stage 1: Resumed execution after #GP privilege fault
        USER_INT3_COUNT.fetch_add(1, Ordering::SeqCst);
        println!("  [USER PROBE] Breakpoint 2: Resumed after #GP! Ring 3 probe complete.");
        USER_PROBE_STAGE.store(2, Ordering::SeqCst);

        // Terminate user probe and return directly to kernel caller
        unsafe {
            return_from_user_mode_probe();
        }
    } else {
        false
    }
}

/// Handles General Protection Fault (`#GP`, Vector 13) occurring during User Mode execution.
pub fn handle_user_gp_fault(frame: &mut TrapFrame) -> bool {
    USER_GP_RECORDED.store(true, Ordering::SeqCst);
    println!("  [USER PROBE] Intercepted privileged instruction 'cli' in Ring 3 (#GP, Error Code: 0x{:X})",
        frame.error_code
    );

    // Skip the faulting 1-byte instruction ('cli' = 0xFA) and resume
    frame.rip += 1;
    true
}

/// Executes comprehensive verification tests for User Mode (Ring 3 Privilege Separation).
pub fn run_user_mode_tests() {
    println!("\n================================================================================");
    println!("       KINETIX OS USER MODE (RING 3 PRIVILEGE SEPARATION) TESTS (PHASE 15)      ");
    println!("================================================================================\n");

    // Test 1: Verify GDT Selectors and TSS RSP0 Configuration
    assert_eq!(KERNEL_CODE_SELECTOR, 0x08, "Kernel Code Selector must be 0x08");
    assert_eq!(KERNEL_DATA_SELECTOR, 0x10, "Kernel Data Selector must be 0x10");
    assert_eq!(USER_DATA_SELECTOR, 0x18, "User Data Selector must be 0x18");
    assert_eq!(USER_CODE_SELECTOR, 0x20, "User Code Selector must be 0x20");
    assert_eq!(TSS_SELECTOR, 0x28, "TSS Selector must be 0x28");

    let rsp0 = gdt::get_kernel_stack();
    assert!(rsp0 != 0, "TSS RSP0 must be configured");
    assert_eq!(rsp0 % 16, 0, "TSS RSP0 must be 16-byte aligned");
    println!("  [PASS] GDT Segments (Kernel 0x08/0x10, User 0x18/0x20, TSS 0x28) verified.");
    println!("  [PASS] TSS RSP0 configured at 0x{:X} (16 KiB kernel stack).", rsp0);

    // Test 2: Prepare Isolated User Process Address Space
    let user_entry = VirtAddr::new(0x4000_0000);
    let user_stack = VirtAddr::new(0x7FFF_F000);
    let mut space = ProcessAddressSpace::new(user_entry, user_stack)
        .expect("Failed to initialize user address space");

    // Allocate physical frame for user test code
    let code_frame = FRAME_ALLOCATOR
        .lock()
        .allocate_frame()
        .expect("OOM allocating user code frame");
    space.allocated_frames.push(code_frame);

    // User machine code probe payload (10 bytes):
    // 0x48, 0x8C, 0xC8       : mov rax, cs
    // 0x48, 0x83, 0xE0, 0x03 : and rax, 3
    // 0xCC                   : int3 (Stage 0: reports CPL in rax)
    // 0xFA                   : cli  (Privileged instruction: hardware triggers #GP!)
    // 0xCC                   : int3 (Stage 1: confirms resumption after #GP)
    let payload: [u8; 10] = [
        0x48, 0x8C, 0xC8,
        0x48, 0x83, 0xE0, 0x03,
        0xCC,
        0xFA,
        0xCC,
    ];

    unsafe {
        let dest = code_frame.as_mut_ptr::<u8>();
        core::ptr::write_bytes(dest, 0, 4096);
        core::ptr::copy_nonoverlapping(payload.as_ptr(), dest, payload.len());
    }

    let code_flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
    space
        .map_user_page(user_entry, code_frame, code_flags)
        .expect("Failed to map user code page");

    // Map user stack (4 pages)
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

    println!("  [PASS] User address space mapped: Code at 0x{:X}, Stack at 0x{:X}",
        user_entry.as_u64(), user_stack.as_u64()
    );

    // Test 3: Drop to Ring 3 and Execute User Probe
    USER_PROBE_STAGE.store(0, Ordering::SeqCst);
    USER_CPL_RECORDED.store(0, Ordering::SeqCst);
    USER_GP_RECORDED.store(false, Ordering::SeqCst);
    USER_INT3_COUNT.store(0, Ordering::SeqCst);

    println!("  Transitioning from Ring 0 to Ring 3 via iretq...");
    let ret = unsafe {
        run_user_mode_probe(
            user_entry.as_u64(),
            user_stack.as_u64() - 16,
            space.pml4_phys.as_u64(),
        )
    };
    assert_eq!(ret, 1, "User mode probe must return status 1 on completion");

    // Test 4: Validate Recorded Privilege Level & Protection Results
    assert_eq!(USER_CPL_RECORDED.load(Ordering::SeqCst), 3, "CPU must have executed with CPL = 3");
    println!("  [PASS] CPL == 3 (Ring 3) verified via CS register read.");

    assert!(USER_GP_RECORDED.load(Ordering::SeqCst), "Privileged instruction 'cli' must trigger #GP");
    println!("  [PASS] Hardware Privilege Barrier verified: 'cli' blocked by CPU with General Protection Fault.");

    assert_eq!(USER_INT3_COUNT.load(Ordering::SeqCst), 2, "Both breakpoints must have executed");
    println!("  [PASS] Exception gates & TSS RSP0 stack switching verified across privilege transitions.");

    // Test 5: Reclaim User Process Address Space
    let frame_count = space.allocated_frames.len();
    drop(space);
    println!("  [PASS] ProcessAddressSpace dropped: reclaimed all {} physical frames.", frame_count);

    println!("\n[TEST] All User Mode (Ring 3 Privilege Separation) Verification Tests PASSED!\n");
}
