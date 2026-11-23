# Kinetix OS — Application Architecture Specification

## 1. Objective and Requirements

A major objective of Kinetix OS is the execution of independent, native user applications stored separately from the kernel binary on the bootable medium.

Applications must:
1. Be compiled independently of the kernel binary.
2. Be stored as separate files on the storage medium (`/KINETIX/APPS/<name>.ELF`).
3. Interacting with the operating system strictly through the Kinetix System Call ABI.
4. Execute in user mode (Ring 3) with isolated virtual address spaces.
5. Terminate cleanly without compromising the stability of the operating system kernel.

---

## 2. Executable Format and Binary Structure

* **Format**: Standard 64-bit ELF (Executable and Linkable Format).
* **Architecture**: `x86_64` (Little Endian, `EM_X86_64`).
* **Relocation Model**: Static ELF executable with fixed base address, or Position-Independent Executable (PIE) with dynamic relocations.
* **Initial User Address Space Layout**:
  - `0x0000_0000_0040_0000`: Base address for user application `.text` (code segment).
  - Subsequent pages: `.rodata`, `.data`, `.bss`.
  - `0x0000_7FFF_FFFF_0000`: Top of user stack (expanding downward, guard page protected).
  - `0x0000_1000_0000_0000`: Base for user heap dynamic memory allocations.

---

## 3. Two-Stage Application Loading Roadmap

```
+-------------------------------------------------------------------------+
| STAGE A: Boot-Time Application Staging (Initial Milestone)               |
|                                                                         |
|  USB Volume                                                             |
|  +--------------------+                                                 |
|  | /KINETIX/APPS/*.ELF| ---> Bootloader reads via UEFI SimpleFileSystem |
|  +--------------------+      Loads into reserved RAM frames             |
|                              Records pointer & size in BootInfo         |
|                                         |                               |
|                                         v                               |
|                              Kernel initializes paging, maps pages to   |
|                              Ring 3 address space, and executes!        |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
| STAGE B: Runtime Autonomous Application Loading (Advanced Milestone)   |
|                                                                         |
|  Kernel Storage Driver                                                  |
|  (PCI -> AHCI/xHCI -> Block Driver -> FAT32 Filesystem Driver)          |
|                                                                         |
|  - User or shell enters: "run calculator.elf"                           |
|  - Kernel opens USB block device independently                          |
|  - Traverses directory structure to locate file                         |
|  - Allocates fresh user page tables (PML4)                              |
|  - Reads file blocks into newly mapped user frames                      |
|  - Creates Task Control Block (TCB) & enqueues into Ready Queue         |
|  - Context switch dispatches task into Ring 3 execution                 |
+-------------------------------------------------------------------------+
```

---

## 4. Kinetix System Call ABI

Communication between user-space applications and the Kinetix kernel is mediated via the high-speed `syscall` and `sysretq` x86-64 CPU instructions.

### 4.1 Register Protocol (System V AMD64 Syscall Convention)
* **Syscall Number**: Passed in register `rax`.
* **Arguments**:
  - Argument 1: `rdi`
  - Argument 2: `rsi`
  - Argument 3: `rdx`
  - Argument 4: `r10` (Note: `rcx` is overwritten by the CPU with the return `RIP`)
  - Argument 5: `r8`
  - Argument 6: `r9`
* **Return Value**: Returned in `rax` (`0` for success, negative error code on failure).
* **Clobbered Registers**: `rcx` (stores user `RIP`), `r11` (stores user `RFLAGS`). All other general-purpose registers are preserved across the system call by the kernel stub.

### 4.2 Initial Core System Calls

| Syscall Number | Name | Arguments | Description |
|---|---|---|---|
| `1` | `sys_exit` | `exit_code: i32` | Terminates the current task and releases resources |
| `2` | `sys_print` | `buffer: *const u8, len: usize` | Prints UTF-8 string to the active console/serial |
| `3` | `sys_yield` | none | Cooperatively yields CPU timeslice to next ready task |
| `4` | `sys_sleep` | `ms: u64` | Blocks task for specified milliseconds |
| `5` | `sys_get_time` | `out_ticks: *mut u64` | Queries current monotonic CPU tick counter |
| `6` | `sys_alloc_pages`| `count: usize -> *mut u8` | Allocates virtual memory pages for user heap |

---

## 5. Application Runtime Library (`kinetix-runtime`)

Native applications do not make raw inline assembly syscalls directly in their business logic. Instead, they depend on `crates/kinetix-runtime`, a lightweight `#![no_std]` Rust library that provides:
1. The `_start` entry point for user applications.
2. Panic handling for applications (invokes `sys_exit(-1)`).
3. Safe Rust wrappers around each system call.
4. Optional simple memory allocator for user-space dynamic data structures.
5. Standard standard-out printing macros (`kinetix_print!`, `kinetix_println!`).
