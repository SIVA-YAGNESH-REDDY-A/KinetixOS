#![no_std]
#![no_main]

extern crate alloc;

mod arch;
mod canvas;
mod cartridges;
mod console;
mod fs;
mod loader;
mod memory;
mod pulses;
mod research;
mod task;
mod user;
mod syscall;

use core::panic::PanicInfo;
use kinetix_boot_protocol::BootInfo;

/// Initial dedicated 64 KiB kernel stack located in identity-mapped BSS (PDPT[0]).
#[repr(align(16))]
struct KernelStack([u8; 64 * 1024]);
static KERNEL_STACK: KernelStack = KernelStack([0; 64 * 1024]);

/// The definitive 64-bit Kinetix Kernel Entry Point.
///
/// Handoff contract:
/// - Called from Kinetix Bootloader after `ExitBootServices()`.
/// - Argument passed in register `RDI`: pointer to immutable `BootInfo`.
#[no_mangle]
#[link_section = ".text._start"]
pub extern "sysv64" fn _start(boot_info: &'static BootInfo) -> ! {
    let stack_top = {
        let ptr = (&raw const KERNEL_STACK.0) as *const u8 as u64;
        ptr + (64 * 1024)
    };

    let entry_fn = kernel_main as *const () as usize;
    unsafe {
        core::arch::asm!(
            "mov rsp, {top}",
            "call {entry}",
            top = in(reg) stack_top,
            entry = in(reg) entry_fn,
            in("rdi") boot_info,
            options(noreturn)
        );
    }
}

extern "sysv64" fn kernel_main(boot_info: &'static BootInfo) -> ! {
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

        // 7. Initialize In-Memory Boot Filesystem (Phase 13)
        fs::init(boot_info);
        fs::run_fs_tests();

        // 8. Execute Native Application Loader Verification Self-Tests (Phase 14)
        loader::run_loader_tests();

        // 9. Initialize CPU Architecture, Interrupt Infrastructure, and System Calls (Phases 8 & 16)
        arch::init();
        syscall::init();

        // 10. Execute Exception & Hardware Interrupt Verification Self-Tests
        arch::run_interrupt_tests();

        // 11. Execute Hardware Interaction Verification Self-Tests (Phase 9)
        console::run_console_tests();
        arch::keyboard::run_keyboard_tests();

        // 12. Initialize Multitasking & Preemptive Multi-Policy Scheduler (Phases 10 & 11)
        task::init();
        task::run_scheduler_tests();

        // 13. Execute CPU Multitasking Academic Research Benchmarks (Phase 12)
        research::run_research_experiments();

        // 14. Execute User Mode (Ring 3 Privilege Separation) Verification Tests (Phase 15)
        user::run_user_mode_tests();

        // 15. Execute System Call ABI (syscall / sysret) Verification Tests (Phase 16)
        syscall::run_syscall_tests();

        // 16. Execute Native Demonstration Applications in Ring 3 (Phase 17)
        loader::run_demonstration_apps();

        // 17. Initialize Kinetic Pointer & Execute Reticle Verification Self-Tests (Phase 18)
        pulses::init(boot_info.framebuffer.width as usize, boot_info.framebuffer.height as usize);
        pulses::pointer::run_pointer_tests();

        // 18. Initialize Canvas Desktop Environment & Execute Compositor Verification Tests (Phase 19)
        canvas::init(boot_info.framebuffer);
        canvas::run_canvas_tests();

        // 19. Launch Cartridge I (Scribe Notes Studio) & Run Verification Tests (Phase 20)
        cartridges::scribe::run_scribe_tests();

        // 20. Launch Cartridge II (Ouroboros Arcade Arena) & Run Verification Tests (Phase 21)
        cartridges::ouroboros::run_ouroboros_tests();

        // 21. Execute Full System Integration & Laptop Regression Tests (Phase 22)
        pulses::router::run_integration_tests();
        println!("[KERNEL] Phase 22 Milestone achieved: Full System Integration & Laptop Verification operational.");

        // 22. Final Desktop Presentation: Composite Desktop & Draw Kinetic Reticle Cursor
        canvas::compose_desktop();
        pulses::reticle::RETICLE.lock().init(boot_info.framebuffer);
        pulses::reticle::RETICLE.lock().draw_at(512, 384);
    } else {
        println!("[KERNEL WARNING] BootInfo signature invalid or version mismatch!");
    }

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
