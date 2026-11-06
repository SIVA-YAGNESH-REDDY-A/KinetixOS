#![no_std]
#![no_main]

extern crate alloc;

mod arch;
mod console;
mod memory;
mod research;
mod task;

use core::panic::PanicInfo;
use kinetix_boot_protocol::BootInfo;

/// Initial dedicated 64 KiB kernel stack.
#[allow(dead_code)]
#[repr(align(16))]
struct KernelStack([u8; 64 * 1024]);
#[allow(dead_code)]
static KERNEL_STACK: KernelStack = KernelStack([0; 64 * 1024]);

/// The definitive 64-bit Kinetix Kernel Entry Point.
///
/// Handoff contract:
/// - Called from Kinetix Bootloader after `ExitBootServices()`.
/// - Argument passed in register `RDI`: pointer to immutable `BootInfo`.
#[no_mangle]
#[link_section = ".text._start"]
pub extern "sysv64" fn _start(boot_info: &'static BootInfo) -> ! {
    // 1. Initialize Serial Diagnostics (COM1: 0x3F8)
    arch::serial::SERIAL1.lock().init();

    // 2. Initialize Graphical Framebuffer Console early if present
    if boot_info.is_valid() && boot_info.framebuffer.physical_address != 0 {
        console::FRAMEBUFFER_CONSOLE.lock().init(boot_info.framebuffer);
    }

    println!("================================================");
    println!("                 KINETIX OS                     ");
    println!("================================================");
    println!();
    println!("Kinetix Kernel Initialized");
    println!();
    println!("Architecture : x86_64");
    println!("Language     : Rust (core + alloc + no_std)");
    println!("Boot Mode    : UEFI (64-bit)");
    println!();
    println!("Kernel Status: RUNNING");
    println!();
    println!("================================================");

    // 3. Validate Boot Information Protocol
    if boot_info.is_valid() {
        println!("[KERNEL] BootInfo verification PASSED (Magic: 0x{:X})", boot_info.magic);
        println!(
            "[KERNEL] Memory Map: {} regions reported by firmware.",
            boot_info.memory_map.region_count
        );

        let mut usable_ram_bytes: u64 = 0;
        for i in 0..boot_info.memory_map.region_count {
            let region = &boot_info.memory_map.regions[i];
            if region.is_usable() {
                usable_ram_bytes += region.length;
            }
        }
        println!(
            "[KERNEL] Usable Physical Memory: {} MiB ({} bytes)",
            usable_ram_bytes / (1024 * 1024),
            usable_ram_bytes
        );

        // Framebuffer diagnostics
        if boot_info.framebuffer.physical_address != 0 {
            println!(
                "[KERNEL] Framebuffer detected at 0x{:X} ({}x{}, stride: {})",
                boot_info.framebuffer.physical_address,
                boot_info.framebuffer.width,
                boot_info.framebuffer.height,
                boot_info.framebuffer.stride
            );
        } else {
            println!("[KERNEL] No framebuffer provided; serial console only.");
        }

        // 4. Report staged application modules
        println!("[KERNEL] Preloaded Application Modules: {}", boot_info.modules.count);
        for i in 0..boot_info.modules.count {
            let module = &boot_info.modules.modules[i];
            let name = core::str::from_utf8(&module.name).unwrap_or("unknown");
            println!(
                "  Module {}: '{}' at 0x{:X} ({} bytes)",
                i, name, module.physical_address, module.size
            );
        }

        // 5. Initialize Memory Management Subsystem (Phase 7)
        memory::init(boot_info);

        // 6. Execute Memory Verification Self-Tests
        memory::run_memory_tests();

        // 7. Initialize CPU Architecture & Interrupt Infrastructure (Phase 8)
        arch::init();

        // 8. Execute Exception & Hardware Interrupt Verification Self-Tests
        arch::run_interrupt_tests();

        // 9. Execute Hardware Interaction Verification Self-Tests (Phase 9)
        console::run_console_tests();
        arch::keyboard::run_keyboard_tests();

        // 10. Initialize Multitasking & Preemptive Multi-Policy Scheduler (Phases 10 & 11)
        task::init();
        task::run_scheduler_tests();

        // 11. Execute CPU Multitasking Academic Research Benchmarks (Phase 12)
        research::run_research_experiments();
    } else {
        println!("[KERNEL WARNING] BootInfo signature invalid or version mismatch!");
    }

    println!("[KERNEL] Phase 12 Milestone achieved: CPU Multitasking Academic Research complete.");

    // Enable interrupts and enter low-power halt loop
    arch::cpu::enable_interrupts();
    arch::cpu::halt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("\n!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    println!("             KERNEL PANIC OCCURRED              ");
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    println!("{}", info);
    println!("System halted.");
    arch::cpu::halt_loop();
}
