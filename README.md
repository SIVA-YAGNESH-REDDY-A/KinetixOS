# Kinetix OS

> **Kinetix OS** — An Independent, 64-bit Preemptive Operating System written in Rust for x86_64 UEFI Hardware.

================================================
                    KINETIX OS
================================================

           Independent Rust Operating System

Architecture       : x86_64
Boot Firmware      : UEFI
Kernel             : Kinetix Kernel
Kernel Language    : Rust
Bootloader         : Kinetix Bootloader

================================================

## 1. Project Identity and Principles

* **Operating System**: Kinetix OS
* **Kernel**: Kinetix Kernel (`no_std`, freestanding)
* **Bootloader**: Kinetix Bootloader (UEFI PE32+ application)
* **Application Interface**: Kinetix System Interface
* **Application Runtime**: Kinetix Runtime
* **Native Applications**: Kinetix Applications

### Complete Runtime Independence
Kinetix OS is engineered to boot and operate independently of whatever operating system is installed on the host computer. It does not run on top of Windows, Linux, POSIX, or any third-party kernel.

## 2. Workspace Structure

```
kinetix_os/
|-- Cargo.toml                   # Root Cargo workspace manifest
|-- rust-toolchain.toml          # Pinned stable toolchain and components
|-- .cargo/config.toml           # Platform and target build configuration
|-- README.md                    # Project overview & architectural index
|-- CHANGELOG.md                 # Version and milestone release notes
|-- CONTRIBUTING.md              # Engineering guidelines & conventions
|
|-- bootloader/
|   |-- kinetix-bootloader/      # Pure Rust UEFI bootloader (BOOTX64.EFI)
|
|-- kernel/
|   |-- kinetix-kernel/          # Freestanding Rust kernel (x86_64-unknown-none)
|
|-- crates/
|   |-- kinetix-boot-protocol/   # Zero-dependency shared BootInfo ABI contract
|   |-- kinetix-runtime/         # User-space runtime library for native apps
|
|-- apps/
|   |-- hello-world/             # Native demonstration application
|
|-- tools/
|   |-- kinetix-image-builder/   # Host disk image assembler (GPT + FAT32 ESP)
|
|-- scripts/
|   |-- windows/                 # PowerShell automation pipeline
|
|-- docs/
|   |-- architecture/            # Architectural specifications & ADRs
|   |-- development/             # Environment setup, audit & development logs
|   |-- testing/                 # Test results & VirtualBox instructions
```

## 3. Quick Start (Windows Host)

### Prerequisites
* Rust Stable (`1.99.1`+) with targets `x86_64-unknown-uefi` and `x86_64-unknown-none`.
* Oracle VirtualBox 7.x with UEFI support.

### Building
```powershell
# Build the UEFI bootloader
cargo build -p kinetix-bootloader --target x86_64-unknown-uefi

# Build the kernel
cargo build -p kinetix-kernel --target x86_64-unknown-none
```
