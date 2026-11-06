# Kinetix OS — Kernel Memory Management Architecture

**Document ID:** SPEC-KERNEL-MEM-001  
**Status:** Implemented & Verified in VirtualBox  
**Target Architecture:** x86_64 Long Mode (4-Level Paging)  
**Primary Language:** Rust (`core`, `alloc`, `no_std`)  

---

## 1. Overview and Architecture

Kinetix OS implements a 3-tier memory management architecture designed for complete runtime independence, deterministic physical memory control, and zero reliance on external or host operating-system facilities:

```
+-------------------------------------------------------------------------+
|                  Rust Standard Collections & Kernel Objects             |
|         (Vec<T>, Box<T>, String, Arc<T>, TaskControlBlock, etc.)         |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
|                  Tier 3: Kernel Heap Allocator                          |
|         - Intrusive Coalescing Free-List Allocator                      |
|         - Implements core::alloc::GlobalAlloc                           |
|         - 4 MiB Initial Kernel Heap (0x0040_0000 .. 0x0080_0000)        |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
|                  Tier 2: 4-Level Paging Subsystem                       |
|         - x86_64 48-bit Canonical Virtual Addressing                    |
|         - Active Page Directory Root in CR3                             |
|         - PML4 -> PDPT -> PD -> PT Table Walking & Translation          |
|         - 4 KiB, 2 MiB, and 1 GiB Huge Page Support                     |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
|                  Tier 1: Physical Frame Allocator                       |
|         - 4 GiB Physical Memory Bitmap (1,048,576 frames @ 4 KiB)       |
|         - 128 KiB Bitmap residing in Kernel BSS (zero disk footprint)   |
|         - Firmware MemoryMap Ingestion & Sanitization                   |
|         - Hardware & Kernel Reservation Guards                          |
+-------------------------------------------------------------------------+
```

---

## 2. Tier 1: Physical Frame Allocator (`BitmapFrameAllocator`)

### 2.1 Bitmap Specifications
- **Page Size:** 4096 bytes (4 KiB standard x86_64 page).
- **Physical Address Ceiling:** 4 GiB ($4 \times 1024 \times 1024 \times 1024$ bytes).
- **Total Physical Frames:** 1,048,576 frames.
- **Bitmap Footprint:** 16,384 `u64` words = 131,072 bytes (128 KiB).
- **Storage Location:** Kernel `.bss` section inside `BitmapFrameAllocator`. Takes 0 bytes on disk in the ELF binary and is initialized into zeroed memory during bootloader loading.

### 2.2 Memory Classification and Protection Guards
During kernel initialization (`memory::init`), the allocator scans the `BootInfo.memory_map` passed by the bootloader and marks conventional UEFI memory as free (bit = 0). The following regions are explicitly protected (bit = 1):

1. **Page 0 Guard (`0x0000_0000 .. 0x0000_1000`):** Protects against null pointer dereferences causing physical corruption.
2. **Real-Mode / Legacy Area (`0x0000_0000 .. 0x0010_0000`, 0 to 1 MiB):** Guards BIOS IVT, BDA, EBDA, and video memory compatibility zones.
3. **Kernel Binary Image (`__kernel_start .. __kernel_end`):** Dynamically calculated from linker symbols (loaded at physical `0x0020_0000`).
4. **Linear Framebuffer (`0x8000_0000`):** Protects the physical video memory aperture from being handed out as general RAM.
5. **Kernel Heap (`0x0040_0000 .. 0x0080_0000`):** Reserved exclusively for the kernel heap allocator.
6. **Preloaded Boot Modules:** Protects native application binaries staged by the bootloader.

---

## 3. Tier 2: 4-Level Virtual Memory Paging (`paging.rs`)

### 3.1 x86_64 48-bit Virtual Address Decomposition
A canonical 64-bit virtual address in long mode is structured as:

$$\text{VirtAddr} = \text{Sign Extension [63:48]} \mid \text{PML4 [47:39]} \mid \text{PDPT [38:30]} \mid \text{PD [29:21]} \mid \text{PT [20:12]} \mid \text{Offset [11:0]}$$

- **PML4 Index:** Bits 39–47 (512 entries per table).
- **PDPT Index:** Bits 30–38 (512 entries per table).
- **PD Index:** Bits 21–29 (512 entries per table).
- **PT Index:** Bits 12–20 (512 entries per table).
- **Offset:** Bits 0–11 (4096-byte alignment).

### 3.2 Page Table Entry (PTE) Flags
- `PRESENT (1 << 0)`: Page is mapped and valid.
- `WRITABLE (1 << 1)`: Read/write permission.
- `USER_ACCESSIBLE (1 << 2)`: Ring 3 User-space accessible.
- `WRITE_THROUGH (1 << 3)`: Write-through caching.
- `NO_CACHE (1 << 4)`: Disable CPU caching.
- `ACCESSED (1 << 5)`: CPU accessed indicator.
- `DIRTY (1 << 6)`: Page dirty indicator.
- `HUGE_PAGE (1 << 7)`: 2 MiB / 1 GiB mapping flag.
- `GLOBAL (1 << 8)`: Prevent TLB eviction on CR3 reload.
- `NO_EXECUTE (1 << 63)`: NX/XD bit preventing code execution.

### 3.3 Active Page Directory Root
The active Level 4 Page Table address is retrieved directly from CPU control register `CR3` via inline assembly:
```rust
pub fn read_cr3() -> PhysAddr {
    let val: u64;
    unsafe {
        core::arch::asm!("mov {}, cr3", out(reg) val, options(nomem, nostack, preserves_flags));
    }
    PhysAddr::new(val & 0x000F_FFFF_FFFF_F000)
}
```

---

## 4. Tier 3: Kernel Heap Allocator (`heap.rs`)

### 4.1 Coalescing Free-List Design
The kernel heap allocator implements an intrusive singly linked list of free memory blocks. Every free block begins with an intrusive `ListNode` header:
```rust
struct ListNode {
    size: usize,
    next: Option<NonNull<ListNode>>,
}
```

- **Allocation Strategy:** First-Fit search with exact splitting. If a block has excess capacity $\ge \text{size\_of::<ListNode>()}$, the block is split into an allocated payload and a remainder free block.
- **Deallocation Strategy:** In-place address sorting with immediate boundary-tag coalescing. When memory is freed, adjacent contiguous free blocks are merged into a single larger block, preventing heap fragmentation.
- **Thread Safety:** Synchronized via `spin::Mutex<FreeListAllocator>`.
- **Global Allocator Hook:** Implements `core::alloc::GlobalAlloc` and is registered as `#[global_allocator]`, unlocking Rust's native `alloc` crate.

### 4.2 Heap Location and Size
- **Virtual/Physical Base:** `0x0040_0000` (4 MiB physical).
- **Capacity:** 4 MiB (4,194,304 bytes).
- **Identity Mapped:** Guaranteed by UEFI page tables during initial kernel boot.

---

## 5. Live VirtualBox Verification Evidence

Booting with 2048 MiB RAM configured in Oracle VirtualBox 7.2 produced the following verified diagnostics on COM1 serial (`serial.log`):

```text
[MEMORY] Initializing Physical Frame Allocator...
  Kernel Image Span: 0x200000..0x229000 (167936 bytes)
  Total Usable RAM : 2004 MiB (513105 frames)
  Free Physical RAM: 2003 MiB (512945 frames)
  Reserved RAM     : 0 MiB (160 frames)
[MEMORY] Active 4-Level Page Table Root (CR3): 0x7EE01000
[MEMORY] Initializing Kernel Heap Allocator...
  Heap Range: 0x400000..0x800000 (4096 KiB)
  Heap Available: 4096 KiB (4194304 bytes)
[MEMORY] Memory subsystem initialized successfully.

[TEST] Running Memory Subsystem Verification Tests...
  [PASS] Physical Frame Allocator (alloc/dealloc/alignment)
  [PASS] 4-Level Paging Walk (Virt 0x200000 -> Phys 0x200000)
  [PASS] Kernel Heap Box allocation and reclamation
  [PASS] Dynamic Vec<u32> capacity growth, elements, and drop
  [PASS] Dynamic String formatting and heap allocation
[TEST] All Memory Subsystem Verification Tests PASSED!

[KERNEL] Phase 7 Milestone achieved: Kernel Memory Management operational.
```
