#![no_std]
#![no_main]

mod serial;

use core::panic::PanicInfo;
use uefi::prelude::*;

#[entry]
fn main() -> Status {
    // 1. Initialize Serial Port COM1 (0x3F8)
    serial::init();
    serial_println!("================================================");
    serial_println!("                    KINETIX OS                  ");
    serial_println!("================================================");
    serial_println!("           Independent Rust Operating System    ");
    serial_println!("Architecture       : x86_64");
    serial_println!("Boot Firmware      : UEFI");
    serial_println!("Kernel             : Kinetix Kernel");
    serial_println!("Kernel Language    : Rust");
    serial_println!("Bootloader         : Kinetix Bootloader");
    serial_println!("Status             : INITIALIZING (Phase 3 Verified)");
    serial_println!("================================================");

    // 2. Clear screen and print banner to UEFI text console if stdout is available
    let _ = uefi::system::with_stdout(|stdout| {
        let _ = stdout.clear();
        let _ = stdout.output_string(
            uefi::cstr16!("================================================\r\n                    KINETIX OS\r\n================================================\r\n\r\n           Independent Rust Operating System\r\n\r\nArchitecture       : x86_64\r\nBoot Firmware      : UEFI\r\nKernel             : Kinetix Kernel\r\nKernel Language    : Rust\r\nBootloader         : Kinetix Bootloader\r\n\r\nStatus             : INITIALIZING (Phase 3 Milestone)\r\n\r\n================================================\r\n")
        );
    });

    serial_println!("[KINETIX BOOTLOADER] UEFI banner displayed successfully.");
    Status::SUCCESS
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("[BOOTLOADER PANIC] {:?}", info);
    loop {}
}
