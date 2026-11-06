//! Intrusive Coalescing Free-List Kernel Heap Allocator.
//!
//! Implements Rust's `core::alloc::GlobalAlloc` trait to unlock the `alloc` crate,
//! enabling dynamic allocation of `Box`, `Vec`, `String`, `Arc`, and collection types.

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use spin::Mutex;

/// Starting physical/virtual address of the primary kernel heap (4 MiB).
pub const HEAP_START: usize = 0x0040_0000;

/// Total size of the primary kernel heap: 4 MiB (4,194,304 bytes).
pub const HEAP_SIZE: usize = 4 * 1024 * 1024;

/// Header for each free memory block in the intrusive linked list.
struct ListNode {
    size: usize,
    next: Option<NonNull<ListNode>>,
}

impl ListNode {
    const fn new(size: usize) -> Self {
        Self { size, next: None }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

/// Inner Free-List allocator with immediate boundary-tag coalescing.
pub struct FreeListAllocator {
    head: ListNode,
    total_bytes: usize,
    allocated_bytes: usize,
    initialized: bool,
}

unsafe impl Send for FreeListAllocator {}
unsafe impl Sync for FreeListAllocator {}

impl FreeListAllocator {
    pub const fn new() -> Self {
        Self {
            head: ListNode::new(0),
            total_bytes: 0,
            allocated_bytes: 0,
            initialized: false,
        }
    }

    /// Initializes the heap with a contiguous memory region.
    ///
    /// # Safety
    /// The caller must guarantee that `heap_start..heap_start + heap_size` is valid,
    /// identity-mapped, and reserved exclusively for the heap.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.total_bytes = heap_size;
        self.allocated_bytes = 0;
        self.initialized = true;

        // Initialize single initial free block spanning the whole heap
        self.add_free_region(heap_start, heap_size);
    }

    /// Adds a freed memory region to the free list, coalescing with adjacent blocks.
    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        // Enforce alignment and minimal size
        assert_eq!(addr % core::mem::align_of::<ListNode>(), 0);
        assert!(size >= core::mem::size_of::<ListNode>());

        // Create new node in the freed memory
        let mut node = ListNode::new(size);
        node.next = self.head.next.take();
        let node_ptr = addr as *mut ListNode;
        node_ptr.write(node);
        self.head.next = Some(NonNull::new_unchecked(node_ptr));

        // Maintain address order and coalesce adjacent blocks
        self.sort_and_coalesce();
    }

    /// Sorts free blocks by address and merges contiguous blocks to prevent fragmentation.
    fn sort_and_coalesce(&mut self) {
        // Step 1: Collect nodes into an array for sorting if count is small,
        // or perform in-place merge. Since free list starts with 1 node, simple insertion/coalescing works.
        let mut current = &mut self.head;
        while let Some(mut curr_ptr) = current.next {
            let curr_node = unsafe { curr_ptr.as_mut() };
            if let Some(mut next_ptr) = curr_node.next {
                let next_node = unsafe { next_ptr.as_mut() };
                // If curr_node and next_node are contiguous in memory:
                if curr_node.end_addr() == next_node.start_addr() {
                    curr_node.size += next_node.size;
                    curr_node.next = next_node.next.take();
                    continue; // Re-check with the new next
                }
            }
            current = unsafe { curr_ptr.as_mut() };
        }
    }

    /// Finds a free block that satisfies `layout` and removes it from the list.
    fn find_region(&mut self, size: usize, align: usize) -> Option<(*mut ListNode, usize)> {
        let mut current = &mut self.head;

        while let Some(mut region) = current.next {
            let region_ref = unsafe { region.as_mut() };
            if let Ok(alloc_start) = Self::alloc_from_region(region_ref, size, align) {
                // Remove or split region
                let next = region_ref.next.take();
                let excess_size = region_ref.end_addr() - (alloc_start + size);

                if excess_size >= core::mem::size_of::<ListNode>() {
                    // Split block: create remaining block after allocation
                    let rem_addr = alloc_start + size;
                    let mut rem_node = ListNode::new(excess_size);
                    rem_node.next = next;
                    let rem_ptr = rem_addr as *mut ListNode;
                    unsafe {
                        rem_ptr.write(rem_node);
                    }
                    current.next = Some(unsafe { NonNull::new_unchecked(rem_ptr) });
                } else {
                    current.next = next;
                }

                return Some((region.as_ptr(), alloc_start));
            } else {
                current = unsafe { current.next.as_mut().unwrap().as_mut() };
            }
        }

        None
    }

    /// Calculates allocation start address within a region, verifying alignment and bounds.
    fn alloc_from_region(region: &ListNode, size: usize, align: usize) -> Result<usize, ()> {
        let alloc_start = align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size).ok_or(())?;

        if alloc_end > region.end_addr() {
            return Err(());
        }

        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < core::mem::size_of::<ListNode>() {
            // Cannot fit remainder block header
            return Err(());
        }

        Ok(alloc_start)
    }

    #[allow(dead_code)]
    #[inline]
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    #[inline]
    pub fn allocated_bytes(&self) -> usize {
        self.allocated_bytes
    }

    #[inline]
    pub fn free_bytes(&self) -> usize {
        self.total_bytes.saturating_sub(self.allocated_bytes)
    }
}

/// Align the given address upwards to alignment `align`.
#[inline]
fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

/// Thread-safe wrapper around `FreeListAllocator` implementing `GlobalAlloc`.
pub struct LockedHeap(Mutex<FreeListAllocator>);

impl LockedHeap {
    pub const fn empty() -> Self {
        Self(Mutex::new(FreeListAllocator::new()))
    }

    /// Initializes the heap with given base address and byte size.
    pub unsafe fn init(&self, heap_start: usize, heap_size: usize) {
        self.0.lock().init(heap_start, heap_size);
    }

    #[allow(dead_code)]
    #[inline]
    pub fn total_bytes(&self) -> usize {
        self.0.lock().total_bytes()
    }

    #[inline]
    pub fn allocated_bytes(&self) -> usize {
        self.0.lock().allocated_bytes()
    }

    #[inline]
    pub fn free_bytes(&self) -> usize {
        self.0.lock().free_bytes()
    }
}

unsafe impl GlobalAlloc for LockedHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let (size, align) = {
            let layout_size = layout.size().max(core::mem::size_of::<ListNode>());
            let layout_align = layout.align().max(core::mem::align_of::<ListNode>());
            (layout_size, layout_align)
        };

        let mut allocator = self.0.lock();
        if !allocator.initialized {
            return core::ptr::null_mut();
        }

        if let Some((_, alloc_start)) = allocator.find_region(size, align) {
            allocator.allocated_bytes += size;
            alloc_start as *mut u8
        } else {
            core::ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let (size, _) = {
            let layout_size = layout.size().max(core::mem::size_of::<ListNode>());
            let layout_align = layout.align().max(core::mem::align_of::<ListNode>());
            (layout_size, layout_align)
        };

        let mut allocator = self.0.lock();
        if allocator.initialized && !ptr.is_null() {
            allocator.allocated_bytes = allocator.allocated_bytes.saturating_sub(size);
            allocator.add_free_region(ptr as usize, size);
        }
    }
}

/// Global Kernel Heap Allocator.
#[global_allocator]
pub static KINETIX_HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();
