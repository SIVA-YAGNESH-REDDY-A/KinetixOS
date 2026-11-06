# Kinetix OS — Dependency Audit

> Status: **Phase 0 — COMPLETE.** Toolchain, targets, linkers, smoke compilation, and
> upstream crate metadata are verified directly on the host machine.
> Last updated: 2026-11-04

## 1. Principles

1. **Runtime vs. build-time are separate columns.** A tool can be required to *build*
   Kinetix OS and still be forbidden at *runtime*.
2. Kernel, scheduler, memory manager, loader and syscall layer stay under project ownership.
   Crates are only accepted for thin, well-understood abstractions.
3. Stable Rust only. Any nightly requirement is a rejection reason unless separately justified.
4. Every OS-side crate must be `no_std` and must not pull `std` via default features.

## 2. Development-host software

| Dependency | Purpose | Version | Source | Installed? | Runtime dependency of Kinetix OS? | License | Decision |
|---|---|---|---|---|---|---|---|
| rustup | Toolchain manager | 1.29.3 | D:\Rust\rustup | **Yes** | No | MIT/Apache-2.0 | **Verified** |
| Rust stable (rustc, cargo) | Compiler / build system | 1.99.1 (commit 58cf8f9c3 2026-10-13) | D:\Rust\cargo | **Yes** | No | MIT/Apache-2.0 | **Verified**, pinned via `rust-toolchain.toml` |
| rustfmt, clippy | Formatting / lints | 1.10.0-stable / 0.1.99 | D:\Rust\cargo | **Yes** | No | MIT/Apache-2.0 | **Verified** |
| `rust-lld` | Linker for UEFI (PE/COFF, lld-link flavour) and kernel/apps (ELF, ld.lld flavour) | bundled (LLVM 23.1.1) | rustup sysroot | **Yes** | No | Apache-2.0 w/ LLVM exc. | **Verified** (smoke test confirmed) |
| `llvm-tools` component | `llvm-objdump`, `llvm-readobj`, `llvm-nm` for inspecting EFI/ELF output | bundled | rustup sysroot | **Yes** | No | Apache-2.0 w/ LLVM exc. | **Verified** |
| Host linker | Links build scripts, proc-macros, host tools | self-contained MinGW (`gcc-ld\ld.lld`, `ld.exe`) | bundled `rust-mingw` | **Yes** | No | GPL/LGPL/LLVM | **Verified** (`stable-x86_64-pc-windows-gnu`) |
| Oracle VirtualBox | UEFI test VM, serial capture, `convertfromraw` | 7.2.22 | D:\VirtualBox-7.2.22-Win.exe | Installer ready (178MB, Authenticode verified) | No | GPLv3 (base) | **Required** (requires user UAC run) |
| VirtualBox Extension Pack | USB2/3 passthrough, RDP | — | Oracle | No | No | PUEL (restrictive) | **Not required** — rejected |
| Git | Version control | 2.55.0.windows.4 | D:\Git\Git | **Yes** | No | GPLv2 | **Verified** |
| Python | Research charts/statistics only | 3.13.14 | D:\Python | **Yes** | **No — must never be** | PSF | Optional (Phase 12) |
| Standalone LLVM/Clang | — | — | — | No | — | — | **Rejected**: `rust-lld` + `llvm-tools` suffice |
| NASM | Assembly | — | — | No | — | — | **Rejected**: Rust `global_asm!`/`asm!` is stable and sufficient |
| GRUB / grub-mkrescue | Bootloader | — | — | No | — | — | **Rejected**: replaces our own bootloader |
| xorriso / ISO | ISO images | — | — | No | — | — | **Rejected**: GPT raw image + VDI is simpler; USB needs a disk image, not ISO |
| mtools / mkfs.fat / WSL | FAT creation | — | — | — | — | — | **Rejected**: host tool in Rust covers it; avoids Linux build dependency |
| QEMU (incl. via other tools, e.g. `uefi-run`, `bootimage` runners) | Emulation | — | — | No | — | — | **Prohibited** by project policy |
| `bootloader` / `bootimage` crates | Complete boot framework | — | — | — | — | — | **Rejected**: would replace the Kinetix Bootloader architecture |

## 3. Rust targets

| Target | Used for | Tier (stable) | Output | Linker | Status / Evidence |
|---|---|---|---|---|---|
| `x86_64-unknown-uefi` | Kinetix Bootloader | Tier 2, `core`+`alloc` available | PE32+ EFI application (`coff-x86-64`) | `rust-lld` | **Verified.** Compiled test crate; `llvm-objdump -f` confirmed `file format coff-x86-64` |
| `x86_64-unknown-none` | Kinetix Kernel | Tier 2, `core`+`alloc` | ELF64 (`elf64-x86-64`) | `rust-lld` | **Verified.** Compiled test crate; `llvm-objdump -f` confirmed `file format elf64-x86-64` |
| `x86_64-unknown-none` | Native applications (initial) | as above | ELF64 | `rust-lld` | **Provisionally selected**; separate linker script with user-space base |
| Custom JSON target | — | requires nightly `-Zbuild-std` | — | — | **Rejected** (stable-only policy) |

## 4. Rust crates — candidates (verified via crates.io)

Columns: *Side* = where it runs (Boot = UEFI bootloader, Kernel, App, Host = dev tool only).

| Dependency | Side | Purpose | Version | MSRV | no_std | Host dependency at runtime | License | Decision |
|---|---|---|---|---|---|---|---|---|
| `uefi` | Boot | Safe bindings to UEFI boot services: GOP, Simple File System, memory map, `exit_boot_services` | 0.41.0 | 1.91 | Yes | None (firmware only, unusable after boot-services exit) | MIT OR Apache-2.0 | **Accepted (bootloader only).** Default features: `[]`. |
| `uefi-raw` | Boot | Raw ABI types (transitive of `uefi`) | 0.41.0 | 1.91 | Yes | None | MIT OR Apache-2.0 | Transitive |
| `x86_64` | Kernel | Typed wrappers: page tables, GDT/IDT/TSS structs, control registers, port I/O | 0.16.0-rc.0 (or 0.15.5) | 1.98 | Yes | None | MIT/Apache-2.0 | **Accepted with `features = ["instructions"]`** (default in 0.16; stable). |
| `spin` | Kernel | Spinlock `Mutex`, `Once` | 0.12.3 | 1.71 | Yes | None | MIT | **Accepted, minimal features** (`mutex`, `spin_mutex`, `once`). Interrupt-safety wrappers written by us. |
| `fatfs` | Host (image builder) | Create/populate FAT32 ESP | 0.3.6 | — | has std+no_std | Host-only | MIT | **Accepted for dev host tool.** Later kernel gets native reader (Phase 13). |
| `gpt` | Host (image builder) | Write protective MBR + GPT | 4.1.0 | 1.65 | std | Host-only | MIT | **Accepted for dev host tool.** |
| `volatile` | Kernel | Volatile MMIO wrappers | — | — | Yes | None | MIT/Apache-2.0 | **Rejected.** `core::ptr::{read,write}_volatile` is standard and explicit. |
| `lazy_static` | Kernel | Lazy globals | — | — | with feature | None | MIT/Apache-2.0 | **Rejected.** `spin::Once` / `core::cell::OnceCell` replaces it. |
| `linked_list_allocator` | Kernel | Heap allocator | — | — | Yes | None | MIT/Apache-2.0 | **Rejected for initial kernel.** Own buddy/free-list allocator first (Phase 7). |
| `xmas-elf` / `goblin` / `elf` | Boot/Kernel | ELF64 parsing | — | — | varies | None | varies | **Rejected.** Minimal, strictly validated ELF64 parser is written by us in `crates/kinetix-boot-protocol`. |

## 5. Runtime-independence check

| Component | Allowed runtime dependencies | Forbidden |
|---|---|---|
| Kinetix Bootloader | UEFI firmware boot/runtime services (until `ExitBootServices`), `core`, `alloc` (UEFI pool allocator) | `std`, any OS API |
| Kinetix Kernel | Hardware, data passed in the Kinetix boot protocol, `core`, `alloc` (own heap) | `std`, UEFI boot services after handoff, libc, any host OS |
| Kinetix Applications | Kinetix syscall ABI, `kinetix-runtime`, `core` | `std`, libc (glibc/musl), Windows/Linux syscalls |
| Host tools (image builder, scripts) | Windows, `std`, PowerShell, Python | — (never copied into the image as runtime components) |

## 6. Verification checklist & outcomes

- [x] Exact stable version installed: `1.99.1 (58cf8f9c3 2026-10-13)`.
- [x] Targets installed and verified: `x86_64-pc-windows-gnu`, `x86_64-unknown-uefi`, `x86_64-unknown-none`.
- [x] `cargo info` queried for `uefi`, `x86_64`, `spin`, `fatfs`, `gpt` → exact versions, licenses, and MSRVs recorded.
- [x] Smoke builds for `x86_64-unknown-uefi` (PE32+) and `x86_64-unknown-none` (ELF64) compiled and linked cleanly using bundled `rust-lld`.
- [x] Object inspection verified using bundled `llvm-objdump`.
- [x] Oracle VirtualBox 7.2.22 official installer downloaded to `D:\VirtualBox-7.2.22-Win.exe` with verified Authenticode signature from Oracle America, Inc.
- [ ] User runs `D:\VirtualBox-7.2.22-Win.exe` (requires UAC prompt on interactive desktop to install VirtualBox kernel drivers).
