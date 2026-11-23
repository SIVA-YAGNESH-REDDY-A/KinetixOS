//! 8254 Programmable Interval Timer (PIT) Driver.
//!
//! Provides periodic timer interrupts on IRQ 0 (Interrupt Vector 32)
//! to drive kernel timekeeping and preemptive multitasking scheduling ticks.

use super::cpu::{io_wait, outb};
use core::sync::atomic::{AtomicU64, Ordering};

const PIT_CHANNEL0_DATA: u16 = 0x40;
const PIT_COMMAND_MODE: u16 = 0x43;

const PIT_BASE_FREQUENCY_HZ: u32 = 1_193_182;

/// Monotonically increasing timer tick counter.
pub static TICKS: AtomicU64 = AtomicU64::new(0);

/// Initializes PIT Channel 0 at the desired frequency (Hz).
///
/// Typical setting: 100 Hz (10 ms per tick).
pub unsafe fn init(target_freq_hz: u32) {
    let divisor = (PIT_BASE_FREQUENCY_HZ + target_freq_hz / 2) / target_freq_hz;
    let divisor = divisor.clamp(1, 65535) as u16;

    // Command: Channel 0, Access mode low/high byte, Operating mode 2 (Rate Generator), Binary mode
    // 00 (Channel 0) | 11 (Access lo/hi) | 010 (Mode 2) | 0 (Binary) = 0x34
    outb(PIT_COMMAND_MODE, 0x34);
    io_wait();

    // Send divisor low byte then high byte
    outb(PIT_CHANNEL0_DATA, (divisor & 0xFF) as u8);
    io_wait();
    outb(PIT_CHANNEL0_DATA, ((divisor >> 8) & 0xFF) as u8);
    io_wait();
}

/// Called by the IRQ 0 interrupt handler on every timer tick.
#[inline]
pub fn on_tick() -> u64 {
    TICKS.fetch_add(1, Ordering::Relaxed) + 1
}

/// Reads the current system tick count.
#[inline]
pub fn get_ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}
