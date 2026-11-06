#![no_std]

use core::arch::asm;
use core::panic::PanicInfo;

pub const SYS_PRINT: usize = 1;
pub const SYS_YIELD: usize = 2;
pub const SYS_EXIT: usize = 3;
pub const SYS_GETPID: usize = 4;
pub const SYS_UPTIME_MS: usize = 5;

/// Invokes a Kinetix kernel system call with 0 arguments.
#[inline(always)]
pub unsafe fn syscall0(number: usize) -> usize {
    let ret: usize;
    asm!(
        "syscall",
        inout("rax") number => ret,
        out("rcx") _,
        out("r11") _,
        options(nostack)
    );
    ret
}

/// Invokes a Kinetix kernel system call with 1 argument.
#[inline(always)]
pub unsafe fn syscall1(number: usize, arg1: usize) -> usize {
    let ret: usize;
    asm!(
        "syscall",
        inout("rax") number => ret,
        in("rdi") arg1,
        out("rcx") _,
        out("r11") _,
        options(nostack)
    );
    ret
}

/// Invokes a Kinetix kernel system call with 2 arguments.
#[inline(always)]
pub unsafe fn syscall2(number: usize, arg1: usize, arg2: usize) -> usize {
    let ret: usize;
    asm!(
        "syscall",
        inout("rax") number => ret,
        in("rdi") arg1,
        in("rsi") arg2,
        out("rcx") _,
        out("r11") _,
        options(nostack)
    );
    ret
}

/// Terminates the current application process.
pub fn exit(code: i32) -> ! {
    unsafe {
        syscall1(SYS_EXIT, code as usize);
    }
    loop {}
}

/// Prints a UTF-8 string to the Kinetix OS console.
pub fn print_str(s: &str) {
    unsafe {
        syscall2(SYS_PRINT, s.as_ptr() as usize, s.len());
    }
}

/// Voluntarily yields the CPU time-slice to the scheduler.
pub fn yield_now() {
    unsafe {
        syscall0(SYS_YIELD);
    }
}

/// Returns the current process ID.
pub fn getpid() -> u64 {
    unsafe { syscall0(SYS_GETPID) as u64 }
}

/// Returns the system uptime in milliseconds since boot.
pub fn uptime_ms() -> u64 {
    unsafe { syscall0(SYS_UPTIME_MS) as u64 }
}

/// Default panic handler for native Kinetix user applications.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    print_str("[APP PANIC] Native application aborted.\n");
    exit(-1);
}
