# ADR-003: Definition of the Kinetix Boot Protocol

## Status
**Accepted**

## Context
When the UEFI bootloader terminates firmware boot services via `ExitBootServices`, it must pass essential environmental metadata to the kernel entry point. This includes:
1. Physical memory layout (usable RAM, reserved regions, ACPI reclaimable memory).
2. Framebuffer configuration (physical address, resolution, stride, format).
3. Staged application binary modules in RAM.
4. Firmware configuration table pointers (e.g., ACPI RSDP for LAPIC and multicore discovery).

Because the bootloader and kernel are compiled as independent binaries with different targets (`x86_64-unknown-uefi` vs `x86_64-unknown-none`), both need a strictly typed, shared data structure with a stable C-compatible memory layout (`#[repr(C)]`).

## Choices Considered
1. **Option A: Multiboot2 Specification**
   * *Advantages*: Standardized specification with tag-based structure.
   * *Disadvantages*: Complex tag parsing required in kernel; designed originally around 32-bit legacy BIOS abstractions; clumsy representation of UEFI 64-bit GOP framebuffers.
2. **Option B: Proprietary Shared Crate (`kinetix-boot-protocol`)**
   * *Advantages*:
     * Pure `#![no_std]` Rust library with `#[repr(C)]` structs.
     * Zero runtime dependencies.
     * Contains only immutable, verified types (`BootInfo`, `MemoryMap`, `MemoryRegion`, `FramebufferInfo`, `ModuleDescriptor`).
     * Compiles under both `x86_64-unknown-uefi` and `x86_64-unknown-none`.
   * *Disadvantages*: Requires maintaining the shared crate within the project repository.

## Decision
We select **Option B**: Implement `crates/kinetix-boot-protocol` as a lightweight, shared `#![no_std]` crate containing the definitive `BootInfo` structure.

## Consequences
* Type-safe, verified handoff between bootloader and kernel.
* Any changes to boot information trigger compiler checks in both the bootloader and kernel, preventing ABI synchronization bugs.
