//! 4-Level Virtual Memory Paging Subsystem for x86_64.
//!
//! Provides safe abstractions for Page Tables (PML4, PDPT, PD, PT),
//! Page Table Entries (PTE), virtual-to-physical address translation,
//! and TLB management.

use super::frame::PhysAddr;

/// Virtual address wrapper providing type safety and canonical validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtAddr(u64);

#[allow(dead_code)]
impl VirtAddr {
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

    #[inline]
    pub fn as_mut_ptr<T>(&self) -> *mut T {
        self.0 as *mut T
    }

    #[inline]
    pub const fn pml4_index(&self) -> usize {
        ((self.0 >> 39) & 0x1FF) as usize
    }

    #[inline]
    pub const fn pdpt_index(&self) -> usize {
        ((self.0 >> 30) & 0x1FF) as usize
    }

    #[inline]
    pub const fn pd_index(&self) -> usize {
        ((self.0 >> 21) & 0x1FF) as usize
    }

    #[inline]
    pub const fn pt_index(&self) -> usize {
        ((self.0 >> 12) & 0x1FF) as usize
    }

    #[inline]
    pub const fn page_offset(&self) -> u64 {
        self.0 & 0xFFF
    }
}

/// Bitflags for x86_64 page table entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageTableFlags(u64);

#[allow(dead_code)]
impl PageTableFlags {
    pub const PRESENT: Self = Self(1 << 0);
    pub const WRITABLE: Self = Self(1 << 1);
    pub const USER_ACCESSIBLE: Self = Self(1 << 2);
    pub const WRITE_THROUGH: Self = Self(1 << 3);
    pub const NO_CACHE: Self = Self(1 << 4);
    pub const ACCESSED: Self = Self(1 << 5);
    pub const DIRTY: Self = Self(1 << 6);
    pub const HUGE_PAGE: Self = Self(1 << 7);
    pub const GLOBAL: Self = Self(1 << 8);
    pub const NO_EXECUTE: Self = Self(1 << 63);

    #[inline]
    pub const fn empty() -> Self {
        Self(0)
    }

    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    #[inline]
    pub const fn bits(&self) -> u64 {
        self.0
    }

    #[inline]
    pub const fn from_bits_truncate(bits: u64) -> Self {
        Self(bits)
    }
}

/// A 64-bit page table entry.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct PageTableEntry(u64);

#[allow(dead_code)]
impl PageTableEntry {
    pub const UNUSED: Self = Self(0);

    #[inline]
    pub const fn is_unused(&self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub fn set_unused(&mut self) {
        self.0 = 0;
    }

    #[inline]
    pub fn flags(&self) -> PageTableFlags {
        PageTableFlags::from_bits_truncate(self.0)
    }

    #[inline]
    pub fn is_present(&self) -> bool {
        self.flags().contains(PageTableFlags::PRESENT)
    }

    #[inline]
    pub fn is_huge(&self) -> bool {
        self.flags().contains(PageTableFlags::HUGE_PAGE)
    }

    #[inline]
    pub fn addr(&self) -> PhysAddr {
        PhysAddr::new(self.0 & 0x000F_FFFF_FFFF_F000)
    }

    #[inline]
    pub fn set_addr(&mut self, addr: PhysAddr, flags: PageTableFlags) {
        self.0 = (addr.as_u64() & 0x000F_FFFF_FFFF_F000) | flags.bits();
    }
}

/// A 4 KiB aligned page table containing 512 entries.
#[repr(align(4096))]
pub struct PageTable {
    pub entries: [PageTableEntry; 512],
}

#[allow(dead_code)]
impl PageTable {
    pub const fn new() -> Self {
        Self {
            entries: [PageTableEntry::UNUSED; 512],
        }
    }

    pub fn zero(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.set_unused();
        }
    }
}

/// Reads the active physical address of the Level 4 Page Table (PML4) from CPU register CR3.
pub fn read_cr3() -> PhysAddr {
    let val: u64;
    unsafe {
        core::arch::asm!("mov {}, cr3", out(reg) val, options(nomem, nostack, preserves_flags));
    }
    PhysAddr::new(val & 0x000F_FFFF_FFFF_F000)
}

/// Invalidates the TLB entry for a given virtual address.
#[allow(dead_code)]
pub fn flush_tlb(addr: VirtAddr) {
    unsafe {
        core::arch::asm!("invlpg [{}]", in(reg) addr.as_u64(), options(nostack, preserves_flags));
    }
}

/// Walks the current 4-level page table to translate a virtual address to its physical counterpart.
///
/// Note: During UEFI handoff, identity-mapping covers all physical RAM, so page table pointers
/// in entries can be accessed directly as physical memory pointers.
pub fn translate_addr(vaddr: VirtAddr) -> Option<PhysAddr> {
    let pml4_phys = read_cr3();
    let pml4 = unsafe { &*(pml4_phys.as_ptr::<PageTable>()) };

    let pml4_entry = pml4.entries[vaddr.pml4_index()];
    if !pml4_entry.is_present() {
        return None;
    }

    let pdpt = unsafe { &*(pml4_entry.addr().as_ptr::<PageTable>()) };
    let pdpt_entry = pdpt.entries[vaddr.pdpt_index()];
    if !pdpt_entry.is_present() {
        return None;
    }

    // 1 GiB Huge Page support
    if pdpt_entry.is_huge() {
        let base = pdpt_entry.addr().as_u64();
        let offset = vaddr.as_u64() & 0x3FFF_FFFF;
        return Some(PhysAddr::new(base + offset));
    }

    let pd = unsafe { &*(pdpt_entry.addr().as_ptr::<PageTable>()) };
    let pd_entry = pd.entries[vaddr.pd_index()];
    if !pd_entry.is_present() {
        return None;
    }

    // 2 MiB Huge Page support
    if pd_entry.is_huge() {
        let base = pd_entry.addr().as_u64();
        let offset = vaddr.as_u64() & 0x1F_FFFF;
        return Some(PhysAddr::new(base + offset));
    }

    let pt = unsafe { &*(pd_entry.addr().as_ptr::<PageTable>()) };
    let pt_entry = pt.entries[vaddr.pt_index()];
    if !pt_entry.is_present() {
        return None;
    }

    Some(PhysAddr::new(pt_entry.addr().as_u64() + vaddr.page_offset()))
}
