# ADR-004: Memory Management Architecture

## Status
**Accepted**

## Context
Operating systems require rigorous management of physical memory frames, virtual address translation via CPU page tables, and dynamic memory allocation for kernel data structures. In x86-64 Long Mode, the hardware Memory Management Unit (MMU) operates with 4-level paging (PML4, PDPT, PD, PT), translating 48-bit virtual addresses into 52-bit physical addresses.

## Architecture Design

### 1. Physical Memory Management: Bitmap Frame Allocator
* The bootloader provides a memory map array indicating usable physical regions (`EfiConventionalMemory`).
* The kernel initialises a bitmapped physical frame allocator (`BitmapFrameAllocator`) placed in the first large contiguous block of usable RAM.
* Each bit in the bitmap tracks the allocation state of one 4096-byte physical frame (`0` = free, `1` = allocated).
* *Allocation Complexity*: $O(1)$ amortized using word-aligned scanning (`u64::trailing_zeros`).

### 2. Virtual Memory: 4-Level Paging with Higher-Half Direct Mapping
* **Lower Half (`0x0000_0000_0000_0000` - `0x0000_7FFF_FFFF_FFFF`)**:
  Reserved exclusively for user-space application processes (Ring 3 code, data, heap, and user stacks).
* **Higher Half (`0xFFFF_8000_0000_0000` - `0xFFFF_FFFF_FFFF_FFFF`)**:
  Reserved for kernel code, data, page tables, MMIO devices, and kernel stacks.
* **Physical Memory Mapping**:
  A region of virtual address space (e.g., offset `0xFFFF_8800_0000_0000`) is mapped to physical address `0x0` onwards (Physical Memory Direct Map), allowing the kernel to access any physical frame directly by adding the offset.

### 3. Dynamic Kernel Heap: Free-List / Buddy Allocator
* Once paging is active, the kernel allocates a designated virtual memory span (e.g. 16 MiB) backed by physical frames for the kernel heap.
* An intrusive linked free-list allocator is implemented, satisfying the Rust `core::alloc::GlobalAlloc` trait.
* This unlocks Rust's `alloc` crate, enabling safe usage of `Vec`, `Box`, `String`, and collections inside kernel subsystems.
