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
