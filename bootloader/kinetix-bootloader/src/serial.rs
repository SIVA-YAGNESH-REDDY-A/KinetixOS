//! 16550 UART Serial Driver for Kinetix Bootloader
//!
//! Provides direct hardware communication via COM1 (`0x3F8`) at 115200 baud (8N1).

use core::arch::asm;
use core::fmt::{self, Write};

const COM1_PORT: u16 = 0x3F8;

#[inline]
unsafe fn outb(port: u16, val: u8) {
    asm!("out dx, al", in("dx") port, in("al") val, options(nostack, preserves_flags));
}

#[inline]
unsafe fn inb(port: u16) -> u8 {
    let ret: u8;
    asm!("in al, dx", in("dx") port, out("al") ret, options(nostack, preserves_flags));
    ret
}

/// Initializes COM1 (0x3F8) for 115200 baud, 8 data bits, no parity, 1 stop bit.
pub fn init() {
    unsafe {
        // Disable interrupts
        outb(COM1_PORT + 1, 0x00);
        // Enable DLAB (set baud rate divisor)
        outb(COM1_PORT + 3, 0x80);
        // Set divisor to 1 (115200 baud)
        outb(COM1_PORT + 0, 0x01);
        outb(COM1_PORT + 1, 0x00);
        // 8 bits, no parity, one stop bit (8N1)
        outb(COM1_PORT + 3, 0x03);
        // Enable FIFO, clear them, with 14-byte threshold
        outb(COM1_PORT + 2, 0xC7);
        // IRQs enabled, RTS/DSR set
        outb(COM1_PORT + 4, 0x0B);
    }
}

/// Sends a single byte over COM1, busy-waiting until the transmit FIFO is ready.
pub fn send_byte(byte: u8) {
    unsafe {
        // Wait for Line Status Register (0x3FD) bit 5 (Empty Transmitter Holding Register)
        while (inb(COM1_PORT + 5) & 0x20) == 0 {}
        outb(COM1_PORT, byte);
    }
}

/// Sends a string over COM1. Automatically expands `\n` to `\r\n`.
pub fn send_str(s: &str) {
    for byte in s.bytes() {
        if byte == b'\n' {
            send_byte(b'\r');
        }
        send_byte(byte);
    }
}

pub struct SerialWriter;

impl Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        send_str(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        let _ = <$crate::serial::SerialWriter as core::fmt::Write>::write_fmt(
            &mut $crate::serial::SerialWriter,
            format_args!($($arg)*)
        );
    };
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => ($crate::serial_print!("{}\n", format_args!($($arg)*)));
}
