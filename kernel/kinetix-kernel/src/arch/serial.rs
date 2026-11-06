//! 16550 UART Serial Driver for Kinetix Kernel

use core::arch::asm;
use core::fmt::{self, Write};
use spin::Mutex;

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

pub struct SerialPort {
    port: u16,
}

impl SerialPort {
    pub const fn new(port: u16) -> Self {
        Self { port }
    }

    pub fn init(&mut self) {
        unsafe {
            outb(self.port + 1, 0x00); // Disable interrupts
            outb(self.port + 3, 0x80); // Enable DLAB
            outb(self.port + 0, 0x01); // Divisor 1 (115200 baud)
            outb(self.port + 1, 0x00);
            outb(self.port + 3, 0x03); // 8 bits, no parity, 1 stop bit (8N1)
            outb(self.port + 2, 0xC7); // Enable & clear FIFO
            outb(self.port + 4, 0x0B); // IRQs enabled, RTS/DSR set
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        unsafe {
            while (inb(self.port + 5) & 0x20) == 0 {}
            outb(self.port, byte);
        }
    }

    pub fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
    }
}

impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str(s);
        Ok(())
    }
}

pub static SERIAL1: Mutex<SerialPort> = Mutex::new(SerialPort::new(COM1_PORT));

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::arch::serial::_print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

pub fn _print(args: fmt::Arguments) {
    let mut serial = SERIAL1.lock();
    let _ = serial.write_fmt(args);
}
