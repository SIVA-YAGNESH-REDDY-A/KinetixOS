# User Mode (Ring 3 Privilege Separation) Architecture

## 1. Architectural Overview & Design Philosophy

Kinetix OS enforces strict hardware-level fault isolation between privileged kernel operations and unprivileged user-mode processes using the x86_64 hierarchical ring protection model. The central objective of Phase 15 is establishing an impermeable privilege boundary where application code operates strictly at **Current Privilege Level 3 (CPL = 3, Ring 3)**, preventing arbitrary hardware access, port I/O, control register manipulation, and execution of privileged processor instructions.

```
+-----------------------------------------------------------------------------+
|                                USER SPACE (Ring 3)                          |
|  - Applications linked at 0x4000_0000 (PDPT[1], 1 GiB .. 2 GiB)            |
|  - User Stack at 0x7FFF_EFF0 (16 KiB isolated mapping)                      |
|  - CPL = 3, CS = 0x23, SS = 0x1B, DS/ES/FS/GS = 0x1B                       |
|  - Privileged instructions ('cli', 'sti', 'in', 'out', 'mov crX') BLOCKED   |
+-----------------------------------------------------------------------------+
                                      |
                     Hardware Transition Barrier (`iretq` / Interrupt)
                                      |
+-----------------------------------------------------------------------------+
|                               KERNEL SPACE (Ring 0)                         |
|  - Kernel Core at 0x0020_0000 (2 MiB), Heap at 0x0040_0000 (PDPT[0])       |
|  - Dedicated 64 KiB Kernel Stack (`KERNEL_STACK`) in BSS                   |
|  - TSS RSP0 Stack (`KERNEL_RSP0_STACK`, 16 KiB) for Ring 3 -> Ring 0 entry  |
|  - CPL = 0, CS = 0x08, SS = 0x10, DS/ES/FS/GS = 0x10                       |
|  - Full access to hardware, MMIO, CR0..CR4, and all physical frames        |
+-----------------------------------------------------------------------------+
```

---

## 2. Global Descriptor Table (GDT) & Task State Segment (TSS)

### 2.1 Segment Selectors & Privilege Levels
The 64-bit GDT (`kernel/kinetix-kernel/src/arch/gdt.rs`) is structured to provide both kernel-level and user-level descriptors according to standard x86_64 conventions:

| Descriptor Index | Selector Value | RPL | Segment Type | Base / Limit | Attributes |
|---|---|---|---|---|---|
| 0 | `0x00` | 0 | Null Descriptor | 0 / 0 | Empty |
| 1 | `0x08` | 0 | Kernel 64-bit Code | 0 / 0 | Present, Ring 0, Exec/Read, Long Mode (L=1) |
| 2 | `0x10` | 0 | Kernel 64-bit Data | 0 / 0 | Present, Ring 0, Read/Write |
| 3 | `0x18` (`0x1B`) | 3 | User 64-bit Data | 0 / 0 | Present, Ring 3 (DPL=3), Read/Write |
| 4 | `0x20` (`0x23`) | 3 | User 64-bit Code | 0 / 0 | Present, Ring 3 (DPL=3), Exec/Read, Long Mode (L=1) |
| 5 & 6 | `0x28` | 0 | Task State Segment (TSS) | `&TSS` (16 bytes) | Present, Ring 0, Available 64-bit TSS (Type 9) |

- **User Data Selector**: Base index `0x18`. Loaded into segment registers with Requested Privilege Level 3 (`0x18 | 3 = 0x1B`).
- **User Code Selector**: Base index `0x20`. Loaded into `CS` via `iretq` with RPL 3 (`0x20 | 3 = 0x23`).

### 2.2 Task State Segment (TSS) RSP0 Privilege Stack Switching
When an interrupt, exception, or breakpoint fires while the CPU is executing in Ring 3 (CPL = 3), the hardware refuses to use the user stack pointer (`RSP`). Instead, the x86_64 processor automatically consults the **TSS Privilege Stack Table**:

$$\text{Target RSP} = \text{TSS.privilege\_stack\_table}[0] \quad (\text{RSP0})$$

Kinetix OS allocates a dedicated, 16-byte aligned 16 KiB kernel stack (`KERNEL_RSP0_STACK`) exclusively for privilege transitions. Upon interrupt arrival from Ring 3:
1. The CPU reads `RSP0` from the active TSS.
2. It switches internal stack context: `RSP = RSP0`.
3. It pushes the 5-quadword hardware frame: `SS (User)`, `RSP (User)`, `RFLAGS`, `CS (User)`, `RIP (User)`.
4. It dispatches to the corresponding ISR gate in Ring 0.

---

## 3. Hardware Privilege Level Transition (`iretq`)

Because the x86_64 architecture forbids software from directly modifying the `CS` register's RPL field via standard instructions (`mov`, `jmp`), dropping from Ring 0 to Ring 3 requires crafting a synthetic interrupt return stack frame and executing `iretq`.

### 3.1 Assembly Transition Sequence (`run_user_mode_probe`)
The transition routine (`kernel/kinetix-kernel/src/user/ring3.rs`) executes the following atomic sequence:

```asm
run_user_mode_probe:
    // 1. Preserve caller's callee-saved registers
    push rbx
    push rbp
    push r12
    push r13
    push r14
    push r15

    // 2. Preserve kernel stack pointer (RSP) and root page table (CR3)
    mov rax, rsp
    mov [rip + KERNEL_SAVED_RSP], rax
    mov rax, cr3
    mov [rip + KERNEL_SAVED_CR3], rax

    // 3. Construct 5-quadword iretq frame on identity-mapped kernel stack
    push 0x1B          // SS: User Data RPL 3
    push rsi           // RSP: User Stack Pointer (e.g. 0x7FFF_EFF0)
    push 0x202         // RFLAGS: IF=1 (Interrupts Enabled), Reserved=1
    push 0x23          // CS: User Code RPL 3 (0x20 | 3 = 0x23)
    push rdi           // RIP: User Entry Point (0x4000_0000)

    // 4. Reload data segment registers with User Data Selector (0x1B)
    mov ax, 0x1B
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    // 5. Switch to isolated user process page directory hierarchy
    mov cr3, rdx

    // 6. Transition privilege to Ring 3!
    iretq
```

### 3.2 Clean Return & Kernel Unwinding (`return_from_user_mode_probe`)
Upon completion of user-mode execution, the kernel exception dispatcher transfers control to `return_from_user_mode_probe`:
1. Restores the kernel page directory: `mov cr3, [rip + KERNEL_SAVED_CR3]`.
2. Restores kernel data segments: `mov ds/es/ss/fs/gs, 0x10`.
3. Restores the original kernel stack pointer: `mov rsp, [rip + KERNEL_SAVED_RSP]`.
4. Pops callee-saved registers (`r15`..`rbx`).
5. Returns status code 1 (`mov rax, 1; ret`) to the calling test coordinator.

---

## 4. Hardware Privilege Enforcement & IDT Gate Configuration

### 4.1 General Protection Fault (`#GP`, Vector 13)
Any attempt by code executing at CPL = 3 to perform privileged operations triggers an immediate `#GP` exception by the CPU hardware:
- Executing `cli` or `sti` when $\text{CPL} > \text{IOPL}$ (default IOPL is 0).
- Reading or writing control registers (`mov cr0..cr4, reg`).
- Accessing pages with $\text{U/S} = 0$ (Supervisor/Kernel pages).

Because `#GP` is classified as a **Fault** (not a Trap), the CPU saves the `RIP` pointing *directly* to the faulting instruction itself. The Kinetix OS handler advances `frame.rip += 1` to skip the 1-byte opcode (`0xFA` for `cli`), allowing execution to resume cleanly.

### 4.2 Software Breakpoint (`#BP`, Vector 3) & IDT DPL = 3
Unlike hardware faults, software interrupts (`INT n`, `INT3`) are checked against the IDT gate's **Descriptor Privilege Level (DPL)**:

$$\text{Access Granted} \iff \text{CPL} \le \text{Gate.DPL}$$

If an IDT gate has $\text{DPL} = 0$, an application in Ring 3 executing `int3` causes a `#GP` rather than entering the breakpoint handler. In Kinetix OS:
```rust
// arch/idt.rs
idt.entries[3].set_dpl(3); // Type attribute 0xEE (Present, DPL 3, 64-bit Interrupt Gate)
```
This enables native user-space debugging, breakpoints, and controlled trap transfers.

---

## 5. Dual Address Space Paging Isolation

User processes execute with an isolated Level 4 Page Map (`PML4`):
1. **Kernel Region (`PML4[0] -> PDPT[0]`)**: Identity-maps physical RAM (0..1 GiB), hosting kernel code (2 MiB), heap (4-8 MiB), GDT, IDT, TSS stacks, and `KERNEL_STACK`. Marked supervisor-only ($\text{U/S} = 0$).
2. **User Region (`PML4[0] -> PDPT[1]`)**: Mapped strictly for addresses `0x4000_0000 .. 0x7FFF_FFFF` (1 GiB .. 2 GiB). Contains application `.text`, `.rodata`, `.data`, and the user stack (`0x7FFF_EFF0`). Marked user-accessible ($\text{U/S} = 1$).
3. **Hardware Framebuffer Region (`PML4[0] -> PDPT[3]`)**: Identity-maps the GOP linear framebuffer at `0xE000_0000` (3.5 GiB) as supervisor-only, ensuring `println!` outputs diagnostic telemetry cleanly during interrupt handling without secondary faults.

---

## 6. Verification Results (Oracle VirtualBox 7.2)

```
================================================================================
       KINETIX OS USER MODE (RING 3 PRIVILEGE SEPARATION) TESTS (PHASE 15)      
================================================================================

  [PASS] GDT Segments (Kernel 0x08/0x10, User 0x18/0x20, TSS 0x28) verified.
  [PASS] TSS RSP0 configured at 0x257040 (16 KiB kernel stack).
  [PASS] User address space mapped: Code at 0x40000000, Stack at 0x7FFFF000
  Transitioning from Ring 0 to Ring 3 via iretq...
  [USER PROBE] Breakpoint 1: User mode confirmed (CPL = 3, CS = 0x23, SS = 0x1B)
  [USER PROBE] Intercepted privileged instruction 'cli' in Ring 3 (#GP, Error Code: 0x0)
  [USER PROBE] Breakpoint 2: Resumed after #GP! Ring 3 probe complete.
  [PASS] CPL == 3 (Ring 3) verified via CS register read.
  [PASS] Hardware Privilege Barrier verified: 'cli' blocked by CPU with General Protection Fault.
  [PASS] Exception gates & TSS RSP0 stack switching verified across privilege transitions.
  [PASS] ProcessAddressSpace dropped: reclaimed all 10 physical frames.

[TEST] All User Mode (Ring 3 Privilege Separation) Verification Tests PASSED!

[KERNEL] Phase 15 Milestone achieved: User Mode (Ring 3 Privilege Separation) operational.
```
