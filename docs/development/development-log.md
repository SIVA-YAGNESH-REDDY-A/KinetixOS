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
