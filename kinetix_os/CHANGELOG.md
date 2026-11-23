# Changelog

All notable changes to the Kinetix OS project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Phase 0**: Host development environment audit and dependency verification.
  - Installed and verified Rust `1.99.1` stable toolchain to `D:\Rust`.
  - Installed targets `x86_64-unknown-uefi` and `x86_64-unknown-none`.
  - Validated bundled `rust-lld` linker and `llvm-objdump` tool.
  - Completed software dependency audit (`docs/development/dependency-audit.md`).
- **Phase 1**: Architecture and technical design.
  - System architecture specification (`docs/architecture/system-architecture.md`).
  - Boot flow specification (`docs/architecture/boot-flow.md`).
  - Runtime independence specification (`docs/architecture/runtime-independence.md`).
  - Native application architecture specification (`docs/architecture/application-architecture.md`).
  - Architecture Decision Records ADR-001 through ADR-005.
- **Phase 2**: Project workspace initialization.
  - Modular Cargo workspace with pinned `rust-toolchain.toml`.
  - Architecture-specific target configuration in `.cargo/config.toml`.
