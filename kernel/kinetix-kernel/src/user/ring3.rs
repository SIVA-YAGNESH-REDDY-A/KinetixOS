//! Low-Level Ring 3 Assembly Stubs and Context Switching for Kinetix OS.
//!
//! Provides `iretq` transition from Ring 0 to Ring 3, segment reloading,
//! and clean kernel state restoration upon probe completion.

use core::arch::global_asm;

extern "C" {
    /// Saves kernel execution context, switches to user CR3, and drops to Ring 3 via `iretq`.
    pub fn run_user_mode_probe(entry_point: u64, user_rsp: u64, pml4_phys: u64) -> u64;

    /// Restores kernel execution context from the saved frame and returns to the kernel test caller.
    pub fn return_from_user_mode_probe() -> !;
}

global_asm!(r#"
.global KERNEL_SAVED_RSP
.global KERNEL_SAVED_CR3

.data
.align 8
KERNEL_SAVED_RSP: .quad 0
KERNEL_SAVED_CR3: .quad 0

.text
.global run_user_mode_probe
run_user_mode_probe:
    // 1. Save kernel callee-saved registers
    push rbx
    push rbp
    push r12
    push r13
    push r14
    push r15

    // 2. Save kernel RSP and CR3
    mov rax, rsp
    mov [rip + KERNEL_SAVED_RSP], rax
    mov rax, cr3
    mov [rip + KERNEL_SAVED_CR3], rax

    // 3. Construct iretq frame before switching CR3
    push 0x1B          // SS: User Data RPL 3
    push rsi           // RSP: User Stack Pointer
    push 0x202         // RFLAGS: Interrupts Enabled (IF=1, Reserved=1)
    push 0x23          // CS: User Code RPL 3 (0x20 | 3 = 0x23)
    push rdi           // RIP: User Entry Point

    // 4. Reload data segments with User Data Selector (0x18 | 3 = 0x1B)
    mov ax, 0x1B
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    // 5. Switch to User Process Page Table (CR3 = RDX)
    mov cr3, rdx

    // 6. Transition to Ring 3!
    iretq

.global return_from_user_mode_probe
return_from_user_mode_probe:
    // 1. Restore kernel CR3
    mov rax, [rip + KERNEL_SAVED_CR3]
    mov cr3, rax

    // 2. Restore kernel Data Segments (0x10)
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax

    // 3. Restore kernel RSP
    mov rsp, [rip + KERNEL_SAVED_RSP]

    // 4. Restore callee-saved registers
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbp
    pop rbx

    // 5. Return status 1 (Success) to run_user_mode_tests caller
    mov rax, 1
    ret
"#);
