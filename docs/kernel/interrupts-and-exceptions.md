# Kinetix OS — CPU Interrupts & Exception Infrastructure

**Document ID:** SPEC-KERNEL-ARCH-002  
**Status:** Implemented & Verified in VirtualBox  
**Target Architecture:** x86_64 Long Mode  
**Implementation Language:** Pure Rust (`core`, `no_std`) + Low-Level Assembly Stubs (`global_asm!`)  

---

## 1. Overview and Architecture

Phase 8 establishes the hardware interaction, fault isolation, and preemptive execution foundation for Kinetix OS:

```
+-------------------------------------------------------------------------+
|                  Rust Interrupt & Exception Dispatcher                  |
|     - Software Breakpoint (#BP) Handling & Execution Resumption         |
|     - Fatal Exception Register Dumps (#PF, #GP, #UD, #DE, etc.)         |
|     - Hardware IRQ 0 (PIT Timer Tick) & IRQ 1 (PS/2 Keyboard Scancode)  |
+-------------------------------------------------------------------------+
                                    ^
                                    | (rdi = &mut TrapFrame)
+-------------------------------------------------------------------------+
|               Assembly Stubs & TrapFrame Preservation                   |
|     - Preserves all 15 general-purpose registers (RAX..R15)             |
|     - Pushes Vector Number and Error Code (dummy 0 if none)             |
|     - Sets DF=0 (cld), calls Rust dispatcher, restores regs, iretq      |
+-------------------------------------------------------------------------+
                                    ^
                                    | (Hardware Interrupt / Exception)
+-------------------------------------------------------------------------+
|                 Interrupt Descriptor Table (IDT)                        |
|     - 256 64-bit Interrupt Gates loaded via lidt                        |
|     - Vectors 0..31: CPU Exceptions (Vector 8 #DF mapped to IST1)       |
|     - Vectors 32..47: Remapped Hardware IRQs 0..15                      |
+-------------------------------------------------------------------------+
         ^                                              ^
         |                                              |
+---------------------------------+     +---------------------------------+
|   Global Descriptor Table (GDT) |     |       Dual 8259 PIC & 8254 PIT  |
| - Kernel Code (0x08, Ring 0)    |     | - Remapped IRQs 0..15 to 32..47 |
| - Kernel Data (0x10, Ring 0)    |     | - PIT Channel 0 at 100 Hz       |
| - TSS with Double Fault IST1    |     | - 10 ms Periodic Scheduler Tick |
+---------------------------------+     +---------------------------------+
```

---

## 2. Global Descriptor Table (GDT) & Task State Segment (TSS)

### 2.1 Segment Selectors
- **Selector `0x08` (Kernel Code):** 64-bit executable, readable, Ring 0 (`0x0020_9A00_0000_0000`).
- **Selector `0x10` (Kernel Data):** 64-bit writable, Ring 0 (`0x0000_9200_0000_0000`).
- **Selector `0x18` (Task State Segment):** 16-byte system descriptor pointing to the kernel TSS.

### 2.2 Dedicated Double Fault IST Stack
- Without a dedicated Interrupt Stack Table (IST), a kernel stack overflow triggers a Double Fault (`#DF`, vector 8). Because the stack is already overflowed, pushing the exception frame triggers an immediate Triple Fault and hardware reboot.
- Kinetix OS configures `ist[0]` in the TSS pointing to a dedicated 16 KiB `DOUBLE_FAULT_STACK`.
- When vector 8 fires, the CPU hardware automatically switches to this dedicated stack before calling the handler, ensuring fault diagnosis.

---

## 3. Interrupt Descriptor Table (IDT) & Assembly Stubs

### 3.1 `TrapFrame` Structure
All registers are captured on the stack and passed by pointer (`&TrapFrame`) in register `RDI` to the dispatcher:
```rust
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TrapFrame {
    pub r15: u64, pub r14: u64, pub r13: u64, pub r12: u64,
    pub r11: u64, pub r10: u64, pub r9:  u64, pub r8:  u64,
    pub rbp: u64, pub rdi: u64, pub rsi: u64, pub rdx: u64,
    pub rcx: u64, pub rbx: u64, pub rax: u64,

    pub vector: u64,
    pub error_code: u64,

    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}
```

### 3.2 Error Code Normalization
- Exceptions that push an error code: Vectors 8 (`#DF`), 10 (`#TS`), 11 (`#NP`), 12 (`#SS`), 13 (`#GP`), 14 (`#PF`), 17 (`#AC`), 21 (`#CP`).
- Exceptions that do not push an error code push a dummy `0` so that `TrapFrame` remains uniform across all 256 gates.

---

## 4. Dual 8259 PIC & 8254 PIT Timer

- **PIC Vector Remapping:**
  - Master PIC: Hardware IRQs 0..7 mapped to vectors **32..39** (`0x20..0x27`).
  - Slave PIC: Hardware IRQs 8..15 mapped to vectors **40..47** (`0x28..0x2F`).
- **8254 PIT Configuration:**
  - Frequency: **100 Hz** (10 ms per tick).
  - Mode: Mode 2 Rate Generator (Square Wave).
  - Divisor: $1,193,182 / 100 \approx 11,932$ (`0x2E9C`).
  - Atomic Tick Counter: `TICKS.fetch_add(1, Ordering::Relaxed)`.

---

## 5. Live VirtualBox Verification Evidence

Booting in Oracle VirtualBox 7.2 produced the following verified serial log:

```text
[ARCH] Initializing Global Descriptor Table (GDT) & TSS...
[ARCH] Initializing Interrupt Descriptor Table (IDT)...
[ARCH] Initializing Dual 8259 PIC (IRQs remapped to 32..47)...
[ARCH] Initializing 8254 PIT Timer at 100 Hz (10 ms ticks)...
[ARCH] CPU & Interrupt Infrastructure initialized successfully.

[TEST] Running CPU Exception & Interrupt Verification Tests...
  Testing Software Breakpoint Exception (int3)...
  [EXCEPTION #BP] Breakpoint caught at RIP: 0x20304A
  [PASS] Software Breakpoint (#BP) handled cleanly, resumed execution.
  Enabling CPU Interrupts (sti) to test 8254 PIT timer ticks...
  [PASS] PIT Timer Interrupts working (received 10 ticks during test).
[TEST] All CPU Exception & Interrupt Verification Tests PASSED!

[KERNEL] Phase 8 Milestone achieved: CPU Interrupts & Exception Infrastructure operational.
```
