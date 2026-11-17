//! Hardware CMOS Real-Time Clock (RTC) Driver for Kinetix OS.
//!
//! Reads live date and time directly from CMOS registers (I/O ports 0x70 / 0x71)
//! so the Horizon chronometer and Telemetry deck display synced real-world time.

use crate::arch::cpu::{inb, outb};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RtcDateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl RtcDateTime {
    pub const fn zero() -> Self {
        Self {
            year: 2026,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
        }
    }
}

fn read_cmos(reg: u8) -> u8 {
    unsafe {
        outb(0x70, reg);
        inb(0x71)
    }
}

fn is_updating() -> bool {
    (read_cmos(0x0A) & 0x80) != 0
}

fn bcd_to_bin(val: u8) -> u8 {
    (val & 0x0F) + ((val / 16) * 10)
}

/// Reads the current date and time from the motherboard CMOS RTC.
pub fn read_datetime() -> RtcDateTime {
    // Wait until any in-progress RTC update completes
    let mut timeout = 10_000;
    while is_updating() && timeout > 0 {
        timeout -= 1;
    }

    let mut sec = read_cmos(0x00);
    let mut min = read_cmos(0x02);
    let mut hour = read_cmos(0x04);
    let mut day = read_cmos(0x07);
    let mut month = read_cmos(0x08);
    let mut year = read_cmos(0x09);
    let reg_b = read_cmos(0x0B);

    // Convert BCD to binary if bit 2 of Status Register B is clear
    if (reg_b & 0x04) == 0 {
        sec = bcd_to_bin(sec);
        min = bcd_to_bin(min);
        hour = bcd_to_bin(hour & 0x7F) | (hour & 0x80);
        day = bcd_to_bin(day);
        month = bcd_to_bin(month);
        year = bcd_to_bin(year);
    }

    // Handle 12-hour format -> 24-hour format if bit 1 of Status Register B is clear
    if (reg_b & 0x02) == 0 && (hour & 0x80) != 0 {
        hour = ((hour & 0x7F) + 12) % 24;
    } else {
        hour &= 0x7F;
    }

    RtcDateTime {
        year: 2000 + year as u16,
        month: month.clamp(1, 12),
        day: day.clamp(1, 31),
        hour: hour % 24,
        minute: min % 60,
        second: sec % 60,
    }
}
