# ADR-002: Implementation of a Custom Rust UEFI Bootloader

## Status
**Accepted**

## Context
An operating system requires a mechanism to load its kernel executable from disk into RAM, initialize hardware interfaces, query the system memory map, and transfer execution control.

Our earlier implementation relied on GRUB2 and legacy BIOS Multiboot headers with `grub-mkrescue` from a Linux/WSL environment. This created heavy external dependencies (Linux packages, xorriso, mtools) and prevented native Windows builds, while obscuring low-level firmware interactions.

## Choices Considered
1. **Option A: Third-Party Bootloader (GRUB2 / Limine / Syslinux)**
   * *Advantages*: Handles disk formats, partitions, and protected-mode transitions automatically.
   * *Disadvantages*: Introduces non-Rust external dependencies; cannot be natively compiled on Windows without WSL; deprives the project of owning its boot protocol and early hardware initialization.
2. **Option B: Ready-Made Rust `bootloader` Crate**
   * *Advantages*: Pure Rust bootloader maintained by the community.
   * *Disadvantages*: Imposes an external memory layout and configuration format; acts as a "black box" boot framework rather than an educational and architectural component of Kinetix OS; frequently requires specific nightly compiler features.
3. **Option C: Custom Rust UEFI Bootloader (`kinetix-bootloader`)**
   * *Advantages*:
     * Pure Rust implementation using `x86_64-unknown-uefi` and the safe `uefi` crate abstractions.
     * Full ownership over the boot protocol, module loading mechanism, framebuffer selection, and memory map interpretation.
     * Compiles directly on Windows via `cargo build --target x86_64-unknown-uefi`.
     * Clean, verified handoff via `ExitBootServices`.
   * *Disadvantages*: Requires manual implementation of ELF64 loading, GOP querying, and memory-map translation.

## Decision
We select **Option C**: Design and implement `kinetix-bootloader` as our own custom UEFI application in Rust.

## Consequences
* Kinetix OS maintains 100% architectural independence from third-party boot managers.
* The build pipeline remains 100% Windows-native without WSL or Linux dependencies.
* The bootloader is maintained directly in the Cargo workspace as `bootloader/kinetix-bootloader`.
