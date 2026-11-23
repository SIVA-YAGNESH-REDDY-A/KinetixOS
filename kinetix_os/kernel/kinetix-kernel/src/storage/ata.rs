//! Native ATA/IDE PIO Mode Block Device Driver for Kinetix OS.
//!
//! Directly drives physical and virtual ATA/IDE controllers via port I/O:
//! - Primary Bus (I/O: 0x1F0..0x1F7, Control: 0x3F6)
//! - Secondary Bus (I/O: 0x170..0x177, Control: 0x376)
//! - Master / Slave drive selection
//! - 28-bit & 48-bit LBA sector reading and writing

use alloc::vec::Vec;
use crate::arch::cpu::{inb, outb, insw, outsw, io_wait};
use super::{BlockDevice, StorageError};

const STATUS_BSY: u8  = 0x80;
const STATUS_DRDY: u8 = 0x40;
const STATUS_DF: u8   = 0x20;
const STATUS_DRQ: u8  = 0x08;
const STATUS_ERR: u8  = 0x01;

const CMD_READ_SECTORS: u8    = 0x20;
const CMD_READ_SECTORS_EXT: u8 = 0x24;
const CMD_WRITE_SECTORS: u8   = 0x30;
const CMD_WRITE_SECTORS_EXT: u8 = 0x34;
const CMD_IDENTIFY: u8        = 0xEC;
const CMD_FLUSH_CACHE: u8     = 0xE7;

#[derive(Clone)]
pub struct AtaDevice {
    pub name: &'static str,
    pub io_base: u16,
    pub ctrl_base: u16,
    pub is_slave: bool,
    pub sector_count: u64,
    pub model: [u8; 40],
    pub supports_lba48: bool,
}

impl AtaDevice {
    pub fn new(name: &'static str, io_base: u16, ctrl_base: u16, is_slave: bool) -> Self {
        Self {
            name,
            io_base,
            ctrl_base,
            is_slave,
            sector_count: 0,
            model: [b' '; 40],
            supports_lba48: false,
        }
    }

    pub fn model_name(&self) -> &str {
        core::str::from_utf8(&self.model).unwrap_or("ATA Hard Disk").trim()
    }

    /// Selects the drive (Master: 0xA0, Slave: 0xB0) on the bus.
    unsafe fn select_drive(&self) {
        let drive_sel = if self.is_slave { 0xB0 } else { 0xA0 };
        outb(self.io_base + 6, drive_sel);
        io_wait();
    }

    /// Waits for BSY bit to clear.
    unsafe fn wait_not_busy(&self) -> Result<u8, StorageError> {
        for _ in 0..100_000 {
            let status = inb(self.ctrl_base);
            if (status & STATUS_BSY) == 0 {
                return Ok(status);
            }
        }
        Err(StorageError::BusyTimeout)
    }

    /// Waits for DRQ bit to be set (data ready to transfer).
    unsafe fn wait_drq(&self) -> Result<(), StorageError> {
        for _ in 0..100_000 {
            let status = inb(self.ctrl_base);
            if (status & STATUS_ERR) != 0 || (status & STATUS_DF) != 0 {
                return Err(StorageError::IoError);
            }
            if (status & STATUS_DRQ) != 0 {
                return Ok(());
            }
        }
        Err(StorageError::BusyTimeout)
    }

    /// Issues ATA IDENTIFY command (0xEC) to discover disk parameters.
    pub fn identify(&mut self) -> Result<(), StorageError> {
        unsafe {
            self.select_drive();

            // Set sector count and LBA registers to 0
            outb(self.io_base + 2, 0);
            outb(self.io_base + 3, 0);
            outb(self.io_base + 4, 0);
            outb(self.io_base + 5, 0);

            // Send IDENTIFY command
            outb(self.io_base + 7, CMD_IDENTIFY);
            io_wait();

            let status = inb(self.io_base + 7);
            if status == 0 || status == 0xFF {
                return Err(StorageError::DeviceNotPresent);
            }

            self.wait_not_busy()?;

            // Check if this is an ATAPI device (LBA mid = 0x14, high = 0xEB)
            let mid = inb(self.io_base + 4);
            let high = inb(self.io_base + 5);
            if mid != 0 || high != 0 {
                return Err(StorageError::NotSupported); // ATAPI not currently handled
            }

            self.wait_drq()?;

            // Read 256 words (512 bytes) of IDENTIFY data
            let mut id_buf = [0u16; 256];
            insw(self.io_base, id_buf.as_mut_ptr(), 256);

            // Extract Model Name (Words 27..46, 40 ASCII chars with byte swap)
            for i in 0..20 {
                let word = id_buf[27 + i];
                self.model[i * 2] = (word >> 8) as u8;
                self.model[i * 2 + 1] = (word & 0xFF) as u8;
            }

            // Word 83: Bit 10 = LBA48 support
            let lba48 = (id_buf[83] & (1 << 10)) != 0;
            self.supports_lba48 = lba48;

            if lba48 {
                let s0 = id_buf[100] as u64;
                let s1 = id_buf[101] as u64;
                let s2 = id_buf[102] as u64;
                let s3 = id_buf[103] as u64;
                self.sector_count = s0 | (s1 << 16) | (s2 << 32) | (s3 << 48);
            } else {
                let s0 = id_buf[60] as u64;
                let s1 = id_buf[61] as u64;
                self.sector_count = s0 | (s1 << 16);
            }

            if self.sector_count == 0 {
                return Err(StorageError::DeviceNotPresent);
            }

            Ok(())
        }
    }
}

impl BlockDevice for AtaDevice {
    fn name(&self) -> &'static str {
        self.name
    }

    fn sector_count(&self) -> u64 {
        self.sector_count
    }

    fn read_sector(&mut self, lba: u64, buf: &mut [u8; 512]) -> Result<(), StorageError> {
        if lba >= self.sector_count && self.sector_count > 0 {
            return Err(StorageError::OutOfBounds);
        }

        unsafe {
            self.wait_not_busy()?;

            if self.supports_lba48 && lba >= 0x0FFF_FFFF {
                // 48-bit LBA Read
                let drive_sel = if self.is_slave { 0x50 } else { 0x40 };
                outb(self.io_base + 6, drive_sel);

                outb(self.io_base + 2, 0); // High sector count
                outb(self.io_base + 3, ((lba >> 24) & 0xFF) as u8);
                outb(self.io_base + 4, ((lba >> 32) & 0xFF) as u8);
                outb(self.io_base + 5, ((lba >> 40) & 0xFF) as u8);

                outb(self.io_base + 2, 1); // Low sector count
                outb(self.io_base + 3, (lba & 0xFF) as u8);
                outb(self.io_base + 4, ((lba >> 8) & 0xFF) as u8);
                outb(self.io_base + 5, ((lba >> 16) & 0xFF) as u8);

                outb(self.io_base + 7, CMD_READ_SECTORS_EXT);
            } else {
                // 28-bit LBA Read
                let drive_sel = 0xE0 | (if self.is_slave { 0x10 } else { 0x00 }) | (((lba >> 24) & 0x0F) as u8);
                outb(self.io_base + 6, drive_sel);
                outb(self.io_base + 2, 1); // 1 sector
                outb(self.io_base + 3, (lba & 0xFF) as u8);
                outb(self.io_base + 4, ((lba >> 8) & 0xFF) as u8);
                outb(self.io_base + 5, ((lba >> 16) & 0xFF) as u8);
                outb(self.io_base + 7, CMD_READ_SECTORS);
            }

            self.wait_not_busy()?;
            self.wait_drq()?;

            insw(self.io_base, buf.as_mut_ptr() as *mut u16, 256);
            Ok(())
        }
    }

    fn write_sector(&mut self, lba: u64, buf: &[u8; 512]) -> Result<(), StorageError> {
        if lba >= self.sector_count && self.sector_count > 0 {
            return Err(StorageError::OutOfBounds);
        }

        unsafe {
            self.wait_not_busy()?;

            if self.supports_lba48 && lba >= 0x0FFF_FFFF {
                let drive_sel = if self.is_slave { 0x50 } else { 0x40 };
                outb(self.io_base + 6, drive_sel);
                outb(self.io_base + 2, 0);
                outb(self.io_base + 3, ((lba >> 24) & 0xFF) as u8);
                outb(self.io_base + 4, ((lba >> 32) & 0xFF) as u8);
                outb(self.io_base + 5, ((lba >> 40) & 0xFF) as u8);

                outb(self.io_base + 2, 1);
                outb(self.io_base + 3, (lba & 0xFF) as u8);
                outb(self.io_base + 4, ((lba >> 8) & 0xFF) as u8);
                outb(self.io_base + 5, ((lba >> 16) & 0xFF) as u8);
                outb(self.io_base + 7, CMD_WRITE_SECTORS_EXT);
            } else {
                let drive_sel = 0xE0 | (if self.is_slave { 0x10 } else { 0x00 }) | (((lba >> 24) & 0x0F) as u8);
                outb(self.io_base + 6, drive_sel);
                outb(self.io_base + 2, 1);
                outb(self.io_base + 3, (lba & 0xFF) as u8);
                outb(self.io_base + 4, ((lba >> 8) & 0xFF) as u8);
                outb(self.io_base + 5, ((lba >> 16) & 0xFF) as u8);
                outb(self.io_base + 7, CMD_WRITE_SECTORS);
            }

            self.wait_not_busy()?;
            self.wait_drq()?;

            outsw(self.io_base, buf.as_ptr() as *const u16, 256);

            // Flush cache
            outb(self.io_base + 7, CMD_FLUSH_CACHE);
            self.wait_not_busy()?;

            Ok(())
        }
    }
}

/// Probes all standard 4 ATA bus drive channels:
/// 1. Primary Master (0x1F0, Master)
/// 2. Primary Slave (0x1F0, Slave)
/// 3. Secondary Master (0x170, Master)
/// 4. Secondary Slave (0x170, Slave)
pub fn probe_all() -> Vec<AtaDevice> {
    let mut drives = Vec::new();
    let candidates = [
        ("ata0-master", 0x1F0, 0x3F6, false),
        ("ata0-slave",  0x1F0, 0x3F6, true),
        ("ata1-master", 0x170, 0x376, false),
        ("ata1-slave",  0x170, 0x376, true),
    ];

    for (name, io, ctrl, slave) in candidates {
        let mut dev = AtaDevice::new(name, io, ctrl, slave);
        if dev.identify().is_ok() {
            drives.push(dev);
        }
    }

    drives
}
