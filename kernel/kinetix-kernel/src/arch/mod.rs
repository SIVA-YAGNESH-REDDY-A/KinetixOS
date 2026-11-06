//! Architecture-Specific Implementation for x86_64.

pub mod cpu;
pub mod gdt;
pub mod idt;
pub mod interrupts;
pub mod pic;
pub mod pit;
pub mod serial;

use crate::println;

/// Initializes the complete CPU architecture and interrupt subsystem:
/// 1. GDT & TSS with dedicated Double Fault IST stack
/// 2. IDT with 256 gates
/// 3. Dual 8259 PIC remapped to vectors 32..47
/// 4. 8254 PIT at 100 Hz (10 ms ticks)
pub fn init() {
    println!("[ARCH] Initializing Global Descriptor Table (GDT) & TSS...");
    gdt::init();

    println!("[ARCH] Initializing Interrupt Descriptor Table (IDT)...");
    idt::init();

    println!("[ARCH] Initializing Dual 8259 PIC (IRQs remapped to 32..47)...");
    unsafe {
        pic::PICS.lock().init();
    }

    println!("[ARCH] Initializing 8254 PIT Timer at 100 Hz (10 ms ticks)...");
    unsafe {
        pit::init(100);
    }

    println!("[ARCH] CPU & Interrupt Infrastructure initialized successfully.");
}

/// Runs verification tests on CPU exceptions and hardware timer interrupts.
pub fn run_interrupt_tests() {
    println!("\n[TEST] Running CPU Exception & Interrupt Verification Tests...");

    // Test 1: Software Breakpoint Exception (#BP, Vector 3) via INT3 instruction
    {
        println!("  Testing Software Breakpoint Exception (int3)...");
        unsafe {
            core::arch::asm!("int3", options(nomem, nostack, preserves_flags));
        }
        println!("  [PASS] Software Breakpoint (#BP) handled cleanly, resumed execution.");
    }

    // Test 2: Hardware Timer Interrupts (IRQ 0 / Vector 32)
    {
        println!("  Enabling CPU Interrupts (sti) to test 8254 PIT timer ticks...");
        let ticks_before = pit::get_ticks();
        cpu::enable_interrupts();

        // Wait for at least 10 timer ticks (100 ms)
        while pit::get_ticks() < ticks_before + 10 {
            cpu::halt(); // Low-power wait for interrupt
        }

        let ticks_after = pit::get_ticks();
        println!(
            "  [PASS] PIT Timer Interrupts working (received {} ticks during test).",
            ticks_after - ticks_before
        );
    }

    println!("[TEST] All CPU Exception & Interrupt Verification Tests PASSED!\n");
}
