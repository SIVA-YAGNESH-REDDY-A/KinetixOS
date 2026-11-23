//! Kernel Memory Management Subsystem for Kinetix OS.
//!
//! Encompasses:
//! - Physical Frame Allocation (4 GiB bitmap)
//! - 4-Level Virtual Memory Paging (x86_64 CR3, PML4, PDPT, PD, PT)
//! - Intrusive Coalescing Kernel Heap Allocator (`GlobalAlloc` for `alloc` crate)

pub mod frame;
pub mod heap;
pub mod paging;

use crate::println;
use kinetix_boot_protocol::BootInfo;

#[allow(unused_imports)]
pub use frame::{BitmapFrameAllocator, PhysAddr, FRAME_ALLOCATOR, PAGE_SIZE};
pub use heap::{KINETIX_HEAP_ALLOCATOR, HEAP_SIZE, HEAP_START};
pub use paging::{read_cr3, translate_addr, VirtAddr};

extern "C" {
    static __kernel_start: u8;
    static __kernel_end: u8;
}

/// Initializes the complete kernel memory subsystem:
/// 1. Discovers and marks physical memory via UEFI MemoryMap.
/// 2. Protects real-mode, kernel, framebuffer, and heap physical pages.
/// 3. Initializes the kernel heap allocator.
pub fn init(boot_info: &'static BootInfo) {
    let kernel_start = unsafe { &__kernel_start as *const u8 as u64 };
    let kernel_end = unsafe { &__kernel_end as *const u8 as u64 };

    println!("[MEMORY] Initializing Physical Frame Allocator...");
    println!(
        "  Kernel Image Span: 0x{:X}..0x{:X} ({} bytes)",
        kernel_start,
        kernel_end,
        kernel_end.saturating_sub(kernel_start)
    );

    // 1. Initialize Physical Frame Allocator
    {
        let mut fa = FRAME_ALLOCATOR.lock();
        fa.init(
            &boot_info.memory_map,
            kernel_start,
            kernel_end,
            &boot_info.framebuffer,
            &boot_info.modules,
        );

        // Reserve kernel heap region in the physical frame allocator
        fa.reserve_range(HEAP_START as u64, HEAP_SIZE as u64);

        println!(
            "  Total Usable RAM : {} MiB ({} frames)",
            fa.total_usable_bytes() / (1024 * 1024),
            fa.total_usable_frames()
        );
        println!(
            "  Free Physical RAM: {} MiB ({} frames)",
            fa.free_bytes() / (1024 * 1024),
            fa.free_frames()
        );
        println!(
            "  Reserved RAM     : {} MiB ({} frames)",
            fa.allocated_bytes() / (1024 * 1024),
            fa.allocated_frames()
        );
    }

    // 2. Report active Paging (CR3)
    let cr3 = read_cr3();
    println!("[MEMORY] Active 4-Level Page Table Root (CR3): 0x{:X}", cr3.as_u64());

    // 3. Initialize Kernel Heap Allocator
    println!("[MEMORY] Initializing Kernel Heap Allocator...");
    println!(
        "  Heap Range: 0x{:X}..0x{:X} ({} KiB)",
        HEAP_START,
        HEAP_START + HEAP_SIZE,
        HEAP_SIZE / 1024
    );
    unsafe {
        KINETIX_HEAP_ALLOCATOR.init(HEAP_START, HEAP_SIZE);
    }
    println!(
        "  Heap Available: {} KiB ({} bytes)",
        KINETIX_HEAP_ALLOCATOR.free_bytes() / 1024,
        KINETIX_HEAP_ALLOCATOR.free_bytes()
    );

    println!("[MEMORY] Memory subsystem initialized successfully.");
}

/// Runs a comprehensive suite of verification tests on the memory subsystem.
pub fn run_memory_tests() {
    println!("\n[TEST] Running Memory Subsystem Verification Tests...");

    // Test 1: Frame Allocator single-frame allocation & deallocation
    {
        let mut fa = FRAME_ALLOCATOR.lock();
        let free_before = fa.free_frames();
        let frame1 = fa.allocate_frame().expect("Failed to allocate physical frame 1");
        let frame2 = fa.allocate_frame().expect("Failed to allocate physical frame 2");

        assert!(frame1.as_u64() % PAGE_SIZE == 0, "Frame 1 must be 4 KiB aligned");
        assert!(frame2.as_u64() % PAGE_SIZE == 0, "Frame 2 must be 4 KiB aligned");
        assert!(frame1.as_u64() != frame2.as_u64(), "Frames must be distinct");
        assert_eq!(fa.free_frames(), free_before - 2);

        fa.deallocate_frame(frame1);
        fa.deallocate_frame(frame2);
        assert_eq!(fa.free_frames(), free_before, "Frames must be properly reclaimed");
        println!("  [PASS] Physical Frame Allocator (alloc/dealloc/alignment)");
    }

    // Test 2: Virtual Memory Translation
    {
        let kernel_start_vaddr = VirtAddr::new(0x0020_0000);
        let phys_translation = translate_addr(kernel_start_vaddr);
        assert!(phys_translation.is_some(), "Kernel virtual address must be mapped");
        assert_eq!(
            phys_translation.unwrap().as_u64(),
            0x0020_0000,
            "Kernel must be identity-mapped at 0x200000"
        );
        println!(
            "  [PASS] 4-Level Paging Walk (Virt 0x200000 -> Phys 0x{:X})",
            phys_translation.unwrap().as_u64()
        );
    }

    // Test 3: Kernel Heap Box Allocation (requires alloc crate)
    {
        extern crate alloc;
        use alloc::boxed::Box;

        let before_allocated = KINETIX_HEAP_ALLOCATOR.allocated_bytes();
        let test_val: u64 = 0x5449_4E45_4B49_5821; // "KINETIX!" inverted pattern
        let boxed = Box::new(test_val);

        assert_eq!(*boxed, test_val, "Boxed value must match original");
        assert!(KINETIX_HEAP_ALLOCATOR.allocated_bytes() > before_allocated);

        drop(boxed);
        assert_eq!(
            KINETIX_HEAP_ALLOCATOR.allocated_bytes(),
            before_allocated,
            "Heap memory must be reclaimed after Box drop"
        );
        println!("  [PASS] Kernel Heap Box allocation and reclamation");
    }

    // Test 4: Dynamic Vector Expansion & Coalescing
    {
        extern crate alloc;
        use alloc::vec::Vec;

        let before_allocated = KINETIX_HEAP_ALLOCATOR.allocated_bytes();
        {
            let mut vec: Vec<u32> = Vec::new();
            for i in 0..500 {
                vec.push(i * 3);
            }

            assert_eq!(vec.len(), 500);
            assert_eq!(vec[10], 30);
            assert_eq!(vec[499], 499 * 3);
        }
        assert_eq!(
            KINETIX_HEAP_ALLOCATOR.allocated_bytes(),
            before_allocated,
            "Heap memory must be reclaimed after Vec drop"
        );
        println!("  [PASS] Dynamic Vec<u32> capacity growth, elements, and drop");
    }

    // Test 5: String formatting and heap concatenation
    {
        extern crate alloc;
        use alloc::format;

        let before_allocated = KINETIX_HEAP_ALLOCATOR.allocated_bytes();
        {
            let s = format!("Kinetix OS Kernel Heap Test: version = {}, page_size = {}", 1, 4096);
            assert!(s.starts_with("Kinetix OS"));
        }
        assert_eq!(
            KINETIX_HEAP_ALLOCATOR.allocated_bytes(),
            before_allocated,
            "Heap memory must be reclaimed after String drop"
        );
        println!("  [PASS] Dynamic String formatting and heap allocation");
    }

    println!("[TEST] All Memory Subsystem Verification Tests PASSED!\n");
}
