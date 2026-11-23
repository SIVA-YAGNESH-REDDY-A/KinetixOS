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

/// Writes a 16-bit word to the specified I/O port.
#[inline(always)]
pub unsafe fn outw(port: u16, value: u16) {
    asm!("out dx, ax", in("dx") port, in("ax") value, options(nomem, nostack, preserves_flags));
}

/// Reads a 16-bit word from the specified I/O port.
#[inline(always)]
pub unsafe fn inw(port: u16) -> u16 {
    let value: u16;
    asm!("in ax, dx", in("dx") port, out("ax") value, options(nomem, nostack, preserves_flags));
    value
}

/// Reads multiple 16-bit words from an I/O port into a buffer (ATA PIO sector read).
#[inline(always)]
pub unsafe fn insw(port: u16, buf: *mut u16, count: usize) {
    asm!(
        "rep insw",
        in("dx") port,
        inout("rdi") buf => _,
        inout("rcx") count => _,
        options(nostack, preserves_flags)
    );
}

/// Writes multiple 16-bit words from a buffer to an I/O port (ATA PIO sector write).
#[inline(always)]
pub unsafe fn outsw(port: u16, buf: *const u16, count: usize) {
    asm!(
        "rep outsw",
        in("dx") port,
        inout("rsi") buf => _,
        inout("rcx") count => _,
        options(nostack, preserves_flags)
    );
}

/// Reads a 32-bit double-word from the specified I/O port (used for PCIe config space).
#[inline(always)]
pub unsafe fn inl(port: u16) -> u32 {
    let value: u32;
    asm!("in eax, dx", in("dx") port, out("eax") value, options(nomem, nostack, preserves_flags));
    value
}

/// Writes a 32-bit double-word to the specified I/O port (used for PCIe config space).
#[inline(always)]
pub unsafe fn outl(port: u16, value: u32) {
    asm!("out dx, eax", in("dx") port, in("eax") value, options(nomem, nostack, preserves_flags));
}

/// Reads the CPU Time-Stamp Counter (RDTSC) for high-precision boot-time profiling.
#[inline(always)]
pub fn rdtsc() -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        asm!("rdtsc", out("eax") low, out("edx") high, options(nomem, nostack, preserves_flags));
    }
    ((high as u64) << 32) | (low as u64)
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
