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

/// Reads an 8-bit byte from the specified I/O port.
#[inline(always)]
pub unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    asm!("in al, dx", in("dx") port, out("al") value, options(nomem, nostack, preserves_flags));
    value
}

/// Writes an 8-bit byte to the specified I/O port.
#[inline(always)]
pub unsafe fn outb(port: u16, value: u8) {
    asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
}

/// Short I/O delay (writes to unused diagnostic port 0x80).
#[inline(always)]
pub unsafe fn io_wait() {
    outb(0x80, 0);
}

/// Reads the CR2 register containing the linear address that caused a page fault.
#[inline(always)]
pub fn read_cr2() -> u64 {
    let value: u64;
    unsafe {
        asm!("mov {}, cr2", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Reads a 64-bit Model-Specific Register (MSR).
#[inline(always)]
pub unsafe fn rdmsr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    asm!(
        "rdmsr",
        in("ecx") msr,
        out("eax") low,
        out("edx") high,
        options(nomem, nostack, preserves_flags)
    );
    ((high as u64) << 32) | (low as u64)
}

/// Writes a 64-bit value to a Model-Specific Register (MSR).
#[inline(always)]
pub unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    asm!(
        "wrmsr",
        in("ecx") msr,
        in("eax") low,
        in("edx") high,
        options(nomem, nostack, preserves_flags)
    );
}

/// Halts the CPU in an infinite loop (for panic or idle task).
pub fn halt_loop() -> ! {
    loop {
        halt();
    }
}
