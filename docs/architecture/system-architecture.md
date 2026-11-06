# Kinetix OS — System Architecture Specification

## 1. Executive Summary

**Kinetix OS** is an independent, 64-bit (`x86_64`), preemptively multitasked operating system implemented primarily in Rust (`no_std`). It boots via modern UEFI firmware on standard x86-64 hardware, initialises its own hardware abstraction, memory management, and interrupt subsystem, executes a custom CPU task scheduler for multitasking research, and provides an isolated execution environment for native, separately compiled applications.

At no point during or after the boot process does Kinetix OS rely on Windows, Linux, POSIX runtimes, or hypervisor host services.

---

## 2. High-Level Subsystem Architecture

The following block diagram depicts the structural layout of Kinetix OS:

```
+--------------------------------------------------------------------------+
|                          Kinetix Applications                            |
|        (Separately compiled ELF64 binaries linked against kinetix-rt)    |
+--------------------------------------------------------------------------+
                                    |  Kinetix System Call ABI (Syscall / Sysret)
+--------------------------------------------------------------------------+
|                            Kinetix Kernel                                |
|  +--------------------------------------------------------------------+  |
|  | Task Management & CPU Scheduler (FCFS, Round-Robin, Priority)     |  |
|  +--------------------------------------------------------------------+  |
|  | Memory Management (Frame Allocator, Paging, Kernel Buddy Heap)     |  |
|  +--------------------------------------------------------------------+  |
|  | Interrupt & Exception Subsystem (IDT, LAPIC, PIT, ISRs)            |  |
|  +--------------------------------------------------------------------+  |
|  | Hardware Abstraction Layer (Serial 16550, GOP Framebuffer, PCI)   |  |
|  +--------------------------------------------------------------------+  |
|  | Storage & Filesystem (Block Device Interface, FAT32 Reader)        |  |
+--------------------------------------------------------------------------+
                                    ^  Kinetix Boot Protocol (BootInfo handoff)
+--------------------------------------------------------------------------+
|                          Kinetix Bootloader                              |
|   (x86_64-unknown-uefi PE32+ application: BOOTX64.EFI)                  |
|   - Locates kernel & application modules on FAT32 ESP volume             |
|   - Sets up GOP framebuffer display                                      |
|   - Retrieves UEFI memory map                                            |
|   - Calls ExitBootServices() and jumps to Kernel Entry Point             |
+--------------------------------------------------------------------------+
                                    ^  UEFI Specification v2.x
+--------------------------------------------------------------------------+
|                          Host Hardware / Firmware                        |
|   (x86_64 CPU, UEFI Firmware, RAM, LAPIC/IOAPIC, NVMe/SATA/USB Disk)     |
+--------------------------------------------------------------------------+
```

---

## 3. Subsystem Breakdown and Responsibilities

### 3.1 Kinetix Bootloader (`kinetix-bootloader`)
* **Format**: 64-bit PE/COFF executable (`BOOTX64.EFI`) targetting `x86_64-unknown-uefi`.
* **Placement**: `/EFI/BOOT/BOOTX64.EFI` on a FAT32 EFI System Partition (ESP).
* **Core Responsibilities**:
  1. Boot from standard UEFI firmware (version 2.4+).
  2. Locate the boot volume using `EFI_SIMPLE_FILE_SYSTEM_PROTOCOL`.
  3. Load `/KINETIX/KERNEL/KINETIX-KERNEL.ELF` into reserved physical memory.
  4. Parse the ELF64 headers to locate program headers (`PT_LOAD`) and the kernel entry point.
  5. Locate optional pre-loaded application binaries in `/KINETIX/APPS/`.
  6. Query the Graphics Output Protocol (GOP) to establish a linear physical framebuffer (resolution, pitch, pixel format).
  7. Retrieve the definitive firmware memory map via `GetMemoryMap`.
  8. Terminate firmware execution via `ExitBootServices`.
  9. Populate the `BootInfo` struct and jump into the 64-bit kernel entry point with the System V AMD64 ABI.

### 3.2 Kinetix Boot Protocol (`kinetix-boot-protocol`)
A dedicated, `no_std`, zero-dependency shared crate that defines the contract between the bootloader and the kernel:
* Memory map descriptor array and physical memory region classifications.
* Linear physical framebuffer metadata (base address, width, height, stride, pixel format).
* Kernel virtual and physical address ranges.
* Initial loaded module descriptors (for Stage A boot-time application loading).
* Hardware table pointers (ACPI RSDP physical address).

### 3.3 Hardware Abstraction Layer (`kinetix-kernel::arch`)
* **CPU Initialization**: GDT (Global Descriptor Table) setup, TSS (Task State Segment) with Interrupt Stack Table (IST) for double fault and page fault isolation.
* **Interrupt Descriptor Table (IDT)**: 256 vector handlers catching CPU exceptions (0–31) and hardware IRQs (32–255).
* **Timers**: Programmable Interval Timer (PIT) initially, followed by the Local APIC timer for high-resolution, timer-driven preemptive scheduling.
* **Diagnostics**: 16550 UART Serial driver on COM1 (`0x3F8`) for instant, headless debugging and VirtualBox log capture.
* **Display**: High-performance linear framebuffer console rendering bitmap glyphs directly to the display without BIOS interrupts.

### 3.4 Memory Management Subsystem (`kinetix-kernel::memory`)
* **Physical Frame Allocator**: Bitmapped frame allocator managing 4 KiB physical memory pages (`frames`), tracking free, reserved, and kernel-occupied physical RAM.
* **Paging & Virtual Memory**: 4-level paging (`PML4` -> `PDPT` -> `PD` -> `PT`). Maps the kernel at higher-half addresses (`0xFFFF_8000_0000_0000` or higher-half direct physical map), leaving lower-half addresses (`0x0000_0000_0000_0000` to `0x0000_7FFF_FFFF_FFFF`) for user processes.
* **Kernel Heap**: Dynamic allocator implementing free-list / buddy allocation backed by the physical frame allocator for `alloc` crate data structures (`Vec`, `Box`, `String`).

### 3.5 Task Management and CPU Scheduler (`kinetix-kernel::scheduler`)
* **Task Control Block (TCB)**: Encapsulates task ID, CPU register state (RSP, RIP, RFLAGS, callee-saved registers), execution state (`Ready`, `Running`, `Blocked`, `Terminated`), page table root (`CR3`), stack pointer, and execution statistics.
* **Context Switching**: Architecture-specific assembly routine (`switch_context`) saving and restoring register state across task switches.
* **Multitasking Research Engine**:
  * Scheduling Policies: First-Come-First-Served (FCFS), preemptive Round-Robin (RR) with configurable time slices, and priority-based scheduling.
  * Real-time kernel metric collection: CPU cycles consumed, context switch latency, waiting time, turnaround time, throughput.

### 3.6 Application Execution Subsystem (`kinetix-kernel::process`)
* **Executable Loading**: In-kernel ELF64 parser validating headers, mapping `PT_LOAD` segments into user virtual address space, allocating user stack, and setting up initial register arguments.
* **Privilege Separation**: Transitions from Ring 0 (Kernel) to Ring 3 (User) via `sysretq` or `iretq`.
* **System Call Interface**: Modern `syscall` / `sysret` instruction handler with atomic stack switching via `MSR_LSTAR` and `MSR_KERNEL_GS_BASE`.
* **Isolation**: Application page faults and general protection faults are caught and handled by the kernel, safely terminating the offending task without compromising kernel stability.

---

## 4. Hardware Baseline and Constraints

| Parameter | Specification |
|---|---|
| Target Architecture | `x86_64` (AMD64 / Intel 64) |
| Minimum RAM | 512 MiB (recommended 2 GiB for VirtualBox testing) |
| Boot Firmware | UEFI 2.4+ (64-bit) |
| Display | UEFI Graphics Output Protocol (GOP) linear framebuffer |
| Serial Diagnostics | Standard 16550 UART (COM1: `0x3F8`, 115200 8N1) |
| Timer Hardware | 8254 PIT / Local APIC Timer |
| Target Physical Media | 4 GB FAT32 USB Pendrive (MBR/GPT hybrid or pure GPT) |
