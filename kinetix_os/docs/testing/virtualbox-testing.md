# Kinetix OS — VirtualBox Testing & Automation Guide

## 1. Overview
Kinetix OS is tested in Oracle VirtualBox configured with 64-bit UEFI firmware (`efi64`), AHCI SATA controller, and COM1 serial port output redirected to a file for deterministic verification.

---

## 2. Virtual Machine Baseline Configuration

| Parameter | Value | Reason |
|---|---|---|
| VM Name | `KinetixOS` | Identifiable project VM |
| Operating System Type | `Other_64` | Freestanding x86_64 OS |
| Firmware | `EFI 64-bit` (`--firmware efi64`) | UEFI specification compliance |
| Virtual RAM | 2048 MiB | Ample RAM for frame allocation and testing |
| Virtual CPUs | 2 vCPUs | Baseline for task scheduling & timer interrupts |
| Storage Controller | Intel AHCI SATA | Native block storage |
| Serial Port 1 (COM1) | I/O: `0x3F8`, IRQ: 4, Mode: `File` | Captures headless serial diagnostic logs |
| Video | VBoxSVGA / Standard Display | Compatible with UEFI GOP |

---

## 3. Automation Workflow

The build and test pipeline is fully automated via PowerShell scripts under `scripts/windows/`:

1. **Verify Environment**:
   ```powershell
   .\scripts\windows\check-environment.ps1
   ```
2. **Compile Bootloader and Kernel**:
   ```powershell
   .\scripts\windows\build-os.ps1
   ```
3. **Assemble UEFI Boot Image**:
   ```powershell
   .\scripts\windows\build-image.ps1
   ```
4. **Boot in VirtualBox**:
   ```powershell
   .\scripts\windows\run-virtualbox.ps1
   ```
