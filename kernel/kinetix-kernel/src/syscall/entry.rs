//! Low-Level System Call Assembly Stubs & MSR Initialization for x86_64.
//!
//! Implements hardware fast system call dispatch via `syscall` / `sysretq`,
//! kernel stack switching, MSR configuration, and user process entry/exit.

use core::arch::global_asm;
use crate::arch::cpu;

pub const MSR_EFER: u32 = 0xC000_0080;
pub const MSR_STAR: u32 = 0xC000_0081;
pub const MSR_LSTAR: u32 = 0xC000_0082;
pub const MSR_SFMASK: u32 = 0xC000_0084;

pub const EFER_SCE: u64 = 1 << 0; // System Call Extensions Enable

#[repr(align(16))]
pub struct SyscallStack([u8; 16 * 1024]);
pub static mut KERNEL_SYSCALL_STACK: SyscallStack = SyscallStack([0; 16 * 1024]);

extern "C" {
    pub static mut KERNEL_SYSCALL_RSP: u64;
    pub fn syscall_entry();

    /// Transitions to Ring 3 and runs the user process until exit.
    pub fn run_user_process(entry_point: u64, user_rsp: u64, pml4_phys: u64) -> i64;

    /// Terminates user process execution, restoring kernel CR3, stack, and segments.
    pub fn exit_user_process_asm(exit_code: i64) -> !;

    pub fn user_syscall_probe_start();
    pub fn user_syscall_probe_end();
}

global_asm!(r#"
.data
.align 8
.global USER_SAVED_RSP
.global KERNEL_SYSCALL_RSP
.global KERNEL_SAVED_PROCESS_RSP
.global KERNEL_SAVED_PROCESS_CR3

USER_SAVED_RSP:            .quad 0
KERNEL_SYSCALL_RSP:        .quad 0
KERNEL_SAVED_PROCESS_RSP:  .quad 0
KERNEL_SAVED_PROCESS_CR3:  .quad 0

.text
.global syscall_entry
syscall_entry:
    // 1. Save user RSP to scratch location
    mov [rip + USER_SAVED_RSP], rsp

    // 2. Switch to dedicated kernel syscall stack
    mov rsp, [rip + KERNEL_SYSCALL_RSP]

    // 3. Preserve user context on kernel syscall stack
    push [rip + USER_SAVED_RSP] // User RSP
    push r11                    // User RFLAGS (saved by hardware in R11)
    push rcx                    // User RIP (saved by hardware in RCX)
    push rbp
    push rbx
    push r12
    push r13
    push r14
    push r15

    // 4. Reload kernel data segments (0x10) using rcx (already pushed to stack)
    mov cx, 0x10
    mov ds, cx
    mov es, cx

    // 5. Map incoming syscall arguments to System V AMD64 ABI:
    // Syscall registers: num=RAX, a1=RDI, a2=RSI, a3=RDX, a4=R10, a5=R8
    // Dispatcher signature: (num: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64)
    // SysV ABI args: rdi, rsi, rdx, rcx, r8, r9
    mov r9, r8         // Arg 5 (from R8)
    mov r8, r10        // Arg 4 (from R10)
    mov rcx, rdx       // Arg 3 (from RDX)
    mov rdx, rsi       // Arg 2 (from RSI)
    mov rsi, rdi       // Arg 1 (from RDI)
    mov rdi, rax       // Syscall number (from RAX)

    cld
    call syscall_dispatch

    // Return value from syscall_dispatch is in RAX

    // 6. Reload user data segments (0x18 | 3 = 0x1B) using cx before restoring rcx
    mov cx, 0x1B
    mov ds, cx
    mov es, cx

    // 7. Restore registers
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbp
    pop rcx            // User RIP restored into RCX for sysretq
    pop r11            // User RFLAGS restored into R11 for sysretq
    pop rsp            // User RSP restored directly

    // 8. Return to Ring 3 via 64-bit sysretq!
    sysretq

.global run_user_process
run_user_process:
    // Arguments: rdi = entry_point, rsi = user_rsp, rdx = pml4_phys
    push rbx
    push rbp
    push r12
    push r13
    push r14
    push r15

    // Save kernel RSP and CR3
    mov rax, rsp
    mov [rip + KERNEL_SAVED_PROCESS_RSP], rax
    mov rax, cr3
    mov [rip + KERNEL_SAVED_PROCESS_CR3], rax

    // Construct iretq frame before changing CR3
    push 0x1B          // SS: User Data RPL 3
    push rsi           // RSP: User Stack Pointer
    push 0x202         // RFLAGS: IF=1, Reserved=1
    push 0x23          // CS: User Code RPL 3
    push rdi           // RIP: User Entry Point

    // Reload data segments with User Data Selector (0x1B)
    mov ax, 0x1B
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    // Switch to User Process Page Table (CR3 = RDX)
    mov cr3, rdx

    // Drop to Ring 3!
    iretq

.global exit_user_process_asm
exit_user_process_asm:
    // Argument: rdi = exit_code (i64)
    // 1. Restore kernel CR3
    mov rax, [rip + KERNEL_SAVED_PROCESS_CR3]
    mov cr3, rax

    // 2. Restore kernel Data Segments (0x10)
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax

    // 3. Restore kernel RSP
    mov rsp, [rip + KERNEL_SAVED_PROCESS_RSP]

    // 4. Restore callee-saved registers
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbp
    pop rbx

    // 5. Return exit_code in RAX to caller of run_user_process
    mov rax, rdi
    ret

.global user_syscall_probe_start
.global user_syscall_probe_end
user_syscall_probe_start:
    // Step 1: SYS_PRINT greeting (36 bytes)
    mov rax, 1
    lea rdi, [rip + user_msg_greet]
    mov rsi, 36
    syscall
    cmp rax, 36
    jne .Lfail

    // Step 2: SYS_PRINT with invalid kernel pointer (0x200000) -> must return -14 (-EFAULT)
    mov rax, 1
    mov rdi, 0x200000
    mov rsi, 16
    syscall
    cmp rax, -14
    jne .Lfail

    // Step 3: SYS_GETPID -> must return 1
    mov rax, 4
    syscall
    cmp rax, 1
    jne .Lfail

    // Step 4: SYS_UPTIME_MS -> must return >= 0
    mov rax, 5
    syscall
    cmp rax, 0
    jl .Lfail

    // Step 5: SYS_YIELD -> must return 0
    mov rax, 2
    syscall
    cmp rax, 0
    jne .Lfail

    // Step 6: SYS_EXIT(42) -> Clean exit with code 42
    mov rax, 3
    mov rdi, 42
    syscall

.Lfail:
    // Exit with failure code 99
    mov rax, 3
    mov rdi, 99
    syscall
    hlt
    jmp .Lfail

user_msg_greet:
    .ascii "  [USER] Syscall SYS_PRINT invoked!\n\0"

.align 8
user_syscall_probe_end:
"#);

/// Initializes the hardware System Call MSR infrastructure.
pub fn init() {
    // 1. Initialize dedicated Kernel Syscall Stack Pointer
    let stack_top = unsafe {
        let ptr = (&raw const KERNEL_SYSCALL_STACK.0) as *const u8 as u64;
        ptr + (16 * 1024)
    };
    unsafe {
        let ptr = &raw mut KERNEL_SYSCALL_RSP;
        *ptr = stack_top;
    }

    unsafe {
        // 2. Enable SCE (System Call Extensions) in IA32_EFER
        let efer = cpu::rdmsr(MSR_EFER);
        cpu::wrmsr(MSR_EFER, efer | EFER_SCE);

        // 3. Configure Segment Selectors in IA32_STAR:
        // Bits [47:32] = Kernel CS (0x08). (Kernel SS is automatically 0x08 + 8 = 0x10)
        // Bits [63:48] = User Segment Base (0x10).
        //                User SS = 0x10 + 8  = 0x18 | 3 = 0x1B
        //                User CS = 0x10 + 16 = 0x20 | 3 = 0x23
        let star: u64 = (0x0010u64 << 48) | (0x0008u64 << 32);
        cpu::wrmsr(MSR_STAR, star);

        // 4. Set Linear Syscall Target Address in IA32_LSTAR
        cpu::wrmsr(MSR_LSTAR, syscall_entry as *const () as usize as u64);

        // 5. Mask Flags in IA32_SFMASK:
        // Mask IF (bit 9, 0x200), DF (bit 10, 0x400), TF (bit 8, 0x100)
        let sfmask: u64 = 0x200 | 0x400 | 0x100;
        cpu::wrmsr(MSR_SFMASK, sfmask);
    }
}
