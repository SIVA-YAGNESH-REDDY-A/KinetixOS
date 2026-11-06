#![no_std]
#![no_main]

use core::panic::PanicInfo;
use kinetix_boot_protocol::BootInfo;

#[no_mangle]
pub extern "C" fn _start(_boot_info: &'static BootInfo) -> ! {
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
