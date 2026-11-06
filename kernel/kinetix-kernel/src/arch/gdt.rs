//! Global Descriptor Table (GDT) & Task State Segment (TSS) for x86_64.
//!
//! Configures:
//! - 64-bit Kernel Code Segment (Ring 0)
//! - 64-bit Kernel Data Segment (Ring 0)
//! - Task State Segment with dedicated Interrupt Stack Table (IST) for Double Faults.

use core::arch::asm;
use spin::Mutex;

pub const KERNEL_CODE_SELECTOR: u16 = 0x08;
pub const KERNEL_DATA_SELECTOR: u16 = 0x10;
pub const USER_DATA_SELECTOR: u16 = 0x18;
pub const USER_CODE_SELECTOR: u16 = 0x20;
pub const TSS_SELECTOR: u16 = 0x28;

#[allow(dead_code)]
pub const USER_DATA_RPL3: u16 = USER_DATA_SELECTOR | 3; // 0x1B
#[allow(dead_code)]
pub const USER_CODE_RPL3: u16 = USER_CODE_SELECTOR | 3; // 0x23

pub const DOUBLE_FAULT_IST_INDEX: u8 = 1;

/// Dedicated 16 KiB stack for Double Fault exception handling.
#[repr(align(16))]
struct InterruptStack([u8; 16384]);
static mut DOUBLE_FAULT_STACK: InterruptStack = InterruptStack([0; 16384]);

/// Dedicated 16 KiB kernel stack used by the CPU when an interrupt or exception
/// occurs while executing in User Mode (Ring 3).
#[repr(align(16))]
struct KernelRsp0Stack([u8; 16384]);
static mut KERNEL_RSP0_STACK: KernelRsp0Stack = KernelRsp0Stack([0; 16384]);

/// Standard x86_64 Task State Segment.
#[repr(C, packed)]
pub struct TaskStateSegment {
    reserved_1: u32,
    pub privilege_stack_table: [u64; 3], // rsp0, rsp1, rsp2
    reserved_2: u64,
    pub interrupt_stack_table: [u64; 7], // ist1 .. ist7
    reserved_3: u64,
    reserved_4: u16,
    pub iomap_base: u16,
}

impl TaskStateSegment {
    pub const fn new() -> Self {
        Self {
            reserved_1: 0,
            privilege_stack_table: [0; 3],
            reserved_2: 0,
            interrupt_stack_table: [0; 7],
            reserved_3: 0,
            reserved_4: 0,
            iomap_base: core::mem::size_of::<Self>() as u16,
        }
    }
}

/// GDT Pointer structure loaded via `lgdt`.
#[repr(C, packed)]
struct GdtPointer {
    limit: u16,
    base: u64,
}

/// Fixed-size GDT containing: Null, Kernel Code, Kernel Data, User Data, User Code, and 16-byte TSS.
#[repr(C, align(16))]
struct GlobalDescriptorTable {
    entries: [u64; 7],
}

impl GlobalDescriptorTable {
    pub const fn new() -> Self {
        Self {
            entries: [
                0,                          // 0x00: Null
                0x0020_9A00_0000_0000,      // 0x08: Kernel Code 64-bit (Present, Ring 0, Executable, Readable, L=1)
                0x0000_9200_0000_0000,      // 0x10: Kernel Data 64-bit (Present, Ring 0, Writable)
                0x0000_F200_0000_0000,      // 0x18: User Data 64-bit (Present, Ring 3, Writable)
                0x0020_FA00_0000_0000,      // 0x20: User Code 64-bit (Present, Ring 3, Executable, Readable, L=1)
                0,                          // 0x28: TSS Low 8 bytes
                0,                          // 0x30: TSS High 8 bytes
            ],
        }
    }

    pub fn set_tss(&mut self, tss_ptr: *const TaskStateSegment) {
        let ptr = tss_ptr as u64;
        let limit = (core::mem::size_of::<TaskStateSegment>() - 1) as u64;

        // Type: 0x89 = Present, Ring 0, Available 64-bit TSS (Type 9)
        let low = (limit & 0xFFFF)
            | ((ptr & 0x00FF_FFFF) << 16)
            | (0x89u64 << 40)
            | (((limit >> 16) & 0x0F) << 48)
            | (((ptr >> 24) & 0xFF) << 56);

        let high = ptr >> 32;

        self.entries[5] = low;
        self.entries[6] = high;
    }
}

static mut TSS: TaskStateSegment = TaskStateSegment::new();
static GDT: Mutex<GlobalDescriptorTable> = Mutex::new(GlobalDescriptorTable::new());

/// Initializes and loads the Kernel GDT and Task State Segment.
pub fn init() {
    // 1. Configure TSS with dedicated Double Fault IST stack and Ring 3 -> Ring 0 RSP0 stack
    let df_stack_top = unsafe {
        let ptr = (&raw const DOUBLE_FAULT_STACK.0) as *const u8 as u64;
        ptr + 16384
    };

    let rsp0_stack_top = unsafe {
        let ptr = (&raw const KERNEL_RSP0_STACK.0) as *const u8 as u64;
        ptr + 16384
    };

    unsafe {
        (*(&raw mut TSS)).interrupt_stack_table[(DOUBLE_FAULT_IST_INDEX - 1) as usize] = df_stack_top;
        (*(&raw mut TSS)).privilege_stack_table[0] = rsp0_stack_top;
    }

    // 2. Insert TSS into GDT
    let mut gdt = GDT.lock();
    gdt.set_tss(&raw const TSS);

    // 3. Load GDT via `lgdt`
    let gdt_ptr = GdtPointer {
        limit: (core::mem::size_of::<GlobalDescriptorTable>() - 1) as u16,
        base: gdt.entries.as_ptr() as u64,
    };

    unsafe {
        asm!("lgdt [{}]", in(reg) &gdt_ptr, options(readonly, nostack, preserves_flags));
    }

    // 4. Reload 64-bit Code Segment (CS) via Far Return (using label '2:' to avoid LLVM binary ambiguity)
    unsafe {
        asm!(
            "push {sel}",
            "lea {tmp}, [rip + 2f]",
            "push {tmp}",
            "retfq",
            "2:",
            sel = in(reg) KERNEL_CODE_SELECTOR as u64,
            tmp = out(reg) _,
            options(preserves_flags)
        );
    }

    // 5. Reload Data Segments (DS, ES, SS, FS, GS)
    unsafe {
        asm!(
            "mov ds, {sel:x}",
            "mov es, {sel:x}",
            "mov ss, {sel:x}",
            "mov fs, {sel:x}",
            "mov gs, {sel:x}",
            sel = in(reg) KERNEL_DATA_SELECTOR,
            options(nostack, preserves_flags)
        );
    }

    // 6. Load Task State Segment register via `ltr`
    unsafe {
        asm!("ltr {sel:x}", sel = in(reg) TSS_SELECTOR, options(nostack, preserves_flags));
    }
}

/// Updates the Kernel Privilege Stack Pointer (RSP0) in the TSS.
#[allow(dead_code)]
pub fn set_kernel_stack(stack_top: u64) {
    unsafe {
        (*(&raw mut TSS)).privilege_stack_table[0] = stack_top;
    }
}

/// Returns the current RSP0 configured in the TSS.
#[allow(dead_code)]
pub fn get_kernel_stack() -> u64 {
    unsafe {
        (*(&raw const TSS)).privilege_stack_table[0]
    }
}
