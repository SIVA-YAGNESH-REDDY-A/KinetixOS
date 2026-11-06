# Kinetix OS — Runtime Independence Specification

## 1. Core Principle

**Kinetix OS is an autonomous operating system.** It does not run inside a host operating system, does not use host APIs, and does not require an operating system to be pre-installed on the host computer.

When booted from a USB drive on a computer containing Windows, Linux, macOS, or blank storage, Kinetix OS runs completely independently of that storage and firmware after the UEFI boot handoff.

---

## 2. Distinction Between Development and Runtime

| Category | Development Environment (Dev Laptop) | Kinetix OS Runtime Environment |
|---|---|---|
| **Role** | Compiling, assembling, and packaging | Executing kernel, drivers, scheduler, and apps |
| **Operating System** | Windows 11 25H2 | **Kinetix OS (Self-contained)** |
| **Languages & Tools** | Rustc, Cargo, PowerShell, Git, Python | **Rust (`core` + `no_std`), minimal x86_64 ASM** |
| **Standard Library** | Rust `std` (host tools only) | **Rust `core` (Kernel/Apps) + `alloc` (Heap)** |
| **System Calls** | Win32 API (`ntdll.dll`, `kernel32.dll`) | **Native Kinetix Syscall ABI (`syscall`/`sysret`)** |
| **Binary Formats** | Windows `.exe` / `.dll` | **PE32+ (`BOOTX64.EFI`) + ELF64 (`.elf`)** |
| **Hardware Access** | Mediated by Windows NT Kernel | **Direct MMIO, Port I/O, Interrupts, Model-Specific Registers (MSRs)** |

---

## 3. Strict Runtime Prohibitions in Kinetix OS

To guarantee absolute runtime independence, the following dependencies are strictly prohibited from the kernel, bootloader, and native applications:

1. **No Host OS System Calls**: No calls to Windows NT syscalls (`0x2E` / `syscall`), Linux syscalls (`syscall` vector `0x80`), or macOS Mach traps.
2. **No Host Shared Libraries**: No linking against `msvcrt.dll`, `kernel32.dll`, `libc.so`, `glibc`, `musl`, or any external dynamic linkers.
3. **No External Virtual Machines / Interpreters**: The kernel and native applications must compile to raw machine code. No Python, Java, or .NET runtimes are permitted at runtime.
4. **No UEFI Dependency Post-Handoff**: While UEFI firmware is used as the standardized hardware initialization vehicle, all UEFI Boot Services are terminated via `ExitBootServices()` before the kernel starts. The kernel owns its own memory allocation, paging, interrupt handling, and drivers.

---

## 4. Architectural Enforcement Mechanisms

### 4.1 Compiler Enforcement (`#![no_std]`)
Every crate intended to execute on the bare-metal target is marked `#![no_std]` at the root of the crate. Attempting to use any type or function from the Rust standard library (`std`) triggers a compile-time failure.

### 4.2 Target Tuple Enforcement
* **Bootloader**: Compiled explicitly for target `x86_64-unknown-uefi`. The compiler emits PE/COFF binaries that strictly adhere to the UEFI specification.
* **Kernel**: Compiled explicitly for target `x86_64-unknown-none`. This target:
  - Disables the x86-64 red zone (a 128-byte region below the stack pointer assumed free by System V ABI, which would be corrupted by CPU interrupt handlers).
  - Emits freestanding ELF64 binaries without dynamic linking or OS-specific ABI tags.
  - Excludes floating-point / SIMD vector registers by default to avoid corrupting register state in interrupt routines before SSE is initialized.

### 4.3 Linker Script Isolation
The kernel executable is linked using a custom linker script (`linker.ld`) that assigns explicitly verified physical and virtual address layouts, enforcing section alignment to 4 KiB page boundaries and strictly separating code (`.text`), read-only data (`.rodata`), initialized data (`.data`), and zero-initialized memory (`.bss`).
