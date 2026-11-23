//! Dual 8259 Programmable Interrupt Controller (PIC) Driver.
//!
//! Remaps legacy hardware IRQs 0..15 to interrupt vectors 32..47 (0x20..0x2F)
//! to avoid collisions with x86_64 CPU architecture exceptions (0..31).

use super::cpu::{inb, io_wait, outb};

const PIC1_COMMAND: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_COMMAND: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

const PIC_EOI: u8 = 0x20;

const ICW1_INIT: u8 = 0x10;
const ICW1_ICW4: u8 = 0x01;
const ICW4_8086: u8 = 0x01;

pub const PIC1_OFFSET: u8 = 32; // IRQ 0..7 mapped to vectors 32..39
pub const PIC2_OFFSET: u8 = 40; // IRQ 8..15 mapped to vectors 40..47

pub struct ChainedPics {
    offset1: u8,
    offset2: u8,
}

impl ChainedPics {
    pub const fn new(offset1: u8, offset2: u8) -> Self {
        Self { offset1, offset2 }
    }

    /// Remaps the PICs to specified vector offsets and unmasks IRQ 0 (timer) and IRQ 1 (keyboard).
    pub unsafe fn init(&self) {
        // Save existing interrupt masks
        let mask1 = inb(PIC1_DATA);
        let mask2 = inb(PIC2_DATA);

        // Start initialization sequence in cascade mode
        outb(PIC1_COMMAND, ICW1_INIT | ICW1_ICW4);
        io_wait();
        outb(PIC2_COMMAND, ICW1_INIT | ICW1_ICW4);
        io_wait();

        // ICW2: Vector offsets
        outb(PIC1_DATA, self.offset1);
        io_wait();
        outb(PIC2_DATA, self.offset2);
        io_wait();

        // ICW3: Tell Master PIC there is a Slave PIC at IRQ 2 (0000 0100 = 0x04)
        outb(PIC1_DATA, 4);
        io_wait();
        // Tell Slave PIC its cascade identity (0000 0010 = 2)
        outb(PIC2_DATA, 2);
        io_wait();

        // ICW4: Set 8086/88 (MCS-80/85) mode
        outb(PIC1_DATA, ICW4_8086);
        io_wait();
        outb(PIC2_DATA, ICW4_8086);
        io_wait();

        // Set masks: unmask IRQ 0 (timer) and IRQ 1 (keyboard), mask others initially
        // Bit 0 = IRQ0 (Timer), Bit 1 = IRQ1 (Keyboard), Bit 2 = Cascade to PIC2
        let master_mask = 0b1111_1000; // Unmask IRQ0 (Timer), IRQ1 (Keyboard), IRQ2 (Cascade)
        let slave_mask = 0b1111_1111;  // Mask all on slave for now

        outb(PIC1_DATA, master_mask);
        io_wait();
        outb(PIC2_DATA, slave_mask);
        io_wait();

        let _ = (mask1, mask2);
    }

    /// Sends an End of Interrupt (EOI) acknowledgment to the PIC.
    pub unsafe fn notify_end_of_interrupt(&self, interrupt_id: u8) {
        if interrupt_id >= self.offset2 && interrupt_id < self.offset2 + 8 {
            outb(PIC2_COMMAND, PIC_EOI);
            outb(PIC1_COMMAND, PIC_EOI);
        } else if interrupt_id >= self.offset1 && interrupt_id < self.offset1 + 8 {
            outb(PIC1_COMMAND, PIC_EOI);
        }
    }

    /// Unmasks a specific hardware IRQ line (0..15).
    pub unsafe fn unmask(&self, irq: u8) {
        if irq < 8 {
            let mask = inb(PIC1_DATA);
            outb(PIC1_DATA, mask & !(1 << irq));
            io_wait();
        } else if irq < 16 {
            let mask = inb(PIC2_DATA);
            outb(PIC2_DATA, mask & !(1 << (irq - 8)));
            io_wait();
            // Ensure Master PIC cascade line (IRQ 2) is unmasked
            let master_mask = inb(PIC1_DATA);
            outb(PIC1_DATA, master_mask & !(1 << 2));
            io_wait();
        }
    }

    /// Disables the 8259 PIC completely by masking all IRQs.
    #[allow(dead_code)]
    pub unsafe fn disable(&self) {
        outb(PIC1_DATA, 0xFF);
        io_wait();
        outb(PIC2_DATA, 0xFF);
        io_wait();
    }
}

pub static PICS: spin::Mutex<ChainedPics> = spin::Mutex::new(ChainedPics::new(PIC1_OFFSET, PIC2_OFFSET));
