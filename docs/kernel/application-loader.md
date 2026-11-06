# Kinetix OS Application Loader & Process Address Space Architecture

**Document**: Kernel Architecture Specification: Native Application Loader (ELF64)  
**Author**: Kinetix OS Systems Architecture Team  
**Status**: Implemented & Verified in VirtualBox 7.2 (Phase 14)  
**Date**: November 2026  

---

## 1. Executive Summary

Phase 14 transitions Kinetix OS from a monolithic freestanding kernel to an operating system capable of instantiating, relocating, and isolating **native user-space applications** formatted as standard 64-bit ELF (`ELF64`) binaries conforming to the System V AMD64 ABI.

The Native Application Loader subsystem:
1. **Parses & Validates ELF64 Executables**: Decodes ELF file headers, validates machine target (`x86_64`), endianness, and loadable program headers (`PT_LOAD`).
2. **Builds Isolated Process Address Spaces**: Creates independent 4-level page tables (`PML4` -> `PDPT` -> `PD` -> `PT`) ensuring memory isolation between user programs and the kernel.
3. **Preserves Kernel Identity Mappings**: Clones kernel identity-mapped page tables (`PDPT[0]`, covering 0..1 GiB) into the user page table hierarchy so that CPU interrupts, exceptions, and system calls continue to execute without page faults.
4. **Allocates User Stacks & Maps Segments**: Allocates dedicated physical frames, copies code and data, zeroes `.bss` areas, and provisions a 16 KiB user stack at `0x7FFF_F000`.
5. **Provides Automatic Resource Reclamation**: Tracks all allocated physical frames in `ProcessAddressSpace` and automatically frees them back to `FRAME_ALLOCATOR` upon process termination (`Drop`).

---

## 2. Process Memory Layout (x86_64 Virtual Address Space)

```
0x0000_0000_0000_0000 +---------------------------------------------------+
                      | KERNEL IDENTITY MAPPINGS (Inherited from Boot)   |
                      | - 0x0000_0000..0x0020_0000 : BIOS / UEFI Reserved |
                      | - 0x0020_0000..0x0025_0000 : Kinetix Kernel Code  |
                      | - 0x0040_0000..0x0080_0000 : Kernel Heap (4 MiB) |
                      | - Framebuffer & Identity Mapped Physical RAM      |
                      | (PML4[0] -> PDPT[0], 0 to 1 GiB)                  |
0x0000_0000_4000_0000 +---------------------------------------------------+
                      | USER CODE SEGMENT (.text)                         |
                      | - Virtual Address: 0x4000_0000 (1 GiB boundary)   |
                      | - Permissions: USER_ACCESSIBLE | PRESENT (R-X)    |
0x0000_0000_4000_1000 +---------------------------------------------------+
                      | USER DATA & BSS SEGMENTS (.rodata, .data, .bss)   |
                      | - Virtual Address: 0x4000_1000                    |
                      | - Permissions: USER_ACCESSIBLE | PRESENT (R--/RW-)|
                      + - - - - - - - - - - - - - - - - - - - - - - - - - +
                      | (Unmapped Guard / Dynamic Heap Expansion Area)    |
                      + - - - - - - - - - - - - - - - - - - - - - - - - - +
0x0000_0000_7FFF_B000 +---------------------------------------------------+
                      | USER PROCESS STACK (16 KiB = 4 Pages)             |
                      | - Permissions: USER | WRITABLE | PRESENT | NX     |
0x0000_0000_7FFF_F000 +---------------------------------------------------+ (USER_STACK_TOP / RSP)
                      | (PML4[0] -> PDPT[1], 1 GiB to 2 GiB)              |
0x0000_0000_8000_0000 +---------------------------------------------------+
```

---

## 3. Subsystem Architecture

The loader subsystem is organized into three components:

1. **`loader/elf.rs`**:
   - `Elf64Header`: 64-byte primary ELF header.
   - `Elf64ProgramHeader`: 56-byte segment descriptor.
   - `ElfBinary<'a>`: Zero-copy lifetime-bound parser verifying `ELF_MAGIC` (`[0x7F, 'E', 'L', 'F']`), `ELF_CLASS_64`, `ELF_DATA_2LSB`, and machine `EM_X86_64` (`0x3E`).
2. **`loader/process.rs`**:
   - `ProcessAddressSpace`: Manages the root `pml4_phys` physical address, entry point, user stack top, and vector of allocated physical frames.
   - `map_user_page()`: Creates multi-level intermediate tables (`PDPT`, `PD`, `PT`) on demand with `USER_ACCESSIBLE` flags.
   - `translate_user_addr()`: Walks page tables for memory verification.
   - `impl Drop`: Automatically releases all allocated frames back to the bitmap allocator.
3. **`loader/mod.rs`**:
   - `load_app_from_fs(path)`: Reads executable file from in-memory RamFS and loads it into a fresh `ProcessAddressSpace`.
   - `run_loader_tests()`: Verification test suite executing in the kernel.

---

## 4. Verification Self-Tests (VirtualBox 7.2 Live Boot)

```text
[TEST] Running Native Application Loader (User ELF64) Verification Tests...
  [PASS] Loaded '/bin/hello-world' into isolated ProcessAddressSpace.
  [PASS] Verified Entry Point: 0x40000000
  [PASS] Code virtual address 0x40000000 translates to physical frame 0x102000
  [PASS] Verified data segment content: 'Hello from Kinetix OS Native Application!'
  [PASS] User stack verified (16 KiB mapped at 0x7FFFF000, canary written & read).
  [PASS] ProcessAddressSpace tracks 11 allocated physical frames.
  [PASS] Process drop successfully reclaimed all 11 physical frames.
  [PASS] Corrupted binary rejected cleanly with ElfError.
[TEST] All Native Application Loader Verification Tests PASSED!
```
