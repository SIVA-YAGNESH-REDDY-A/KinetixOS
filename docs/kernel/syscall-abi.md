# System Call ABI (`syscall` / `sysret`) Architecture

## 1. Architectural Overview & Design Motivation

In the x86_64 architecture, traditional software interrupt gates (e.g. `int 0x80`) incur substantial processor cycle penalties due to IDT lookup, descriptor privilege checks, and complete hardware stack frame pushes. Modern 64-bit operating systems utilize the **Fast System Call Facility** via processor instructions `syscall` and `sysretq`.

Phase 16 establishes the complete **Kinetix System Call Application Binary Interface (ABI)**, providing low-overhead, high-performance transitions from unprivileged user processes (**Ring 3**) into the kernel (**Ring 0**), complete register preservation, security pointer validation, and standard POSIX-compatible error reporting.

```
+-----------------------------------------------------------------------------+
|                               USER SPACE (Ring 3)                           |
|  - Applications linked at 0x4000_0000 (PDPT[1], 1 GiB .. 2 GiB)             |
|  - System Call Invocation:                                                  |
|      RAX = Syscall Number (1..5)                                            |
|      RDI = Arg 1, RSI = Arg 2, RDX = Arg 3, R10 = Arg 4, R8 = Arg 5        |
|      Execute `syscall` instruction                                          |
+-----------------------------------------------------------------------------+
                                      |
                      Hardware Fast Call Transition (`syscall`)
                      - Hardware copies RIP -> RCX, RFLAGS -> R11
                      - Hardware loads CS & SS from IA32_STAR
                      - Hardware masks RFLAGS &= ~IA32_SFMASK
                      - Hardware jumps to address in IA32_LSTAR
                                      |
+-----------------------------------------------------------------------------+
|                            KERNEL ENTRY (`syscall_entry`)                    |
|  - Save user RSP to scratch location (`USER_SAVED_RSP`)                     |
|  - Switch stack pointer to dedicated `KERNEL_SYSCALL_STACK` (16 KiB)        |
|  - Push user context: [User RSP, RFLAGS (R11), RIP (RCX), RBP, RBX, R12..15] |
|  - Reload kernel data segments (0x10) for DS & ES                           |
|  - Marshal arguments to System V AMD64 ABI (rdi, rsi, rdx, rcx, r8, r9)    |
|  - Dispatch to Rust handler: `syscall_dispatch(...)`                         |
|  - Reload user data segments (0x1B) for DS & ES                             |
|  - Restore user context and user RSP                                        |
|  - Return to Ring 3 via `sysretq` (or exit kernel via `exit_user_process`)  |
+-----------------------------------------------------------------------------+
```

---

## 2. Model-Specific Registers (MSR) Configuration

The fast system call interface is governed by four x86_64 Model-Specific Registers configured during kernel initialization ([`syscall/entry.rs`](file:///d:/kinetix_os/kernel/kinetix-kernel/src/syscall/entry.rs)):

| Register Name | MSR Address | Purpose | Kinetix OS Value / Configuration |
|---|---|---|---|
| `IA32_EFER` | `0xC000_0080` | Extended Feature Enable Register | Bit 0 (`SCE` = System Call Extensions) set to `1` |
| `IA32_STAR` | `0xC000_0081` | Segment Selector Base Register | Bits [47:32] = `0x0008` (Kernel CS/SS), Bits [63:48] = `0x0010` (User CS/SS Base) |
| `IA32_LSTAR` | `0xC000_0082` | Long Mode Syscall Target Address | `&syscall_entry` (Linear 64-bit entry point address) |
| `IA32_SFMASK` | `0xC000_0084` | System Call Flag Mask | `0x700` (Masks `IF` bit 9, `DF` bit 10, `TF` bit 8) |

### 2.1 Selector Mechanics in `IA32_STAR`
- **Syscall Entry**: When `syscall` executes, the CPU loads:
  $$\text{CS} = \text{STAR}[47:32] = \text{0x08} \quad (\text{Kernel Code, RPL 0})$$
  $$\text{SS} = \text{STAR}[47:32] + 8 = \text{0x10} \quad (\text{Kernel Data, RPL 0})$$
- **Sysret Exit (`sysretq`)**: When returning to 64-bit Ring 3, the CPU loads:
  $$\text{SS} = \text{STAR}[63:48] + 8 = \text{0x10} + 8 = \text{0x18} \mid 3 = \text{0x1B} \quad (\text{User Data, RPL 3})$$
  $$\text{CS} = \text{STAR}[63:48] + 16 = \text{0x10} + 16 = \text{0x20} \mid 3 = \text{0x23} \quad (\text{User Code, RPL 3})$$

---

## 3. Register Preservation & Stack Switching

Unlike hardware interrupts, the x86_64 `syscall` instruction **does not switch the stack pointer (`RSP`)**. The hardware assumes software will switch stacks safely:

1. `syscall_entry` immediately records the incoming user `RSP` into static storage `USER_SAVED_RSP`.
2. `RSP` is loaded with `KERNEL_SYSCALL_STACK` (16 KiB dedicated kernel stack in identity-mapped BSS).
3. The user's execution context is pushed to the kernel stack:
   - User `RSP` (from `USER_SAVED_RSP`)
   - User `RFLAGS` (saved in `R11` by CPU hardware)
   - User `RIP` (saved in `RCX` by CPU hardware)
   - Callee-saved registers (`RBP`, `RBX`, `R12`, `R13`, `R14`, `R15`)
4. Arguments are mapped to the System V AMD64 calling convention:
   - Syscall Number: `RAX` $\rightarrow$ `RDI`
   - Arg 1: `RDI` $\rightarrow$ `RSI`
   - Arg 2: `RSI` $\rightarrow$ `RDX`
   - Arg 3: `RDX` $\rightarrow$ `RCX`
   - Arg 4: `R10` $\rightarrow$ `R8`
   - Arg 5: `R8` $\rightarrow$ `R9`

---

## 4. Kinetix System Call Table & Security Boundaries

| Syscall Number | Constant | Arguments | Return Value | Functional Description |
|---|---|---|---|---|
| `1` | `SYS_PRINT` | `a1`: buffer ptr, `a2`: length | Number of bytes or `-EFAULT` (`-14`) | Prints validated UTF-8 string to serial and framebuffer console |
| `2` | `SYS_YIELD` | None | `0` (Success) | Voluntarily yields remaining CPU time-slice via `task::yield_now()` |
| `3` | `SYS_EXIT` | `a1`: exit code (`i32`) | Does not return | Terminates current user process, unwinding back to kernel runner |
| `4` | `SYS_GETPID` | None | `u64` (Process ID) | Returns current active process identifier |
| `5` | `SYS_UPTIME_MS` | None | `u64` (Milliseconds) | Returns elapsed system uptime since boot via PIT timer ticks |

### 4.1 Security Boundary Enforcement (`-EFAULT`)
To prevent unprivileged user code from probing or reading supervisor memory, `SYS_PRINT` strictly validates incoming pointers:
$$\text{Valid Address Range: } [\text{0x4000\_0000}, \; \text{0x8000\_0000})$$
If a user program passes a pointer outside this range (e.g. attempting to read kernel memory at `0x0020_0000`), the kernel immediately halts evaluation and returns `EFAULT` (`-14`).

---

## 5. Verification Results (Oracle VirtualBox 7.2)

```
================================================================================
          KINETIX OS SYSTEM CALL ABI (SYSCALL / SYSRET) TESTS (PHASE 16)        
================================================================================

  [PASS] Hardware Fast System Call MSRs (EFER, STAR, LSTAR, SFMASK) verified.
  [PASS] IA32_STAR Selectors: Kernel CS 0x08 / SS 0x10, User SS 0x1B / CS 0x23.
  [PASS] User address space mapped (Entry: 0x40000000, Stack: 0x7FFFF000).
  Transitioning to Ring 3 to execute user syscall test suite...
  [USER] Syscall SYS_PRINT invoked!
  [PASS] SYS_PRINT verified (printed message from Ring 3 to console).
  [PASS] Security boundary verified: SYS_PRINT with kernel pointer rejected (-EFAULT).
  [PASS] SYS_GETPID verified (returned PID 1).
  [PASS] SYS_UPTIME_MS verified (returned positive tick uptime).
  [PASS] SYS_YIELD verified (voluntarily yielded and resumed execution in Ring 3).
  [PASS] SYS_EXIT verified: process terminated cleanly with exit status 42.
  [PASS] ProcessAddressSpace dropped: reclaimed all 10 physical frames.

[TEST] All System Call ABI (syscall / sysret) Verification Tests PASSED!

[KERNEL] Phase 16 Milestone achieved: System Call ABI (syscall / sysret) operational.
```
