//! Native Application Loader Subsystem for Kinetix OS.
//!
//! Encompasses:
//! - ELF64 executable validation and program header decoding
//! - Isolated user process address space creation
//! - Segment allocation, mapping, and .bss zeroing
//! - User stack initialization and memory reclamation

pub mod elf;
pub mod process;

use crate::memory::frame::FRAME_ALLOCATOR;
use crate::memory::paging::{PageTableFlags, VirtAddr};
use crate::println;

pub use elf::{ElfBinary, ElfError};
pub use process::{ProcessAddressSpace, USER_CODE_BASE, USER_STACK_SIZE_PAGES, USER_STACK_TOP};

/// Errors that can occur during application loading.
#[allow(dead_code)]
#[derive(Debug)]
pub enum LoadError {
    FileNotFound,
    Elf(ElfError),
    AddressSpaceInitFailed(&'static str),
    PageMappingFailed(&'static str),
    OutOfMemory,
}

/// Loads a native application directly from the in-memory boot filesystem.
pub fn load_app_from_fs(path: &str) -> Result<ProcessAddressSpace, LoadError> {
    let bytes = crate::fs::read(path).ok_or(LoadError::FileNotFound)?;
    load_app_from_bytes(path, bytes)
}

/// Parses an ELF64 binary and instantiates a complete user process address space.
pub fn load_app_from_bytes(name: &str, bytes: &[u8]) -> Result<ProcessAddressSpace, LoadError> {
    let elf = ElfBinary::parse(bytes).map_err(LoadError::Elf)?;

    let entry = VirtAddr::new(elf.entry_point());
    let stack_top = VirtAddr::new(USER_STACK_TOP);
    let mut space = ProcessAddressSpace::new(entry, stack_top)
        .map_err(LoadError::AddressSpaceInitFailed)?;

    // 1. Load each PT_LOAD program segment
    for ph in elf.load_segments() {
        let mut flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
        if ph.is_writable() {
            flags = PageTableFlags::from_bits_truncate(flags.bits() | PageTableFlags::WRITABLE.bits());
        }
        if !ph.is_executable() {
            flags = PageTableFlags::from_bits_truncate(flags.bits() | PageTableFlags::NO_EXECUTE.bits());
        }

        let seg_data = elf.segment_data(ph).unwrap_or(&[]);
        let pages_needed = ((ph.p_memsz as usize) + 4095) / 4096;

        for i in 0..pages_needed {
            let frame = FRAME_ALLOCATOR
                .lock()
                .allocate_frame()
                .ok_or(LoadError::OutOfMemory)?;
            space.allocated_frames.push(frame);

            // Zero entire page first (handles .bss automatically)
            unsafe {
                core::ptr::write_bytes(frame.as_mut_ptr::<u8>(), 0, 4096);
            }

            // Copy file data if present within this page
            let page_file_offset = i * 4096;
            if page_file_offset < seg_data.len() {
                let copy_len = core::cmp::min(4096, seg_data.len() - page_file_offset);
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        seg_data.as_ptr().add(page_file_offset),
                        frame.as_mut_ptr::<u8>(),
                        copy_len,
                    );
                }
            }

            let page_vaddr = VirtAddr::new(ph.p_vaddr + (i * 4096) as u64);
            space
                .map_user_page(page_vaddr, frame, flags)
                .map_err(LoadError::PageMappingFailed)?;
        }
    }

    // 2. Allocate and map User Process Stack (16 KiB below USER_STACK_TOP)
    let stack_flags = PageTableFlags::PRESENT
        | PageTableFlags::WRITABLE
        | PageTableFlags::USER_ACCESSIBLE
        | PageTableFlags::NO_EXECUTE;

    let stack_base = USER_STACK_TOP - (USER_STACK_SIZE_PAGES * 4096) as u64;
    for i in 0..USER_STACK_SIZE_PAGES {
        let frame = FRAME_ALLOCATOR
            .lock()
            .allocate_frame()
            .ok_or(LoadError::OutOfMemory)?;
        space.allocated_frames.push(frame);

        unsafe {
            core::ptr::write_bytes(frame.as_mut_ptr::<u8>(), 0, 4096);
        }

        let page_vaddr = VirtAddr::new(stack_base + (i * 4096) as u64);
        space
            .map_user_page(page_vaddr, frame, stack_flags)
            .map_err(LoadError::PageMappingFailed)?;
    }

    let _ = name;
    Ok(space)
}

/// Executes comprehensive verification self-tests for the Native Application Loader.
pub fn run_loader_tests() {
    println!("\n[TEST] Running Native Application Loader (User ELF64) Verification Tests...");

    // Test 1: Load /bin/hello-world from the In-Memory Filesystem
    let app_path = "/bin/hello-world";
    let space = load_app_from_fs(app_path)
        .expect("Failed to load /bin/hello-world into process address space");
    println!("  [PASS] Loaded '{}' into isolated ProcessAddressSpace.", app_path);

    // Test 2: Verify Entry Point Alignment & Address
    assert_eq!(
        space.entry_point.as_u64(),
        USER_CODE_BASE,
        "Entry point must equal USER_CODE_BASE (0x4000_0000)"
    );
    println!("  [PASS] Verified Entry Point: 0x{:X}", space.entry_point.as_u64());

    // Test 3: Verify Virtual Address Translation of Code Segment
    let code_phys = space
        .translate_user_addr(space.entry_point)
        .expect("Code segment must be mapped in user address space");
    println!("  [PASS] Code virtual address 0x{:X} translates to physical frame 0x{:X}",
        space.entry_point.as_u64(), code_phys.as_u64()
    );

    // Test 4: Verify Content of Data Segment (String Inspection)
    // In hello-world, .rodata is at 0x4000_1000
    let data_vaddr = VirtAddr::new(0x4000_1000);
    let data_phys = space
        .translate_user_addr(data_vaddr)
        .expect("Data segment must be mapped at 0x4000_1000");

    let string_slice = unsafe {
        let ptr = data_phys.as_ptr::<u8>();
        core::slice::from_raw_parts(ptr, 41)
    };
    let text = core::str::from_utf8(string_slice).unwrap_or("");
    assert!(text.contains("Hello from Kinetix OS"), "Data segment must contain expected string");
    println!("  [PASS] Verified data segment content: '{}'", text.trim());

    // Test 5: Verify User Stack Mapping & Canary Write/Read
    let stack_top_page = VirtAddr::new(USER_STACK_TOP - 4096);
    let stack_phys = space
        .translate_user_addr(stack_top_page)
        .expect("User stack top page must be mapped");

    let canary_val: u64 = 0xCAFE_BABE_DEAD_BEEF;
    unsafe {
        let canary_ptr = (stack_phys.as_u64() + 4096 - 8) as *mut u64;
        canary_ptr.write(canary_val);
        assert_eq!(canary_ptr.read(), canary_val, "Stack canary readback mismatch");
    }
    println!("  [PASS] User stack verified (16 KiB mapped at 0x{:X}, canary written & read).",
        space.user_stack_top.as_u64()
    );

    // Test 6: Verify Process Address Space Frame Tracking and Drop Cleanup
    let allocated_frames_count = space.allocated_frames.len();
    assert!(allocated_frames_count >= 6, "Must have allocated at least 6 frames (PML4, PDPT, PD, PT, Code, Stack)");
    println!("  [PASS] ProcessAddressSpace tracks {} allocated physical frames.", allocated_frames_count);

    let before_drop = FRAME_ALLOCATOR.lock().allocated_frames();
    drop(space);
    let after_drop = FRAME_ALLOCATOR.lock().allocated_frames();
    assert_eq!(
        before_drop - after_drop,
        allocated_frames_count,
        "All process frames must be reclaimed on drop"
    );
    println!("  [PASS] Process drop successfully reclaimed all {} physical frames.", allocated_frames_count);

    // Test 7: Verify Rejection of Invalid / Corrupted ELF Buffers
    let invalid_bytes = [0u8; 32];
    let err = load_app_from_bytes("invalid", &invalid_bytes);
    assert!(matches!(err, Err(LoadError::Elf(_))), "Invalid binary must return ElfError");
    println!("  [PASS] Corrupted binary rejected cleanly with ElfError.");

    println!("[TEST] All Native Application Loader Verification Tests PASSED!\n");
}
