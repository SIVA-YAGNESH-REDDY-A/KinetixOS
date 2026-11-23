//! ACPI & Hardware Power Controller for Kinetix OS.
//!
//! Provides bare-metal computer shutdown and reset primitives:
//! - QEMU / Bochs / VirtualBox ACPI power-off (I/O ports 0x604, 0x4004, 0xB004)
//! - Universal 8042 Keyboard Controller CPU Reset (Port 0x64, pulse 0xFE)
//! - APM Power Management Shutdown (I/O port 0x8900)

use crate::arch::cpu::{inb, outb, outw};

/// Shuts down the physical or virtual computer.
pub fn poweroff() -> ! {
    crate::println!("[POWER] Initiating system shutdown sequence...");

    unsafe {
        // 1. Enable ACPI Mode in VirtualBox / Hardware Chipset
        // VirtualBox SMI_CMD is at 0x442E (and standard PC 0xB2).
        // ACPI_ENABLE command byte is 0xA1.
        outb(0x442E, 0xA1);
        outb(0xB2, 0xA1);
        crate::arch::cpu::io_wait();

        // 2. VirtualBox ACPI PM1a_CNT Port (0x4004)
        // SLP_TYPa = 5 -> (5 << 10) | 0x2000 = 0x3400
        outw(0x4004, 0x3400);
        outw(0x4004, 0x2000);
        outw(0x4000, 0x3400);

        // 3. Modern Q35 / QEMU / Hyper-V ACPI power-off
        outw(0x604, 0x2000);
        outw(0xB004, 0x2000);

        // 4. APM Shutdown (Port 0x8900)
        outw(0x8900, 0x2000);
    }

    crate::println!("[POWER] ACPI signal sent. Halting CPU cores.");
    crate::arch::cpu::disable_interrupts();
    crate::arch::cpu::halt_loop();
}

/// Reboots the computer via the 8042 keyboard controller pulse.
pub fn reboot() -> ! {
    crate::println!("[POWER] Initiating hardware system reboot...");

    unsafe {
        // Wait until input buffer is empty
        let mut timeout = 100_000;
        while (inb(0x64) & 0x02) != 0 && timeout > 0 {
            timeout -= 1;
        }
        // Send Pulse Reset command (0xFE) to 8042 controller
        outb(0x64, 0xFE);
    }

    crate::arch::cpu::halt_loop();
}
