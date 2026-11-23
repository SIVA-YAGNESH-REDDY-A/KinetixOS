# Contributing to Kinetix OS

## 1. Engineering Principles
* **Safety First**: Safe Rust is strongly preferred. Every `unsafe` block must be accompanied by a `// SAFETY:` explanatory comment detailing the invariants preserved.
* **No Host Runtime Dependencies**: Code executing on bare metal (`bootloader`, `kernel`, `apps`) must be marked `#![no_std]` and must never depend on the standard library or host operating system APIs.
* **Deterministic Builds**: Use pinned compiler versions, explicit targets, and checked-in `Cargo.lock` files.
* **Single Responsibility**: Keep kernel modules small, modular, and focused. Do not scatter hardware-specific logic across unrelated subsystems.

## 2. Naming Conventions
* **Crates / Packages**: Lowercase kebab-case prefixed with `kinetix-` (e.g., `kinetix-bootloader`, `kinetix-kernel`).
* **Rust Modules & Files**: Snake_case (e.g., `frame_allocator.rs`, `context_switch.s`).
* **Types / Traits / Structs**: PascalCase (e.g., `BootInfo`, `TaskControlBlock`).
* **Constants & Statics**: SCREAMING_SNAKE_CASE (e.g., `KERNEL_VIRTUAL_BASE`, `PAGE_SIZE`).
* **Documentation**: Lowercase kebab-case markdown files.

## 3. Testing Policy
Every implementation phase must have clear, verifiable evidence recorded in `docs/testing/test-results.md` before proceeding to the next milestone.
