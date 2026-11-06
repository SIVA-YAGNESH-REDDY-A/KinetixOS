#![no_std]
#![no_main]

use kinetix_runtime::{exit, getpid, print_str, print_u64, uptime_ms};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    print_str("  [APP: hello-world] Hello from Kinetix OS Native Application!\n");
    print_str("  [APP: hello-world] Process ID : ");
    print_u64(getpid());
    print_str("\n");
    print_str("  [APP: hello-world] Boot Uptime: ");
    print_u64(uptime_ms());
    print_str(" ms\n");
    print_str("  [APP: hello-world] Exiting cleanly with status 0.\n");
    exit(0);
}
