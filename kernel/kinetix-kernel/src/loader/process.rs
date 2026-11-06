//! User Process Virtual Address Space & Page Table Management.
//!
//! Provides isolated 4-level page tables for user processes, mapping user code,
//! data, bss, and stack into dedicated physical memory frames while preserving
//! kernel identity mappings.

use alloc::vec::Vec;
use crate::memory::frame::{PhysAddr, FRAME_ALLOCATOR};
use crate::memory::paging::{self, PageTable, PageTableFlags, VirtAddr};

/// Base virtual address where user executables are linked (1 GiB boundary).
pub const USER_CODE_BASE: u64 = 0x4000_0000;

/// Top of the user process stack space (just below 2 GiB boundary).
pub const USER_STACK_TOP: u64 = 0x7FFF_F000;

/// Default user stack size: 16 KiB (4 pages).
pub const USER_STACK_SIZE_PAGES: usize = 4;

/// Isolated address space managing memory allocations and page tables for a user process.
pub struct ProcessAddressSpace {
    pub pml4_phys: PhysAddr,
    pub entry_point: VirtAddr,
    pub user_stack_top: VirtAddr,
    pub allocated_frames: Vec<PhysAddr>,
}

impl ProcessAddressSpace {
    /// Creates a new isolated address space with inherited kernel mappings.
    pub fn new(entry_point: VirtAddr, user_stack_top: VirtAddr) -> Result<Self, &'static str> {
        let mut allocated_frames = Vec::new();

        // 1. Allocate physical frame for process PML4
        let pml4_phys = FRAME_ALLOCATOR
            .lock()
            .allocate_frame()
            .ok_or("Failed to allocate physical frame for PML4")?;
        allocated_frames.push(pml4_phys);

        let pml4 = unsafe { &mut *(pml4_phys.as_mut_ptr::<PageTable>()) };
        pml4.zero();

        // 2. Allocate physical frame for process PDPT (PML4[0])
        let pdpt_phys = FRAME_ALLOCATOR
            .lock()
            .allocate_frame()
            .ok_or("Failed to allocate physical frame for PDPT")?;
        allocated_frames.push(pdpt_phys);

        let pdpt = unsafe { &mut *(pdpt_phys.as_mut_ptr::<PageTable>()) };
        pdpt.zero();

        // 3. Link PDPT into PML4 at entry 0 with user-accessible permissions
        pml4.entries[0].set_addr(
            pdpt_phys,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
        );

        // 4. Inherit kernel mappings from active boot PML4
        let active_pml4_phys = paging::read_cr3();
        let active_pml4 = unsafe { &*(active_pml4_phys.as_ptr::<PageTable>()) };

        // Kernel identity-mapping lives in active PML4[0] -> PDPT[0] (0..1 GiB)
        if active_pml4.entries[0].is_present() {
            let kernel_pdpt_phys = active_pml4.entries[0].addr();
            let kernel_pdpt = unsafe { &*(kernel_pdpt_phys.as_ptr::<PageTable>()) };
            // Copy PDPT[0] so kernel code, heap, GDT, IDT, and stacks are present during interrupts
            pdpt.entries[0] = kernel_pdpt.entries[0];
        }

        // Copy any higher-level entries present in the boot PML4 (e.g. framebuffer)
        for i in 1..512 {
            if active_pml4.entries[i].is_present() {
                pml4.entries[i] = active_pml4.entries[i];
            }
        }

        Ok(Self {
            pml4_phys,
            entry_point,
            user_stack_top,
            allocated_frames,
        })
    }

    /// Maps a 4 KiB virtual page to a physical frame in this process's page table.
    pub fn map_user_page(
        &mut self,
        vaddr: VirtAddr,
        paddr: PhysAddr,
        flags: PageTableFlags,
    ) -> Result<(), &'static str> {
        let pml4 = unsafe { &mut *(self.pml4_phys.as_mut_ptr::<PageTable>()) };

        let pml4_idx = vaddr.pml4_index();
        let pdpt_idx = vaddr.pdpt_index();
        let pd_idx = vaddr.pd_index();
        let pt_idx = vaddr.pt_index();

        // Level 4 (PML4) -> Level 3 (PDPT)
        let pdpt_phys = if pml4.entries[pml4_idx].is_present() {
            pml4.entries[pml4_idx].addr()
        } else {
            let frame = FRAME_ALLOCATOR
                .lock()
                .allocate_frame()
                .ok_or("OOM allocating PDPT")?;
            self.allocated_frames.push(frame);
            unsafe {
                core::ptr::write_bytes(frame.as_mut_ptr::<u8>(), 0, 4096);
            }
            pml4.entries[pml4_idx].set_addr(
                frame,
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
            );
            frame
        };

        // Level 3 (PDPT) -> Level 2 (PD)
        let pdpt = unsafe { &mut *(pdpt_phys.as_mut_ptr::<PageTable>()) };
        let pd_phys = if pdpt.entries[pdpt_idx].is_present() {
            pdpt.entries[pdpt_idx].addr()
        } else {
            let frame = FRAME_ALLOCATOR
                .lock()
                .allocate_frame()
                .ok_or("OOM allocating PD")?;
            self.allocated_frames.push(frame);
            unsafe {
                core::ptr::write_bytes(frame.as_mut_ptr::<u8>(), 0, 4096);
            }
            pdpt.entries[pdpt_idx].set_addr(
                frame,
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
            );
            frame
        };

        // Level 2 (PD) -> Level 1 (PT)
        let pd = unsafe { &mut *(pd_phys.as_mut_ptr::<PageTable>()) };
        let pt_phys = if pd.entries[pd_idx].is_present() {
            pd.entries[pd_idx].addr()
        } else {
            let frame = FRAME_ALLOCATOR
                .lock()
                .allocate_frame()
                .ok_or("OOM allocating PT")?;
            self.allocated_frames.push(frame);
            unsafe {
                core::ptr::write_bytes(frame.as_mut_ptr::<u8>(), 0, 4096);
            }
            pd.entries[pd_idx].set_addr(
                frame,
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
            );
            frame
        };

        // Level 1 (PT) -> Leaf entry pointing to physical frame
        let pt = unsafe { &mut *(pt_phys.as_mut_ptr::<PageTable>()) };
        pt.entries[pt_idx].set_addr(paddr, flags);

        Ok(())
    }

    /// Walks this process's page table hierarchy to translate a virtual address to physical.
    pub fn translate_user_addr(&self, vaddr: VirtAddr) -> Option<PhysAddr> {
        let pml4 = unsafe { &*(self.pml4_phys.as_ptr::<PageTable>()) };

        let pml4_entry = pml4.entries[vaddr.pml4_index()];
        if !pml4_entry.is_present() {
            return None;
        }

        let pdpt = unsafe { &*(pml4_entry.addr().as_ptr::<PageTable>()) };
        let pdpt_entry = pdpt.entries[vaddr.pdpt_index()];
        if !pdpt_entry.is_present() {
            return None;
        }

        let pd = unsafe { &*(pdpt_entry.addr().as_ptr::<PageTable>()) };
        let pd_entry = pd.entries[vaddr.pd_index()];
        if !pd_entry.is_present() {
            return None;
        }

        let pt = unsafe { &*(pd_entry.addr().as_ptr::<PageTable>()) };
        let pt_entry = pt.entries[vaddr.pt_index()];
        if !pt_entry.is_present() {
            return None;
        }

        Some(PhysAddr::new(pt_entry.addr().as_u64() + vaddr.page_offset()))
    }
}

impl Drop for ProcessAddressSpace {
    fn drop(&mut self) {
        let mut fa = FRAME_ALLOCATOR.lock();
        for &frame in &self.allocated_frames {
            fa.deallocate_frame(frame);
        }
    }
}
