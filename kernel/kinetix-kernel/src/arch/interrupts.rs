//! Hardware Interrupt & CPU Exception Dispatcher for x86_64.
//!
//! Provides low-level assembly stubs (`global_asm!`), `TrapFrame` register capture,
//! exception handlers, and hardware IRQ handling.

use super::cpu::{self, inb};
use super::pic::PICS;
use super::pit;
use crate::println;
use core::arch::global_asm;

/// Register state captured upon interrupt or CPU exception entry.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TrapFrame {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,

    pub vector: u64,
    pub error_code: u64,

    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

// Low-level assembly entry stubs
global_asm!(r#"
.macro ISR_NO_ERRCODE num
.global isr\num
isr\num:
    push 0
    push \num
    jmp common_isr_stub
.endm

.macro ISR_ERRCODE num
.global isr\num
isr\num:
    push \num
    jmp common_isr_stub
.endm

ISR_NO_ERRCODE 0
ISR_NO_ERRCODE 1
ISR_NO_ERRCODE 2
ISR_NO_ERRCODE 3
ISR_NO_ERRCODE 4
ISR_NO_ERRCODE 5
ISR_NO_ERRCODE 6
ISR_NO_ERRCODE 7
ISR_ERRCODE    8
ISR_NO_ERRCODE 9
ISR_ERRCODE    10
ISR_ERRCODE    11
ISR_ERRCODE    12
ISR_ERRCODE    13
ISR_ERRCODE    14
ISR_NO_ERRCODE 15
ISR_NO_ERRCODE 16
ISR_ERRCODE    17
ISR_NO_ERRCODE 18
ISR_NO_ERRCODE 19
ISR_NO_ERRCODE 20
ISR_ERRCODE    21
ISR_NO_ERRCODE 22
ISR_NO_ERRCODE 23
ISR_NO_ERRCODE 24
ISR_NO_ERRCODE 25
ISR_NO_ERRCODE 26
ISR_NO_ERRCODE 27
ISR_NO_ERRCODE 28
ISR_ERRCODE    29
ISR_ERRCODE    30
ISR_NO_ERRCODE 31

.global irq0
irq0:
    push 0
    push 32
    jmp common_isr_stub

.global irq1
irq1:
    push 0
    push 33
    jmp common_isr_stub

.global irq12
irq12:
    push 0
    push 44
    jmp common_isr_stub

.global isr_default
isr_default:
    push 0
    push 255
    jmp common_isr_stub

common_isr_stub:
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15

    mov rdi, rsp
    cld
    call interrupt_dispatch

    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax

    add rsp, 16
    iretq
"#);

static EXCEPTION_NAMES: [&str; 32] = [
    "Divide-by-Zero (#DE)",
    "Debug (#DB)",
    "Non-Maskable Interrupt (NMI)",
    "Breakpoint (#BP)",
    "Overflow (#OF)",
    "Bound Range Exceeded (#BR)",
    "Invalid Opcode (#UD)",
    "Device Not Available (#NM)",
    "Double Fault (#DF)",
    "Coprocessor Segment Overrun",
    "Invalid TSS (#TS)",
    "Segment Not Present (#NP)",
    "Stack-Segment Fault (#SS)",
    "General Protection Fault (#GP)",
    "Page Fault (#PF)",
    "Reserved",
    "x87 Floating-Point Exception (#MF)",
    "Alignment Check (#AC)",
    "Machine Check (#MC)",
    "SIMD Floating-Point Exception (#XM)",
    "Virtualization Exception (#VE)",
    "Control Protection Exception (#CP)",
    "Reserved", "Reserved", "Reserved", "Reserved",
    "Reserved", "Reserved",
    "VMM Communication Exception (#VC)",
    "Security Exception (#SX)",
    "Reserved", "Reserved",
];

/// Central dispatch point called from `common_isr_stub`.
#[no_mangle]
pub extern "sysv64" fn interrupt_dispatch(frame: &mut TrapFrame) {
    if frame.vector < 32 {
        handle_exception(frame);
    } else if frame.vector >= 32 && frame.vector < 48 {
        handle_irq(frame);
    } else {
        println!("[INTERRUPT] Unhandled spurious vector {}", frame.vector);
    }
}

fn handle_exception(frame: &mut TrapFrame) {
    let vec = frame.vector as usize;
    let name = if vec < EXCEPTION_NAMES.len() {
        EXCEPTION_NAMES[vec]
    } else {
        "Unknown Exception"
    };

    // Vector 3 (#BP): Software breakpoint — check for user mode breakpoint first
    if frame.vector == 3 {
        if (frame.cs & 3) == 3 {
            if crate::user::handle_user_breakpoint(frame) {
                return;
            }
        }
        println!("  [EXCEPTION #BP] Breakpoint caught at RIP: 0x{:X}", frame.rip);
        return;
    }

    // Vector 13 (#GP): General Protection Fault — check for user mode privilege violation
    if frame.vector == 13 {
        if (frame.cs & 3) == 3 {
            if crate::user::handle_user_gp_fault(frame) {
                return;
            }
        }
    }

    // Vector 14 (#PF): Page Fault — inspect CR2
    if frame.vector == 14 {
        let cr2 = cpu::read_cr2();
        println!("\n!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
        println!("              PAGE FAULT (#PF) OCCURRED         ");
        println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
        println!("Faulting Linear Address (CR2): 0x{:X}", cr2);
        println!("Instruction Pointer (RIP)    : 0x{:X}", frame.rip);
        println!("Error Code                   : 0x{:X}", frame.error_code);
        println!("Flags: Present={}, Write={}, User={}, Exec={}",
            (frame.error_code & 1) != 0,
            (frame.error_code & 2) != 0,
            (frame.error_code & 4) != 0,
            (frame.error_code & 16) != 0
        );
        dump_registers(frame);
        cpu::halt_loop();
    }

    // All other exceptions: fatal error
    println!("\n!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    println!("             CPU EXCEPTION: {}                  ", name);
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    println!("Vector: {} | Error Code: 0x{:X}", frame.vector, frame.error_code);
    dump_registers(frame);
    cpu::halt_loop();
}

fn handle_irq(frame: &TrapFrame) {
    let irq = (frame.vector - 32) as u8;

    match irq {
        0 => {
            // IRQ 0: PIT Timer
            pit::on_tick();
            unsafe {
                PICS.lock().notify_end_of_interrupt(frame.vector as u8);
            }
            crate::task::handle_timer_tick();
        }
        1 => {
            // IRQ 1: PS/2 Keyboard
            let scancode = unsafe { inb(0x60) };
            super::keyboard::on_interrupt(scancode);
            unsafe {
                PICS.lock().notify_end_of_interrupt(frame.vector as u8);
            }
        }
        12 => {
            // IRQ 12: Kinetic Pointer (PS/2 Mouse / Wireless Dongle)
            crate::pulses::pointer::on_interrupt();
            unsafe {
                PICS.lock().notify_end_of_interrupt(frame.vector as u8);
            }
        }
        _ => {
            unsafe {
                PICS.lock().notify_end_of_interrupt(frame.vector as u8);
            }
        }
    }
}

fn dump_registers(frame: &TrapFrame) {
    println!("RIP: 0x{:016X}  RSP: 0x{:016X}  RFLAGS: 0x{:08X}", frame.rip, frame.rsp, frame.rflags);
    println!("RAX: 0x{:016X}  RBX: 0x{:016X}  RCX:    0x{:016X}", frame.rax, frame.rbx, frame.rcx);
    println!("RDX: 0x{:016X}  RSI: 0x{:016X}  RDI:    0x{:016X}", frame.rdx, frame.rsi, frame.rdi);
    println!("RBP: 0x{:016X}  R8:  0x{:016X}  R9:     0x{:016X}", frame.rbp, frame.r8, frame.r9);
    println!("R10: 0x{:016X}  R11: 0x{:016X}  R12:    0x{:016X}", frame.r10, frame.r11, frame.r12);
    println!("R13: 0x{:016X}  R14: 0x{:016X}  R15:    0x{:016X}", frame.r13, frame.r14, frame.r15);
}
