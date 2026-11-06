//! Physical Frame Allocator for Kinetix OS.
//!
//! Tracks physical memory frames using a 4 GiB bitmap where 1 bit represents
//! a 4 KiB frame (0 = Free, 1 = Used/Reserved).

use kinetix_boot_protocol::{FramebufferInfo, MemoryMap, MemoryType, ModuleList};
use spin::Mutex;

/// Architecture page size: 4096 bytes (4 KiB).
pub const PAGE_SIZE: u64 = 4096;

/// Maximum physical memory managed by this bitmap: 4 GiB (4,294,967,296 bytes).
pub const MAX_PHYSICAL_MEMORY: u64 = 4 * 1024 * 1024 * 1024;

/// Total number of 4 KiB frames in 4 GiB: 1,048,576 frames.
pub const TOTAL_FRAMES: usize = (MAX_PHYSICAL_MEMORY / PAGE_SIZE) as usize;

/// Number of 64-bit words needed to represent 1,048,576 bits: 16,384 words (128 KiB).
pub const BITMAP_WORDS: usize = TOTAL_FRAMES / 64;

/// Physical address wrapper providing type safety.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PhysAddr(u64);

impl PhysAddr {
    #[inline]
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    #[inline]
    pub const fn as_u64(&self) -> u64 {
        self.0
    }

    #[inline]
    pub fn as_ptr<T>(&self) -> *const T {
        self.0 as *const T
    }

    #[allow(dead_code)]
    #[inline]
    pub fn as_mut_ptr<T>(&self) -> *mut T {
        self.0 as *mut T
    }

    #[allow(dead_code)]
    #[inline]
    pub const fn is_aligned_to(&self, alignment: u64) -> bool {
        (self.0 % alignment) == 0
    }
}

/// Global Bitmap-based Physical Frame Allocator.
pub struct BitmapFrameAllocator {
    bitmap: [u64; BITMAP_WORDS],
    initialized: bool,
    total_usable_frames: usize,
    allocated_frames: usize,
    last_search_index: usize,
}

impl BitmapFrameAllocator {
    pub const fn new() -> Self {
        Self {
            bitmap: [!0u64; BITMAP_WORDS],
            initialized: false,
            total_usable_frames: 0,
            allocated_frames: 0,
            last_search_index: 0,
        }
    }

    /// Initializes the frame allocator from firmware-provided memory map and reservations.
    pub fn init(
        &mut self,
        memory_map: &MemoryMap,
        kernel_start: u64,
        kernel_end: u64,
        framebuffer: &FramebufferInfo,
        modules: &ModuleList,
    ) {
        // Step 1: Mark all frames as reserved (bit = 1)
        for word in self.bitmap.iter_mut() {
            *word = !0u64;
        }

        let mut usable_count = 0;

        // Step 2: Clear bits for all usable conventional RAM regions reported by UEFI
        for i in 0..memory_map.region_count {
            let region = &memory_map.regions[i];
            if region.region_type == MemoryType::Usable {
                let start_frame = ((region.start + PAGE_SIZE - 1) / PAGE_SIZE) as usize;
                let end_frame = ((region.start + region.length) / PAGE_SIZE) as usize;

                let clamped_end = if end_frame > TOTAL_FRAMES { TOTAL_FRAMES } else { end_frame };

                for frame in start_frame..clamped_end {
                    self.set_bit(frame, false);
                    usable_count += 1;
                }
            }
        }

        self.total_usable_frames = usable_count;

        // Step 3: Explicitly protect critical hardware & OS structures:

        // 3a. Reserve Page 0 (0x0000..0x1000) for null pointer trap
        self.reserve_range(0, PAGE_SIZE);

        // 3b. Reserve first 1 MiB (0x000000..0x100000) for BIOS/IVT/BDA/VGA compatibility
        self.reserve_range(0, 1024 * 1024);

        // 3c. Reserve kernel binary image (align kernel end to 4 KiB boundary)
        let k_start = kernel_start;
        let k_end_aligned = (kernel_end + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        self.reserve_range(k_start, k_end_aligned - k_start);

        // 3d. Reserve GOP Framebuffer region if within 4 GiB
        if framebuffer.physical_address != 0 && framebuffer.physical_address < MAX_PHYSICAL_MEMORY {
            self.reserve_range(framebuffer.physical_address, framebuffer.size as u64);
        }

        // 3e. Reserve bootloader staged modules
        for i in 0..modules.count {
            let m = &modules.modules[i];
            if m.physical_address != 0 && m.physical_address < MAX_PHYSICAL_MEMORY {
                self.reserve_range(m.physical_address, m.size as u64);
            }
        }

        // Calculate initially allocated frames among usable frames
        let mut free_count = 0;
        for i in 0..TOTAL_FRAMES {
            if !self.get_bit(i) {
                free_count += 1;
            }
        }
        self.allocated_frames = self.total_usable_frames.saturating_sub(free_count);
        self.last_search_index = 0;
        self.initialized = true;
    }

    /// Reserves a physical memory range [start, start + length)
    pub fn reserve_range(&mut self, start: u64, length: u64) {
        let start_frame = (start / PAGE_SIZE) as usize;
        let end_frame = ((start + length + PAGE_SIZE - 1) / PAGE_SIZE) as usize;
        let clamped_end = if end_frame > TOTAL_FRAMES { TOTAL_FRAMES } else { end_frame };

        for frame in start_frame..clamped_end {
            self.set_bit(frame, true);
        }
    }

    /// Allocates a single 4 KiB physical frame.
    pub fn allocate_frame(&mut self) -> Option<PhysAddr> {
        if !self.initialized {
            return None;
        }

        let start_word = self.last_search_index / 64;

        // Scan from last search index to end of bitmap
        for word_idx in start_word..BITMAP_WORDS {
            let word = self.bitmap[word_idx];
            if word != !0u64 {
                let bit_idx = (!word).trailing_zeros() as usize;
                let frame_idx = word_idx * 64 + bit_idx;
                if frame_idx < TOTAL_FRAMES {
                    self.set_bit(frame_idx, true);
                    self.allocated_frames += 1;
                    self.last_search_index = frame_idx + 1;
                    return Some(PhysAddr::new((frame_idx as u64) * PAGE_SIZE));
                }
            }
        }

        // Wrap around from 0 to start_word
        for word_idx in 0..start_word {
            let word = self.bitmap[word_idx];
            if word != !0u64 {
                let bit_idx = (!word).trailing_zeros() as usize;
                let frame_idx = word_idx * 64 + bit_idx;
                if frame_idx < TOTAL_FRAMES {
                    self.set_bit(frame_idx, true);
                    self.allocated_frames += 1;
                    self.last_search_index = frame_idx + 1;
                    return Some(PhysAddr::new((frame_idx as u64) * PAGE_SIZE));
                }
            }
        }

        None
    }

    /// Allocates `count` contiguous 4 KiB physical frames.
    #[allow(dead_code)]
    pub fn allocate_contiguous_frames(&mut self, count: usize) -> Option<PhysAddr> {
        if !self.initialized || count == 0 {
            return None;
        }

        let mut consecutive = 0;
        let mut start_frame = 0;

        for frame in 0..TOTAL_FRAMES {
            if !self.get_bit(frame) {
                if consecutive == 0 {
                    start_frame = frame;
                }
                consecutive += 1;
                if consecutive == count {
                    // Found contiguous run; mark all as allocated
                    for f in start_frame..(start_frame + count) {
                        self.set_bit(f, true);
                    }
                    self.allocated_frames += count;
                    return Some(PhysAddr::new((start_frame as u64) * PAGE_SIZE));
                }
            } else {
                consecutive = 0;
            }
        }

        None
    }

    /// Deallocates a previously allocated physical frame.
    pub fn deallocate_frame(&mut self, addr: PhysAddr) {
        if !self.initialized {
            return;
        }

        let paddr = addr.as_u64();
        if paddr % PAGE_SIZE != 0 {
            return; // Not aligned
        }

        let frame_idx = (paddr / PAGE_SIZE) as usize;
        if frame_idx < TOTAL_FRAMES && self.get_bit(frame_idx) {
            self.set_bit(frame_idx, false);
            self.allocated_frames = self.allocated_frames.saturating_sub(1);
            if frame_idx < self.last_search_index {
                self.last_search_index = frame_idx;
            }
        }
    }

    #[inline]
    fn get_bit(&self, frame: usize) -> bool {
        let word_idx = frame / 64;
        let bit_idx = frame % 64;
        (self.bitmap[word_idx] & (1u64 << bit_idx)) != 0
    }

    #[inline]
    fn set_bit(&mut self, frame: usize, value: bool) {
        let word_idx = frame / 64;
        let bit_idx = frame % 64;
        if value {
            self.bitmap[word_idx] |= 1u64 << bit_idx;
        } else {
            self.bitmap[word_idx] &= !(1u64 << bit_idx);
        }
    }

    #[inline]
    pub fn total_usable_frames(&self) -> usize {
        self.total_usable_frames
    }

    #[inline]
    pub fn allocated_frames(&self) -> usize {
        self.allocated_frames
    }

    #[inline]
    pub fn free_frames(&self) -> usize {
        self.total_usable_frames.saturating_sub(self.allocated_frames)
    }

    #[inline]
    pub fn total_usable_bytes(&self) -> u64 {
        (self.total_usable_frames as u64) * PAGE_SIZE
    }

    #[inline]
    pub fn allocated_bytes(&self) -> u64 {
        (self.allocated_frames as u64) * PAGE_SIZE
    }

    #[inline]
    pub fn free_bytes(&self) -> u64 {
        (self.free_frames() as u64) * PAGE_SIZE
    }
}

/// Global thread-safe mutex wrapper for physical frame allocation.
pub static FRAME_ALLOCATOR: Mutex<BitmapFrameAllocator> = Mutex::new(BitmapFrameAllocator::new());
