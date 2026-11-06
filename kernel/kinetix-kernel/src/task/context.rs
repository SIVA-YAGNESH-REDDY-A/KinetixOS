//! Low-Level CPU Context Switch for x86_64 Tasks.
//!
//! Preserves and restores callee-saved registers (RBX, RBP, R12, R13, R14, R15)
//! and swaps the hardware stack pointer (RSP).

use core::arch::global_asm;

extern "C" {
    /// Switches execution context from the currently executing task to the next task.
    ///
    /// - `old_rsp`: Points to where the current task's updated stack pointer should be stored.
    /// - `new_rsp`: Points to the target task's stack pointer to restore.
    pub fn switch_context(old_rsp: *mut u64, new_rsp: *const u64);

    /// Entry trampoline jumped into when a newly spawned task is scheduled for the first time.
    pub fn task_trampoline();
}

global_asm!(r#"
.global switch_context
switch_context:
    // 1. Save callee-saved registers of current task
    push rbx
    push rbp
    push r12
    push r13
    push r14
    push r15

    // 2. Save current RSP to *old_rsp (RDI)
    mov [rdi], rsp

    // 3. Load target RSP from *new_rsp (RSI)
    mov rsp, [rsi]

    // 4. Restore callee-saved registers of target task
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbp
    pop rbx

    // 5. Return into the target task's saved RIP
    ret

.global task_trampoline
task_trampoline:
    // First argument for task_entry_wrapper is entry_fn (stored in R12 during stack init)
    mov rdi, r12
    call task_entry_wrapper

    // If task function returns, gracefully exit the task
    call task_exit_trampoline

1:  hlt
    jmp 1b
"#);

#[no_mangle]
pub extern "sysv64" fn task_entry_wrapper(entry_fn: usize) {
    let func: fn() = unsafe { core::mem::transmute(entry_fn) };
    func();
}

#[no_mangle]
pub extern "sysv64" fn task_exit_trampoline() -> ! {
    super::exit();
}
