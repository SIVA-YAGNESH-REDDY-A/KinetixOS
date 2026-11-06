//! Interrupt Descriptor Table (IDT) for x86_64.
//!
//! Provides the 256-entry gate table loaded into the CPU via `lidt`.

use super::gdt::{DOUBLE_FAULT_IST_INDEX, KERNEL_CODE_SELECTOR};
use spin::Mutex;

/// An entry in the 64-bit Interrupt Descriptor Table.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    type_attr: u8,
    offset_mid: u16,
    offset_high: u32,
    zero: u32,
}

impl IdtEntry {
    pub const fn empty() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attr: 0,
            offset_mid: 0,
            offset_high: 0,
            zero: 0,
        }
    }

    /// Configures an interrupt gate handler.
    ///
    /// `ist_index`: 0 = use current stack; 1..7 = use Interrupt Stack Table from TSS.
    pub fn set_handler(&mut self, handler_addr: u64, ist_index: u8) {
        self.offset_low = (handler_addr & 0xFFFF) as u16;
        self.selector = KERNEL_CODE_SELECTOR;
        self.ist = ist_index & 0x07;
        // 0x8E = Present (1), Ring 0 (00), 0, 64-bit Interrupt Gate (1110)
        self.type_attr = 0x8E;
        self.offset_mid = ((handler_addr >> 16) & 0xFFFF) as u16;
        self.offset_high = ((handler_addr >> 32) & 0xFFFF_FFFF) as u32;
        self.zero = 0;
    }
}

/// IDT pointer structure loaded by `lidt`.
#[repr(C, packed)]
struct IdtPointer {
    limit: u16,
    base: u64,
}

/// Complete 256-entry IDT.
#[repr(C, align(16))]
pub struct InterruptDescriptorTable {
    pub entries: [IdtEntry; 256],
}

impl InterruptDescriptorTable {
    pub const fn new() -> Self {
        Self {
            entries: [IdtEntry::empty(); 256],
        }
    }
}

pub static IDT: Mutex<InterruptDescriptorTable> = Mutex::new(InterruptDescriptorTable::new());

/// Sets up exception and interrupt gates and loads the IDT with `lidt`.
pub fn init() {
    let mut idt = IDT.lock();

    // Import assembly stubs declared in interrupts.rs
    extern "C" {
        fn isr0(); fn isr1(); fn isr2(); fn isr3(); fn isr4(); fn isr5(); fn isr6(); fn isr7();
        fn isr8(); fn isr9(); fn isr10(); fn isr11(); fn isr12(); fn isr13(); fn isr14(); fn isr15();
        fn isr16(); fn isr17(); fn isr18(); fn isr19(); fn isr20(); fn isr21(); fn isr22(); fn isr23();
        fn isr24(); fn isr25(); fn isr26(); fn isr27(); fn isr28(); fn isr29(); fn isr30(); fn isr31();
        fn irq0(); // Vector 32: PIT Timer
        fn irq1(); // Vector 33: Keyboard
        fn isr_default();
    }

    let isr_table: [unsafe extern "C" fn(); 32] = [
        isr0, isr1, isr2, isr3, isr4, isr5, isr6, isr7,
        isr8, isr9, isr10, isr11, isr12, isr13, isr14, isr15,
        isr16, isr17, isr18, isr19, isr20, isr21, isr22, isr23,
        isr24, isr25, isr26, isr27, isr28, isr29, isr30, isr31,
    ];

    // Configure CPU Exception Handlers (0..31)
    for (vec, isr) in isr_table.iter().enumerate() {
        let ist = if vec == 8 { DOUBLE_FAULT_IST_INDEX } else { 0 };
        idt.entries[vec].set_handler(*isr as *const () as usize as u64, ist);
    }

    // Configure Hardware IRQs
    idt.entries[32].set_handler(irq0 as *const () as usize as u64, 0); // IRQ 0: Timer
    idt.entries[33].set_handler(irq1 as *const () as usize as u64, 0); // IRQ 1: Keyboard

    // Fill remaining interrupt gates with default handler
    for vec in 34..256 {
        idt.entries[vec].set_handler(isr_default as *const () as usize as u64, 0);
    }

    // Load IDT
    let idt_ptr = IdtPointer {
        limit: (core::mem::size_of::<InterruptDescriptorTable>() - 1) as u16,
        base: idt.entries.as_ptr() as u64,
    };

    unsafe {
        core::arch::asm!("lidt [{}]", in(reg) &idt_ptr, options(readonly, nostack, preserves_flags));
    }
}
