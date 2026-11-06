#![no_std]
#![no_main]

use kinetix_runtime::{exit, print_str};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    print_str("Hello from Kinetix OS Native Application!\n");
    exit(0);
}
