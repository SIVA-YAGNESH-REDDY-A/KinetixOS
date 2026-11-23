# Kinetix OS — Development Environment Setup

> Status: **Phase 0 — IN PROGRESS.** Inspection complete; installation pending approval.
> Last updated: 2026-11-04

This document records the *actual* state of the development computer (from read-only
inspection), not an assumed one. All values below came from commands run on the host.

## 1. Host system (inspected 2026-11-04)

| Item | Observed value | Source |
|---|---|---|
| Machine | Lenovo LOQ 15IRX9 (model 83DV), BIOS NECN50WW | `HKLM\HARDWARE\DESCRIPTION\System\BIOS` |
| OS | Windows (registry ProductName "Windows 10 Home Single Language", DisplayVersion 25H2, build 26200.9711 → this build number is the Windows 11 25H2 line; the ProductName string is a known legacy value) | `HKLM\...\Windows NT\CurrentVersion` |
| Architecture | AMD64 (64-bit OS) | `%PROCESSOR_ARCHITECTURE%`, .NET |
| CPU | 13th Gen Intel Core i5-13450HX, Family 6 Model 183 Stepping 1, 16 logical processors | `HKLM\HARDWARE\...\CentralProcessor\0` |
| RAM | 15.71 GB total, ~7.8 GB available at inspection time | `Microsoft.VisualBasic.Devices.ComputerInfo` |
| C: | NTFS, 224.5 GB, 87.3 GB free | `System.IO.DriveInfo` |
| D: | NTFS, 250 GB, 241.7 GB free (workspace drive) | `System.IO.DriveInfo` |
| Host firmware | UEFI, Secure Boot **enabled** | `%firmware_type%`, `SecureBoot\State` |
| Virtualization-Based Security | **Enabled** (`EnableVirtualizationBasedSecurity=1`, HVCI `Enabled=1`) | `HKLM\SYSTEM\...\DeviceGuard` |
| Shell | Windows PowerShell 5.1.26100.9711 (Desktop) | `$PSVersionTable` |
| Workspace | `D:\kinetix_os` | — |

### Notes
- Windows 11 consumer builds still report `ProductName = Windows 10 ...` in the registry;
  build 26200 identifies the actual release line.
- **VBS/HVCI enabled means the Windows hypervisor is active.** VirtualBox 7.x on such a
  host runs through the Hyper-V "Native Execution Manager" (NEM) backend instead of its own
  VT-x engine. It works, but is slower and can affect timer behaviour. This matters for the
  Phase 12 CPU research (timing measurements). We will **not** disable VBS/HVCI without a
  separate explanation and explicit approval.
- Host Secure Boot state is irrelevant to the VirtualBox guest (VirtualBox EFI Secure Boot is
  a separate per-VM setting). It *is* relevant for physical USB boot (Phase 17).

## 2. Software inventory (inspected 2026-11-04)

| Tool | Status | Version / Path | Required? |
|---|---|---|---|
| rustup | **Missing** | — | Required |
| rustc / cargo | **Missing** | — | Required |
| rustfmt / clippy | **Missing** | — | Required (dev quality) |
| MSVC Build Tools (`link.exe`) | **Missing** (VS Installer 4.10 present, no products installed per `vswhere`) | — | Depends on host-toolchain decision (see §4) |
| LLVM / LLD (standalone) | Missing | — | **Not required** — rustup ships `rust-lld` |
| Oracle VirtualBox / VBoxManage | **Missing** | — | Required |
| Git | Installed | 2.55.0.windows.4, `D:\Git\Git\cmd\git.exe` | Required |
| Python | Installed | 3.13.14, `D:\Python\python.exe` | Optional (research analysis only) |
| winget | Present | `...\WindowsApps\winget.exe` (not runnable inside agent sandbox) | Optional install channel |
| WSL | Present (`wsl.exe`), state not inspectable from agent sandbox | — | **Not required**, not to be used |
| Node.js | Installed | v24.19.0 | Not used |
| xorriso, mtools, mkfs.fat, 7z, dd, nasm | Missing | — | **Not required** (see §5) |
| QEMU | Missing | — | **Prohibited** by project policy |
| Chocolatey / Scoop | Missing | — | Not required |

## 3. Previous attempt (historical record)

> **Correction (2026-11-04 22:06):** the agent's sandboxed shell initially showed the files
> below inside `D:\kinetix_os`, but that view was stale. On the real disk the files had
> already been **deleted by the user at 2026-11-04 16:26** (they are in the D: Recycle Bin,
> together with an earlier set deleted 2026-11-01 09:54 that included a custom
> `x86_64-kinetix.json` target and `rustup-init`). The agent moved/deleted nothing.
> `D:\kinetix_os` now contains only the new `docs/` tree. The analysis is kept as a
> lessons-learned record.
>
> Process lesson: filesystem state that matters (deletions, moves, installed tools) is
> re-verified outside the sandbox before acting on it.

The previous attempt (not a Git repository) contained:

| Item | Observation |
|---|---|
| `Cargo.toml` | single crate `kinetix_os`, deps `volatile 0.2.6`, `spin 0.9.8`, `lazy_static 1.4` |
| `src/boot.s`, `linker.ld`, `boot.o` | NASM **Multiboot** entry, kernel linked at 1 MiB |
| `build.sh`, `iso/boot/grub/grub.cfg`, `kinetix_os.iso` (15 MB) | GRUB `grub-mkrescue` ISO, built from a Linux/WSL shell |
| `src/vga_buffer.rs` | VGA text-mode (0xB8000) output |
| `target/` | old build output |

**Assessment:** this design is incompatible with the Kinetix OS architecture:
it depends on GRUB (a third-party bootloader replacing our own), uses NASM/C-style
assembly toolchain, BIOS-era VGA text mode (unavailable under pure UEFI GOP), and a
Linux/WSL build host. It will **not** be reused as code. Its failure modes (custom JSON
target → nightly + `build-std`; GRUB; WSL-only build script) directly informed the
stable-only, own-bootloader, Windows-native decisions in `dependency-audit.md`.

## 4. Rust host-toolchain decision (pending)

The OS targets (`x86_64-unknown-uefi`, `x86_64-unknown-none`) link with the
`rust-lld` linker bundled by rustup — no MSVC or LLVM install is needed for them.
However, Cargo also compiles **host** code: build scripts, procedural macros
(e.g. `uefi-macros`), and our own host tool `kinetix-image-builder`. That needs a
host linker:

| Option | Extra install | Pros | Cons |
|---|---|---|---|
| `stable-x86_64-pc-windows-msvc` + VS Build Tools (MSVC + Windows SDK) | ~3–6 GB, admin rights | Rust's officially recommended Windows host; best crate compatibility | Large install; contradicts "no large compiler toolchain unless necessary" |
| `stable-x86_64-pc-windows-gnu` | None (self-contained MinGW linker shipped by rustup) | No admin, no extra compiler install, Tier 1 host | Slightly less common on Windows; crates that compile C code via `cc` would need a MinGW GCC (none planned) |

Recommendation recorded in `dependency-audit.md`; final decision awaits user approval.

## 5. Boot-image strategy (proposed)

No ISO, xorriso, mtools, WSL, or `diskpart` is required. Proposed pipeline:

1. `cargo build` → `BOOTX64.EFI` (PE32+) and `kinetix-kernel` (ELF64).
2. Host tool `tools/kinetix-image-builder` (Rust, `std`, development-only) writes a raw
   GPT disk image containing one FAT32 EFI System Partition with the layout from the
   project specification (`EFI/BOOT/BOOTX64.EFI`, `KINETIX/KERNEL/...`, `KINETIX/APPS/...`).
3. `VBoxManage convertfromraw` → `.vdi` attached to the UEFI VM.
4. The same raw `.img` is later the artifact written to the 4 GB USB (Phase 17).

Rationale: one source of truth for image layout, fully reproducible, no admin rights,
never touches physical disks, and written in the project's primary language.

## 6. Installation status & verification

| # | Action | Status | Verification |
|---|---|---|---|
| 1 | Install rustup with `stable-x86_64-pc-windows-gnu` into `D:\Rust` | **Completed** | `rustup 1.29.3`, `rustc 1.99.1`, `cargo 1.99.1` |
| 2 | Add targets `x86_64-unknown-uefi` and `x86_64-unknown-none` | **Completed** | Both installed; smoke builds compiled with `rust-lld` |
| 3 | Add component `llvm-tools` | **Completed** | `llvm-objdump.exe`, `llvm-readobj.exe`, `llvm-nm.exe` present in sysroot |
| 4 | Add components `rustfmt` and `clippy` | **Completed** | `rustfmt 1.10.0-stable`, `clippy 0.1.99` verified |
| 5 | Verify candidate crates on crates.io | **Completed** | `uefi 0.41.0`, `x86_64 0.16.0-rc.0`, `spin 0.12.3`, `fatfs 0.3.6`, `gpt 4.1.0` |
| 6 | Download Oracle VirtualBox 7.2.22 installer | **Completed** | `D:\VirtualBox-7.2.22-Win.exe` (178 MB, Authenticode verified signed by Oracle America, Inc.) |
| 7 | Install VirtualBox 7.2.22 | **Pending User UAC** | Running the installer requires kernel driver installation (`VBoxDrv.sys`). Run `D:\VirtualBox-7.2.22-Win.exe` and accept UAC. |

## 7. Verification log

| Date | Check | Result |
|---|---|---|
| 2026-11-04 | Host OS / CPU / RAM / disks | 13th Gen i5-13450HX (16 threads), 16 GB RAM, 241 GB free on D: |
| 2026-11-04 | Git | 2.55.0.windows.4 at `D:\Git\Git\cmd\git.exe` |
| 2026-11-04 | Rust toolchain & targets | Installed to `D:\Rust`; smoke tests produce valid `coff-x86-64` (UEFI) and `elf64-x86-64` (None) |
| 2026-11-04 | Candidate crates audit | Queried & verified against crates.io index |
| 2026-11-04 | Oracle VirtualBox | Installer downloaded and verified at `D:\VirtualBox-7.2.22-Win.exe`. User to run installer to complete installation. |
