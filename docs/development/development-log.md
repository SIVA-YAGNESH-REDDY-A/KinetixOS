# Kinetix OS — Engineering Development Log

This document tracks chronological engineering milestones, architectural choices, build verifications, and technical resolutions.

---

### [2026-11-04] — Phase 0: Development Environment & Dependency Audit
* **Action**: Inspected host hardware (Lenovo LOQ, i5-13450HX, 16 threads, 16 GB RAM, 241 GB free on D:).
* **Action**: Confirmed lack of Rust and VirtualBox installations. Decided on self-contained `stable-x86_64-pc-windows-gnu` host toolchain to eliminate large MSVC / Windows SDK footprint.
* **Action**: Installed Rust `1.99.1` into `D:\Rust` (`RUSTUP_HOME=D:\Rust\rustup`, `CARGO_HOME=D:\Rust\cargo`).
* **Action**: Installed targets `x86_64-unknown-uefi` and `x86_64-unknown-none`.
* **Action**: Verified compiler and bundled `rust-lld` linker by compiling minimal test executables in scratch directory; verified PE32+ and ELF64 formats using `llvm-objdump`.
* **Action**: Downloaded official, Authenticode-verified Oracle VirtualBox 7.2.22 installer to `D:\VirtualBox-7.2.22-Win.exe`.
* **Documentation**: Generated `docs/development/environment-setup.md` and `docs/development/dependency-audit.md`.

---

### [2026-11-06] — Phase 1: Architecture and Technical Design
* **Action**: Formulated comprehensive system architecture (`docs/architecture/system-architecture.md`).
* **Action**: Specified the deterministic UEFI-to-kernel boot sequence (`docs/architecture/boot-flow.md`).
* **Action**: Established runtime independence constraints (`docs/architecture/runtime-independence.md`).
* **Action**: Specified the native ELF64 application loading and system call ABI (`docs/architecture/application-architecture.md`).
* **Action**: Created Architecture Decision Records ADR-001 through ADR-005.

---

### [2026-11-06] — Phase 2: Project Workspace Initialization
* **Action**: Formulated modular Cargo workspace manifest (`Cargo.toml`) with pinned `rust-toolchain.toml`.
* **Action**: Configured compilation targets in `.cargo/config.toml`.
* **Action**: Created standard project guidelines (`README.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, `.gitignore`).
* **Action**: Initialized Git repository and created initial commit `be231d0`.

---

### [2026-11-06] — Phase 3: First UEFI Bootloader & Disk Image Builder
* **Action**: Implemented `kinetix-bootloader` in pure Rust targetting `x86_64-unknown-uefi`.
* **Action**: Implemented 16550 UART COM1 serial diagnostics driver (`bootloader/kinetix-bootloader/src/serial.rs`).
* **Action**: Implemented pure-Rust GPT partition table and protective MBR generator (`tools/kinetix-image-builder/src/gpt.rs`) without external C or dynamic library dependencies.
* **Action**: Generated first raw 64 MiB GPT bootable disk image with FAT32 ESP partition (`artifacts/boot-images/kinetix-os-bootloader.img`). Verified MBR signature `0xAA55` and GPT signature `'EFI PART'`.

---

### [2026-11-06] — Phase 4: First Kinetix Kernel
* **Action**: Implemented freestanding 64-bit kernel in Rust targetting `x86_64-unknown-none`.
* **Action**: Configured kernel linker script (`kernel/kinetix-kernel/linker.ld`) placing `.text` at 2 MiB (`0x0020_0000`) with 4 KiB page-aligned sections.
* **Action**: Implemented COM1 serial logging macros (`print!`, `println!`) protected by spinlock `Mutex`.
* **Action**: Implemented direct linear framebuffer console scaffolding.
* **Action**: Verified ELF64 binary format and entry address `0x0020_0000` via `llvm-objdump`.

---

### [2026-11-06] — Phase 5 & 6: Bootloader-to-Kernel Integration & VirtualBox First Boot
* **Action**: Implemented ELF64 parser and loader in `kinetix-bootloader` (`loader.rs`).
* **Action**: Implemented file reading via UEFI `SimpleFileSystem` protocol to load `/KERNEL.ELF` and `/KINETIX/KERNEL/KINETIX-KERNEL.ELF`.
* **Action**: Allocated physical pages and mapped `PT_LOAD` segments into memory at `0x0020_0000`.
* **Action**: Initialized GOP framebuffer (1024x768, 32-bpp BGR) and recorded metadata into `BootInfo`.
* **Action**: Resolved ABI calling convention boundary between UEFI MS x64 and Kernel System V AMD64 using `extern "sysv64"` on Rust stable.
* **Action**: Successfully performed `ExitBootServices()`, converted UEFI memory map entries (111 regions, 2004 MiB usable RAM) into `BootInfo`.
* **Action**: Launched Oracle VirtualBox 7.2.22 with 64-bit UEFI firmware and COM1 serial redirection.
* **Milestone**: **MILESTONE A ACHIEVED — KINETIX OS FIRST BOOT IN VIRTUALBOX!** Verified via live serial logs:
  - Magic signature `0x4B494E4554495821` verified.
  - Usable physical memory: 2004 MiB.
  - Linear framebuffer: `0x80000000` (1024x768).
  - Kernel Status: **RUNNING**.

---

### [2026-11-06] — Phase 7: Kernel Memory Management Subsystem
* **Action**: Designed and implemented 3-tier memory management foundation:
  1. `BitmapFrameAllocator` (`kernel/kinetix-kernel/src/memory/frame.rs`): 4 GiB physical memory bitmap tracking 1,048,576 frames (4 KiB each). 128 KiB bitmap placed in kernel BSS (zero file footprint). Scans UEFI memory map, sets usable frames, and enforces reservation guards for Page 0, real-mode/BIOS (0..1 MiB), kernel binary image (0x200000..0x229000), framebuffer (0x80000000), and kernel heap.
  2. 4-Level Paging (`kernel/kinetix-kernel/src/memory/paging.rs`): Virtual address decomposition (PML4, PDPT, PD, PT), page table entries, flags, active CR3 register reading via inline assembly, and page table walker translating virtual to physical addresses.
  3. Intrusive Coalescing Free-List Kernel Heap Allocator (`kernel/kinetix-kernel/src/memory/heap.rs`): Implements `core::alloc::GlobalAlloc` with boundary-tag address sorting and immediate coalescing to prevent heap fragmentation. Configured with a 4 MiB initial heap at physical `0x0040_0000`. Registered as `#[global_allocator]`.
* **Action**: Unlocked Rust's `alloc` crate (`extern crate alloc;`) in `kinetix-kernel`, enabling `Box`, `Vec`, `String`, and standard collections.
* **Action**: Added exported linker symbols `__kernel_start` and `__kernel_end` in `linker.ld` to dynamically guard kernel memory pages.
* **Action**: Implemented in-kernel memory self-tests verifying frame alloc/dealloc/alignment, page table virtual-to-physical translation, `Box` allocation/drop, dynamic `Vec` growth (500 elements), and `format!` heap strings.
* **Action**: Updated `scripts/windows/run-virtualbox.ps1` to detect and stop running VMs before disk conversion and lock release.
* **Milestone**: **PHASE 7 COMPLETED & VERIFIED IN VIRTUALBOX**. Serial log verified all 5 tests passed:
  - Total usable RAM: 2004 MiB (513,105 frames).
  - Free physical RAM: 2003 MiB (512,945 frames).
  - Active CR3 root: `0x7EE01000`.
  - Kernel Heap available: 4096 KiB (4,194,304 bytes).
  - All tests `[PASS]`. Status: `Phase 7 Milestone achieved: Kernel Memory Management operational.`

---

### [2026-11-06] — Phase 8: CPU Interrupts & Exception Infrastructure
* **Action**: Designed and implemented complete CPU interrupt and exception subsystem:
  1. `gdt.rs` (`kernel/kinetix-kernel/src/arch/gdt.rs`): 64-bit Kernel Code (`0x08`), Kernel Data (`0x10`), and Task State Segment (`0x18`) with dedicated 16 KiB Double Fault IST stack (`DOUBLE_FAULT_STACK`). Loaded via `lgdt`, `ltr`, and far `retfq`.
  2. `idt.rs` (`kernel/kinetix-kernel/src/arch/idt.rs`): 256-entry 64-bit Interrupt Descriptor Table loaded via `lidt`. Mapped all 32 x86_64 CPU exceptions (vector 8 `#DF` mapped to IST1), IRQ 0 (timer), and IRQ 1 (keyboard).
  3. `pic.rs` (`kernel/kinetix-kernel/src/arch/pic.rs`): Dual 8259 PIC driver remapping hardware IRQs 0..15 to interrupt vectors 32..47.
  4. `pit.rs` (`kernel/kinetix-kernel/src/arch/pit.rs`): 8254 PIT driver operating at 100 Hz (10 ms ticks) driving `TICKS` atomic counter for preemptive scheduling.
  5. `interrupts.rs` (`kernel/kinetix-kernel/src/arch/interrupts.rs`): Low-level assembly entry stubs (`global_asm!`), `TrapFrame` register capture (RAX..R15, vector, error code, RIP, CS, RFLAGS, RSP, SS), and Rust dispatchers.
* **Action**: Implemented in-kernel verification tests for software breakpoint exception (`int3`) and hardware PIT timer ticks (`sti` + `halt`).
* **Milestone**: **PHASE 8 COMPLETED & VERIFIED IN VIRTUALBOX**. Serial log verified:
  - GDT & TSS initialized and active.
  - IDT initialized and loaded into CPU.
  - Software breakpoint (`#BP`) caught at RIP `0x20304A` and execution safely resumed.
  - Hardware timer interrupts active: delivered 10 ticks (100 ms) via IRQ 0.
  - Status: `Phase 8 Milestone achieved: CPU Interrupts & Exception Infrastructure operational.`

---

### [2026-11-06] — Phase 9: Hardware Interaction Layer (Graphical Console & PS/2 Keyboard)
* **Action**: Designed and implemented the hardware interaction and console output layer:
  1. `font.rs` (`kernel/kinetix-kernel/src/console/font.rs`): Complete 8x16 monospace font table embedded in kernel `.rodata` covering ASCII 32 (`' '`) to 126 (`'~'`).
  2. `console/mod.rs` (`kernel/kinetix-kernel/src/console/mod.rs`): Direct linear framebuffer text console supporting 1024x768 @ 32 bpp resolution (128 columns $\times$ 48 rows text grid), automatic line wrapping, backspace (`\x08`), tab expansion (`\t`), and hardware-accelerated memory-move vertical row scrolling.
  3. Dual Console Dispatcher: Integrated `print!` and `println!` macros to simultaneously broadcast all kernel logging to COM1 serial (0x3F8) and the on-screen graphical display.
  4. `keyboard.rs` (`kernel/kinetix-kernel/src/arch/keyboard.rs`): PS/2 Keyboard Controller driver decoding Scancode Set 1 make/break codes, tracking Shift, Caps Lock, Ctrl, Alt states, and buffering typed keys in a thread-safe 256-entry circular ring buffer (`KEYBOARD_BUFFER`).
* **Action**: Updated `scripts/windows/run-virtualbox.ps1` to prevent PowerShell `$ErrorActionPreference` aborting on external tool informational messages.
* **Milestone**: **PHASE 9 COMPLETED & VERIFIED IN VIRTUALBOX**. Serial log verified:
  - Console display active at 1024x768 (128 cols $\times$ 48 rows).
  - 8x16 Monospace Font rendering verified.
  - PS/2 Keyboard Set 1 scancode decoder verified (letters, shift modifiers, enter, backspace).
  - Status: `Phase 9 Milestone achieved: Hardware Interaction Layer operational.`

---

### [2026-11-06] — Phase 10: Cooperative Task Management & CPU Context Switching
* **Action**: Designed and implemented cooperative multitasking and low-level context switching foundation:
  1. `tcb.rs` (`kernel/kinetix-kernel/src/task/tcb.rs`): Task Control Block (`Task`), unique `TaskId`, lifecycle states (`Ready`, `Running`, `Blocked`, `Terminated`), and dedicated 64 KiB kernel stack allocation. Initialized synthetic stack frame with 16-byte alignment to seamlessly transition into `task_trampoline`.
  2. `context.rs` (`kernel/kinetix-kernel/src/task/context.rs`): Low-level assembly context switch (`switch_context`) saving/restoring callee-saved registers (`RBX`, `RBP`, `R12..R15`), swapping hardware stack pointer (`RSP`), and jumping via `ret`. Implemented `task_trampoline` wrapper to invoke task entry functions and automatically trigger `task::exit()` upon completion.
  3. `task/mod.rs` (`kernel/kinetix-kernel/src/task/mod.rs`): Global `TASK_MANAGER` managing task queue, boot thread registration (Task 0 "Kernel Main"), task spawning (`task::spawn`), voluntary CPU yielding (`yield_now`), and graceful termination (`exit`).
* **Action**: Implemented in-kernel verification test spawning Task 1 ("Task Alpha") and Task 2 ("Task Beta") interleaving execution in lockstep through `yield_now()`.
* **Milestone**: **PHASE 10 COMPLETED & VERIFIED IN VIRTUALBOX**. Serial log verified:
  - Task 0 ("Kernel Main") registered as active running thread.
  - Task 1 ("Task Alpha") and Task 2 ("Task Beta") spawned.
  - Interleaved execution: Alpha 1 -> Beta 1 -> Alpha 2 -> Beta 2 -> Alpha 3 -> Beta 3.
  - Status: `Phase 10 Milestone achieved: Cooperative Task Management & Context Switching operational.`

---

### [2026-11-06] — Phase 11: Preemptive Multi-Policy CPU Scheduler
* **Action**: Designed and implemented timer-driven preemptive multitasking and multi-policy scheduling engine:
  1. `scheduler.rs` (`kernel/kinetix-kernel/src/task/scheduler.rs`): Implemented three distinct scheduling disciplines:
     - `RoundRobin`: Circular time-sliced task distribution.
     - `Priority`: Strict priority preemption with priority-weighted quantums (Low = 20 ms, Normal = 40 ms, High = 80 ms, Realtime = 120 ms).
     - `FairShare`: Dynamic least-consumed CPU time balancing (CFS style).
  2. Telemetry and Quantum Tracking (`tcb.rs`): Added `Priority` enum, `quantum_ticks`, `ticks_remaining`, and tracking counters (`total_ticks_consumed`, `context_switches`, `preemptions`, `voluntary_yields`) preparing the infrastructure for Phase 12 academic multitasking research.
  3. Preemptive Timer Integration (`arch/interrupts.rs` & `task/mod.rs`): Hooked IRQ 0 (100 Hz PIT timer interrupt) to `task::handle_timer_tick()`. When a running task's time-slice expires, an involuntary context switch is executed directly from the timer interrupt path, saving registers and resuming the preempted task seamlessly via the `iretq` stack frame.
* **Action**: Implemented in-kernel verification tests verifying policy reconfiguration and executing compute-intensive tasks ("Worker Alpha" and "Worker Beta") with NO voluntary yields, confirming timer-driven involuntary preemption.
* **Milestone**: **PHASE 11 COMPLETED & VERIFIED IN VIRTUALBOX**. Serial log verified:
  - Policy switching verified (RoundRobin, Priority, FairShare).
  - Worker Alpha and Worker Beta executed concurrently to completion under timer preemption.
  - Status: `Phase 11 Milestone achieved: Preemptive Multi-Policy CPU Scheduler operational.`

---

### [2026-11-06] — Phase 12: CPU Multitasking Academic Research & Workload Benchmarks
* **Action**: Designed and implemented the comprehensive empirical research benchmark engine:
  1. `metrics.rs` (`kernel/kinetix-kernel/src/research/metrics.rs`): High-precision cycle measurement via `rdtsc` (`read_tsc()`), 128-bit integer formulation of Jain's Fairness Index (`compute_jains_fairness`), and structured telemetry records (`BenchmarkResult`, `TaskMetric`).
  2. `workloads.rs` (`kernel/kinetix-kernel/src/research/workloads.rs`): Implemented controlled workload suites with `core::hint::black_box` to prevent compiler dead-code elimination:
     - Pure Compute (Symmetric CPU-bound hashing and polynomial arithmetic).
     - Interactive I/O Simulation (Short compute bursts followed by voluntary yields).
     - Heterogeneous Mixed Workload (Co-running compute-bound and interactive I/O tasks).
  3. `research/mod.rs` (`kernel/kinetix-kernel/src/research/mod.rs`): Coordinated 5 empirical benchmark trials across Round-Robin, Priority-Based, and Fair-Share policies. Emitted publication-grade ASCII telemetry tables and per-task execution breakdowns to both serial and linear framebuffer console.
  4. Scheduler Benchmarking Extensions (`task/mod.rs`): Added `reset_for_benchmark()`, `has_active_workers()`, and `collect_worker_metrics()` to enable isolated benchmark trials with automatic memory reclamation of terminated task stacks.
* **Action**: Published academic research paper `docs/research/multitasking-performance.md` documenting methodology, mathematical models, empirical findings, and architectural conclusions.
* **Milestone**: **PHASE 12 COMPLETED & VERIFIED IN VIRTUALBOX**. Serial log verified:
  - Trial 1 (Pure Compute, Round-Robin): 619.4M cycles, 9 ticks, 4 switches, 1 preemption, 60% fairness.
  - Trial 2 (Interactive I/O, Round-Robin): 14.7M cycles, 0 ticks, 61 switches, 30 voluntary yields, 0 preemptions, 100% fairness.
  - Trial 3 (Mixed Heterogeneous, Round-Robin): 491.2M cycles, 8 ticks, 43 switches, 1 preemption, 20 yields, 47% fairness.
  - Trial 4 (Mixed Heterogeneous, Priority): 487.5M cycles, 9 ticks, 51 switches, 4 preemptions, 20 yields, 49% fairness.
  - Trial 5 (Mixed Heterogeneous, Fair-Share): 479.9M cycles, 6 ticks, 44 switches, 1 preemption, 20 yields, 25% fairness.
  - Status: `Phase 12 Milestone achieved: CPU Multitasking Academic Research complete.`

---

### [2026-11-06] — Phase 13: In-Memory / Read-Only Boot Filesystem (tar / ustar)
* **Action**: Designed and implemented the in-memory boot filesystem (RamFS) and Virtual Filesystem (VFS) abstraction:
  1. `ustar.rs` (`kernel/kinetix-kernel/src/fs/ustar.rs`): Zero-copy POSIX ustar archive parser with 512-byte header parsing, octal number decoding, checksum mathematical validation, path normalization, and entry iteration.
  2. `vfs.rs` (`kernel/kinetix-kernel/src/fs/vfs.rs`): In-memory RAM filesystem (`RamFs`) supporting hierarchical directory traversal (`list_dir()`), file metadata queries (`stat()`), existence checks (`exists()`), and direct byte slice opening (`open()`).
  3. `fs/mod.rs` (`kernel/kinetix-kernel/src/fs/mod.rs`): Unified VFS dispatcher embedding compile-time archive (`EMBEDDED_BOOTFS` via `include_bytes!`) and dynamically overlaying bootloader-staged external modules (`BootInfo.modules`).
  4. Tooling & Packaging:
     - Configured `scripts/windows/build-kernel.ps1` to automatically package `artifacts/ramdisk/` into `kernel/kinetix-kernel/bootfs.tar`.
     - Populated initial filesystem: `/bin/hello-world` (native ELF64 binary), `/etc/os-release`, `/etc/motd`, `/README.txt`.
     - Updated `tools/kinetix-image-builder` and `bootloader/kinetix-bootloader` to stage `BOOTFS.TAR` and `HELLO-WORLD.ELF` into `BootInfo.modules`.
* **Action**: Published architecture specification: `docs/kernel/filesystem-architecture.md`.
* **Milestone**: **PHASE 13 COMPLETED & VERIFIED IN VIRTUALBOX**. Serial log verified:
  - Staged module `BOOTFS.TAR` detected at `0x7D483000` (7680 bytes).
  - RamFS root directory listing verified (3 top-level entries: `bin/`, `etc/`, `README.txt`).
  - `stat('/etc/os-release')` verified (175 bytes, mode 0o666).
  - Header block checksum verified.
  - File reading verified (`/etc/os-release`, `/etc/motd`).
  - Executable inspection: `/bin/hello-world` verified as valid ELF64 binary (1144 bytes, magic: `0x7F 'ELF'`).
  - Status: `Phase 13 Milestone achieved: In-Memory Boot Filesystem (RamFS / ustar) operational.`

---

### [2026-11-06] — Phase 14: Native Application Loader (User ELF64 Relocation)
* **Action**: Designed and implemented the native ELF64 application loader and isolated user process address space subsystem:
  1. `app.ld` (`apps/hello-world/app.ld`): Created dedicated user-space application linker script linking `hello-world` at virtual address `0x4000_0000` (1 GiB boundary, within small code model reach), separate from kernel code at 2 MiB and kernel heap at 4 MiB.
  2. `elf.rs` (`kernel/kinetix-kernel/src/loader/elf.rs`): Zero-copy ELF64 header and program header parser validating machine type (`EM_X86_64`), class (`ELFCLASS64`), little-endian format, entry point, and loadable segment flags (`PF_R`, `PF_W`, `PF_X`).
  3. `process.rs` (`kernel/kinetix-kernel/src/loader/process.rs`): Implemented `ProcessAddressSpace` creating independent 4-level page tables (`PML4` -> `PDPT` -> `PD` -> `PT`). Inherits kernel identity mapping (0..1 GiB) via `PDPT[0]`, dedicated user space in `PDPT[1]` (`0x4000_0000`..`0x7FFF_FFFF`), user stack allocation (16 KiB at `0x7FFF_F000`), and automatic physical frame cleanup on `Drop`.
  4. `loader/mod.rs` (`kernel/kinetix-kernel/src/loader/mod.rs`): High-level loader API reading binaries directly from the in-memory RamFS (`/bin/hello-world`), mapping segments, zeroing `.bss`, and running verification tests.
* **Action**: Published architecture specification: `docs/kernel/application-loader.md`.
* **Milestone**: **PHASE 14 COMPLETED & VERIFIED IN VIRTUALBOX**. Serial log verified:
  - Loaded `/bin/hello-world` (9008 bytes) into isolated `ProcessAddressSpace`.
  - Entry point verified at `0x40000000`.
  - Virtual-to-physical translation verified (`0x40000000` -> `0x102000`).
  - Read-only data segment content verified (`"Hello from Kinetix OS Native Application!"` at `0x4000_1000`).
  - User stack verified (16 KiB at `0x7FFF_F000`, canary written and readback confirmed).
  - 11 physical frames tracked and 100% reclaimed on drop.
  - Rejection of invalid/corrupted binaries verified.
  - Status: `Phase 14 Milestone achieved: Native Application Loader (User ELF64 Relocation) operational.`

## Phase 15: User Mode (Ring 3 Privilege Separation) — 2026-11-06
- **Objective**: Implement hardware-enforced CPU privilege level separation (Ring 0 Kernel vs. Ring 3 User), GDT user segment descriptors, Task State Segment (TSS) RSP0 privilege stack switching, synthetic `iretq` frame transition, hardware privilege fault enforcement (`#GP` on privileged instructions), software breakpoint trap gates (`#BP` with IDT DPL = 3), and clean kernel context unwinding.
- **Architectural Implementation**:
  - `arch/gdt.rs`: Configured User Data Selector (`0x18`, RPL 3: `0x1B`), User Code Selector (`0x20`, RPL 3: `0x23`), TSS (`0x28`), and dedicated 16 KiB `KERNEL_RSP0_STACK` for hardware privilege transitions.
  - `arch/idt.rs`: Implemented `IdtEntry::set_dpl(dpl)` and configured Vector 3 (`#BP`) with DPL = 3 (`type_attr = 0xEE`) allowing user-mode execution of `int3`.
  - `user/ring3.rs`: Created assembly routines `run_user_mode_probe` (saving callee-saved registers, pushing `iretq` frame, setting user data segments, loading user CR3, dropping to Ring 3 via `iretq`) and `return_from_user_mode_probe` (restoring kernel CR3, kernel segment registers, original kernel RSP, restoring registers, returning status 1).
  - `user/mod.rs`: Test coordinator and exception hooks (`handle_user_breakpoint`, `handle_user_gp_fault`), executing user payload testing CS RPL, `cli` instruction blocking, and breakpoint resumption.
  - `loader/process.rs`: Enhanced page directory cloning to copy higher physical mappings (e.g. `PDPT[3]` for GOP framebuffer at `0xE000_0000`), allowing dual console telemetry without secondary page faults.
  - `main.rs`: Switched kernel execution to dedicated 64 KiB `KERNEL_STACK` located in identity-mapped BSS (`PDPT[0]`).
- **Empirical Verification (Oracle VirtualBox 7.2)**:
  - GDT selectors and TSS RSP0 verified.
  - User mode confirmed: CPL == 3 recorded via CS register read (`CS = 0x23`, `SS = 0x1B`).
  - Hardware privilege barrier: privileged instruction `cli` blocked by CPU with General Protection Fault (`#GP`, Error Code 0x0).
  - Resumption after `#GP`: `frame.rip += 1` advanced past faulting instruction and reached second breakpoint cleanly.
  - Stack switching and clean kernel unwinding: returned safely to caller, dropped `ProcessAddressSpace`, and reclaimed all 10 physical frames.
  - Status: `[KERNEL] Phase 15 Milestone achieved: User Mode (Ring 3 Privilege Separation) operational.`

## Phase 16: System Call ABI (`syscall` / `sysret`) — 2026-11-06
- **Objective**: Design and implement high-performance, low-latency fast system calls using x86_64 `syscall` / `sysretq`, Model-Specific Register configuration (MSRs `0xC0000080`..`0xC0000084`), kernel stack switching, System V AMD64 argument marshalling, user-space pointer bounds validation (`-EFAULT`), and core system services (`SYS_PRINT`, `SYS_YIELD`, `SYS_EXIT`, `SYS_GETPID`, `SYS_UPTIME_MS`).
- **Architectural Implementation**:
  - `arch/cpu.rs`: Implemented `rdmsr` and `wrmsr` assembly intrinsics.
  - `syscall/entry.rs`: Configured `IA32_EFER.SCE`, `IA32_STAR` (Kernel 0x08/0x10, User 0x1B/0x23), `IA32_LSTAR` (`syscall_entry`), and `IA32_SFMASK` (masking IF, DF, TF). Built assembly entry/exit stubs with stack switching to dedicated 16 KiB `KERNEL_SYSCALL_STACK`, register preservation, and `run_user_process` / `exit_user_process_asm`.
  - `syscall/mod.rs`: Implemented central `syscall_dispatch` routing with user address validation (`0x4000_0000 .. 0x8000_0000`), UTF-8 validation, and core syscall implementations.
  - `crates/kinetix-runtime`: Unified user-space runtime constants and wrappers (`sys_print`, `sys_yield`, `sys_exit`, `getpid`, `uptime_ms`).
  - `main.rs`: Integrated `syscall::init()` and `syscall::run_syscall_tests()`.
- **Empirical Verification (Oracle VirtualBox 7.2)**:
  - Hardware MSRs verified: `EFER.SCE` set, `STAR` selectors correct, `LSTAR` points to `syscall_entry`, `SFMASK` masks interrupts.
  - `SYS_PRINT` verified: unprivileged Ring 3 probe printed string to console.
  - Security boundary verified: `SYS_PRINT` with kernel pointer `0x200000` rejected with `EFAULT` (`-14`).
  - `SYS_GETPID` returned PID 1; `SYS_UPTIME_MS` returned positive tick count.
  - `SYS_YIELD` voluntarily yielded CPU and resumed execution in Ring 3 cleanly.
  - `SYS_EXIT(42)` terminated user execution and returned exit code 42 to kernel caller.
  - All 10 user physical frames reclaimed on drop.
  - Status: `[KERNEL] Phase 16 Milestone achieved: System Call ABI (syscall / sysret) operational.`

## Phase 17: Native Demonstration Applications — 2026-11-06
- **Objective**: Design, package, execute, and verify native user applications running in unprivileged Ring 3 using `kinetix-runtime`, communicating exclusively through the hardware fast system call ABI (`syscall` / `sysretq`), and validating complete lifecycle isolation, compute benchmarks, and physical memory reclamation.
- **Architectural Implementation**:
  - `crates/kinetix-runtime`: Extended user runtime with low-level assembly stubs (`syscall0`, `syscall1`, `syscall2`), numeric formatters (`print_u64`, `print_i64`), and ergonomic API (`print_str`, `yield_now`, `getpid`, `uptime_ms`, `exit`).
  - `apps/hello-world`: Refactored to pure runtime API, displaying greeting banner, PID 1, uptime in ms, and exiting cleanly with code 0.
  - `apps/sysinfo`: Implemented system introspection utility that verifies CPL == 3 from user CS register, queries OS name, AMD64 target architecture, UEFI firmware, syscall ABI linkage, process identity, and boot uptime.
  - `apps/bench`: Implemented user-space arithmetic prime-search benchmark (`2..3000`) with periodic cooperative `yield_now()` invocations every 500 iterations, mathematically verifying compiler register preservation across syscall boundaries.
  - `kernel/syscall/entry.rs`: Fixed user register preservation bug in `syscall_entry` assembly stub: preserved all 15 general-purpose registers (including loop counter `RSI`) plus user stack alignment, ensuring user loops and register states remain pristine across kernel dispatch.
  - `kernel/loader/mod.rs`: Implemented `run_demonstration_apps()`: sequentially loads `/bin/hello-world`, `/bin/sysinfo`, and `/bin/bench` from `bootfs.tar`, instantiates an isolated `ProcessAddressSpace` for each, drops into Ring 3, asserts exit status 0, and reclaims all 11 physical frames per app.
  - `tools/kinetix-image-builder` & `scripts/windows/build-os.ps1`: Automated compilation and FAT32/USTAR staging for all three native applications.
- **Empirical Verification (Oracle VirtualBox 7.2)**:
  - `/bin/hello-world`: Loaded at entry `0x4000_0000`, printed greeting banner, PID 1, and uptime 710 ms via `SYS_PRINT`, `SYS_GETPID`, `SYS_UPTIME_MS`; terminated with exit status 0; 11 physical frames reclaimed.
  - `/bin/sysinfo`: Loaded in clean address space, verified CPL == 3, displayed complete system profile, terminated with exit status 0; 11 physical frames reclaimed.
  - `/bin/bench`: Executed trial division over 2,999 candidates with 6 cooperative yields, discovered 430 primes, confirmed prime sum checksum 593,823, terminated with exit status 0; 11 physical frames reclaimed.
  - Status: `[KERNEL] Phase 17 Milestone achieved: Native Demonstration Applications (hello-world, sysinfo, bench) operational.`

## Phase 18: Pulses & Reticle (Universal Kinetic Pointer Subsystem) — 2026-11-07
- **Objective**: Design, implement, and verify the kinetic pointer motion vector and software cursor subsystem supporting legacy USB emulation (Flipkart SmartBuy M500 2.4 GHz wireless USB dongle and laptop touchpads) and virtualized pointing devices via 8042 controller auxiliary port and IRQ 12.
- **Architectural Implementation**:
  - `arch/pic.rs`: Implemented `ChainedPics::unmask(irq)` supporting both Master and Slave PIC IRQ lines, unmasking IRQ 12 (Bit 4 of Slave PIC) and cascade IRQ 2.
  - `arch/interrupts.rs` & `arch/idt.rs`: Added low-level assembly entry stub `irq12`, registered Vector 44 in IDT, and routed interrupt dispatch to `crate::pulses::pointer::on_interrupt()`.
  - `pulses/reticle.rs`: Designed 12×19 arrow pointer sprite with 1-pixel dark outline (`#1A1A1A`) and white core (`#FFFFFF`). Implemented dirty-rectangle background backup and restoration in `Reticle::move_to`, guaranteeing zero screen tearing and zero flicker.
  - `pulses/pointer.rs`: Configured 8042 PS/2 controller auxiliary port initialization (commands `0xA8`, `0x20`, `0x60`, `0xF6`, `0xF4`, `0xD4`), implemented 3-byte packet state machine with Bit 3 phase guard synchronization, overflow protection, and coordinate boundary clamping.
  - `main.rs`: Integrated `pulses::init()` and `pulses::pointer::run_pointer_tests()`.
- **Empirical Verification (Oracle VirtualBox 7.2)**:
  - Initial pointer position verified at display center: `(512, 384)`.
  - Motion pulse vector (+75, -50) with Left Click verified: updated coordinates to `(587, 334)` with `left_pressed == true`.
  - Negative overflow clamping verified: clamped to `(0, 0)`.
  - Positive overflow clamping verified: clamped to `(1012, 749)` with `right_pressed == true`.
  - Reticle restoration to center verified: clean background pixel restoration with zero display flicker.
  - Status: `[KERNEL] Phase 18 Milestone achieved: Pulses & Reticle (Universal Kinetic Pointer Subsystem) operational.`

## Phase 19: Canvas & Horizon (The Kinetix Desktop Environment) — 2026-11-07
- **Objective**: Design, synthesize, and verify the Kinetix OS desktop graphical user interface, featuring a 32-bit off-screen double buffer (Prism), a procedurally synthesized blue bloom/wave backdrop, a bottom taskbar dock (Horizon) with live chronometer and Kinetix spark emblem, interactive desktop Cartridge beacons (Scribe, Ouroboros, Telemetry), and movable Flow chamber window frames.
- **Architectural Implementation**:
  - `memory/heap.rs`: Expanded primary kernel heap from 4 MiB to 16 MiB (`0x0040_0000`..`0x0140_0000`) to comfortably accommodate the 3 MiB off-screen backbuffer and window surfaces.
  - `canvas/prism.rs`: Created `PrismSurface` maintaining a 1024×768 32-bit backbuffer (3,145,728 bytes) with primitives: `fill_rect`, `draw_rect_outline`, `draw_char`, `draw_text`, automatic BGR/RGB color translation, and full-screen contiguous hardware blit (`blit_to_screen`).
  - `canvas/backdrop.rs`: Implemented procedural blue bloom backdrop generation using integer fixed-point trigonometric harmonics (`SIN_LUT`), rendering smooth sky gradient transitions and central flowing blue petal ribbons without external image files.
  - `canvas/horizon.rs`: Implemented the 44-pixel bottom dock ($Y = 724..768$) with Kinetix diamond spark logo, search quick-launch plate, and live formatted chronometer uptime telemetry.
  - `canvas/beacons.rs`: Implemented interactive desktop icons (`Scribe`, `Ouroboros`, `Telemetry`) with custom 32×32 emblems, hover glass illumination (`#1E3A5F`), and hit testing.
  - `canvas/chamber.rs`: Created `FlowChamber` draggable window frame primitives with title bar header, crimson red close button (`[X]`), and drag bounds.
  - `main.rs`: Integrated `canvas::init()` and `canvas::run_canvas_tests()`.
- **Empirical Verification (Oracle VirtualBox 7.2)**:
  - Prism double buffer verified: 1024×768×4 (3,145,728 bytes).
  - Procedural backdrop verified: distinct non-zero sky and bloom petal pixels confirmed.
  - Horizon dock verified at $Y=724..768$ with Kinetix spark emblem and live chronometer.
  - Desktop beacons hit-testing verified for all 3 cartridges (Scribe, Ouroboros, Telemetry).
  - Flow chamber window geometry, title header, and close button hit testing verified.
  - Status: `[KERNEL] Phase 19 Milestone achieved: Canvas & Horizon (The Kinetix Desktop Environment) operational.`

## Phase 20: Cartridge I — Scribe (Interactive Notes Chamber) — 2026-11-07
- **Objective**: Design, implement, and verify the first native graphical productivity cartridge, Scribe Studio, featuring a windowed text editing chamber, multi-line document buffer, live typing engine, backspace deletion, word wrapping, radiant cyan cursor rendering, and window lifecycle management.
- **Architectural Implementation**:
  - `cartridges/scribe.rs`: Implemented `ScribeChamber` within a 640×480 `FlowChamber` window positioned at $(180, 100)$. Created dynamic `Vec<char>` document buffer with dual-axis cursor tracking (`cursor_col`, `cursor_row`), automatic line wrapping at 72 columns, line height spacing of 18 pixels, and margin gutter guide.
  - Implemented text editing primitives: `insert_char` (with control character filtering), `insert_text`, `delete_backspace` (with multi-line wrap reversal), and live bottom document telemetry status bar (`Ln X, Col Y | Chars: Z | UTF-8`).
  - `canvas/mod.rs`: Added `compose_desktop()` to composite open cartridge windows alongside the procedural backdrop, horizon dock, and desktop beacons.
  - `main.rs`: Integrated `cartridges::scribe::run_scribe_tests()`.
- **Empirical Verification (Oracle VirtualBox 7.2)**:
  - Scribe window launch verified at $(180, 100)$ [640×480].
  - Multi-line text insertion verified: 3 lines (107 characters) inserted with newline wrapping and correct cursor line indexing.
  - Backspace deletion verified: deleted character and successfully replaced with `'!'`.
  - Close button `[X]` hit-testing verified: window closed cleanly and re-opened for active desktop workspace.
  - Status: `[KERNEL] Phase 20 Milestone achieved: Cartridge I (Scribe Notes Chamber) operational.`

## Phase 21: Cartridge II — Ouroboros (Interactive Snake Arena) — 2026-11-07
- **Objective**: Design, implement, and verify the native arcade gaming cartridge, Ouroboros Arena, featuring an arcade window chamber, 20×20 playing grid, serpentine body vector physics, directional control with 180° self-reversal protection, pseudo-random food spawning, score tracking, wall/self collision detection, game over modal overlay, and match reset.
- **Architectural Implementation**:
  - `cartridges/ouroboros.rs`: Implemented `OuroborosArena` within a 424×460 `FlowChamber` window positioned at $(300, 120)$. Created 20×20 grid (each cell 19×19 with 1px padding), serpentine body vector with lime head (`#4ADE80`) and emerald segments (`#22C55E`), ruby food orb (`#EF4444`), and LCG pseudo-random spawner.
  - Implemented game dynamics: `change_direction` (with opposite-direction filtering), `tick` (wall/self collision testing, body extension, food consumption, tail trimming), `render` (grid, snake, food, status bar, and Game Over modal overlay), and `reset`.
  - `canvas/mod.rs`: Updated `compose_desktop()` to composite the Ouroboros Arena onto the physical display surface.
  - `main.rs`: Integrated `cartridges::ouroboros::run_ouroboros_tests()`.
- **Empirical Verification (Oracle VirtualBox 7.2)**:
  - Ouroboros arena launch verified at $(300, 120)$ [424×460].
  - Initial snake state verified: 3 segments centered at $(10, 10)$, food placed at $(19, 8)$.
  - Movement physics verified: stepped Up and Left cleanly.
  - 180-degree turn rejection verified: instant Right turn while moving Left rejected.
  - Food consumption verified: score incremented by 10 points and snake length expanded to 4.
  - Wall collision verified: running into boundary triggered `GameState::GameOver`.
  - Match reset verified: restored state to `Running` with clean score and initial 3 segments.
  - Status: `[KERNEL] Phase 21 Milestone achieved: Cartridge II (Ouroboros Snake Arena) operational.`
