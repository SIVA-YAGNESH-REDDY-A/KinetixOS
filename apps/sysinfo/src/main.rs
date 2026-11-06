#![no_std]
#![no_main]

use kinetix_runtime::{exit, getpid, print_str, print_u64, uptime_ms};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    print_str("  [APP: sysinfo] ========================================\n");
    print_str("  [APP: sysinfo]        KINETIX OS SYSTEM INFORMATION     \n");
    print_str("  [APP: sysinfo] ========================================\n");
    print_str("  [APP: sysinfo] OS Name     : Kinetix OS (Pure Rust no_std)\n");
    print_str("  [APP: sysinfo] Target Arch : x86_64 (AMD64 Long Mode)\n");
    print_str("  [APP: sysinfo] Firmware    : UEFI (64-bit Native)\n");
    print_str("  [APP: sysinfo] Privilege   : Ring 3 (Unprivileged User Mode)\n");
    print_str("  [APP: sysinfo] ABI Linkage : Fast System Call (SYSCALL / SYSRET)\n");
    print_str("  [APP: sysinfo] Process ID  : ");
    print_u64(getpid());
    print_str("\n");
    print_str("  [APP: sysinfo] System Time : ");
    print_u64(uptime_ms());
    print_str(" ms elapsed\n");
    print_str("  [APP: sysinfo] ========================================\n");
    exit(0);
}
