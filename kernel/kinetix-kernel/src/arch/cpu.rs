#![allow(dead_code)]

//! Low-Level CPU Control for Kinetix Kernel

use core::arch::asm;

/// Halts the CPU until the next external interrupt occurs.
#[inline(always)]
pub fn halt() {
    unsafe {
        asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}

/// Disables maskable hardware interrupts.
#[inline(always)]
pub fn disable_interrupts() {
    unsafe {
        asm!("cli", options(nomem, nostack, preserves_flags));
    }
}

/// Enables maskable hardware interrupts.
#[inline(always)]
pub fn enable_interrupts() {
    unsafe {
        asm!("sti", options(nomem, nostack, preserves_flags));
    }
}

/// Halts the CPU in an infinite loop (for panic or idle task).
pub fn halt_loop() -> ! {
    loop {
        halt();
    }
}
