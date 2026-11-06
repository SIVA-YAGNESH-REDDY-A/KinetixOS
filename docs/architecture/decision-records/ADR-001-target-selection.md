# ADR-001: Selection of Rust Compilation Targets

## Status
**Accepted**

## Context
Kinetix OS consists of multiple distinct software components:
1. A UEFI firmware application running in 64-bit Long Mode before OS handoff.
2. A freestanding operating system kernel running directly on the CPU with custom memory mapping and interrupt handling.
3. Native user-mode applications running on top of Kinetix OS.
4. Host-side build tools running on Windows during the image creation pipeline.

Each environment has distinct requirements regarding the target binary format, ABI calling conventions, standard library availability, and CPU register usage.

## Choices Considered
1. **Option A: Custom JSON Target Specifications (`x86_64-kinetix-bootloader.json`, `x86_64-kinetix-kernel.json`)**
   * *Advantages*: Fine-grained control over every compiler flag, CPU feature, and ABI detail.
   * *Disadvantages*: Requires nightly Rust toolchain with `-Zbuild-std=core,alloc`. In our previous OS attempt, this dependency caused continuous breakages and build instability whenever nightly updated.
2. **Option B: Standard Tier 2 Targets (`x86_64-unknown-uefi` and `x86_64-unknown-none`)**
   * *Advantages*:
     * Officially supported on the **Stable Rust toolchain** (`rustc 1.99.1`).
     * `x86_64-unknown-uefi` automatically generates PE32+ binaries with the Microsoft x64 calling convention and standard subsystem required by UEFI firmware.
     * `x86_64-unknown-none` generates freestanding 64-bit ELF binaries with red-zone disabled and soft-float defaults, perfectly suited for bare-metal kernels.
     * Bundled `rust-lld` links both targets seamlessly without needing external MSVC or GCC installations.
   * *Disadvantages*: Requires custom linker scripts to position sections at specific addresses for the kernel.

## Decision
We select **Option B**:
* `x86_64-unknown-uefi` for `kinetix-bootloader`.
* `x86_64-unknown-none` for `kinetix-kernel` and initial native applications.
* `x86_64-pc-windows-gnu` for development host tools (`kinetix-image-builder`).

## Consequences
* The entire project compiles deterministically on the stable Rust compiler.
* No nightly toolchain dependency is introduced.
* The kernel will use an explicit linker script (`linker.ld`) to position the kernel code at higher-half or designated memory regions.
