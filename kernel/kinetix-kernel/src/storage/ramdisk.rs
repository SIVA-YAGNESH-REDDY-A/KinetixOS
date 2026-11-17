//! In-Memory RAM Disk Block Device for Kinetix OS.
//!
//! Provides a standard 512-byte sector block device interface over contiguous memory buffers.

use super::{BlockDevice, StorageError};

pub struct RamDisk {
    pub name: &'static str,
    pub buffer: &'static mut [u8],
    pub sector_count: u64,
}

impl RamDisk {
    pub fn new(name: &'static str, buffer: &'static mut [u8]) -> Self {
        let sector_count = (buffer.len() / 512) as u64;
        Self {
            name,
            buffer,
            sector_count,
        }
    }
}

impl BlockDevice for RamDisk {
    fn name(&self) -> &'static str {
        self.name
    }

    fn sector_count(&self) -> u64 {
        self.sector_count
    }

    fn read_sector(&mut self, lba: u64, buf: &mut [u8; 512]) -> Result<(), StorageError> {
        let start = (lba * 512) as usize;
        let end = start + 512;
        if end > self.buffer.len() {
            return Err(StorageError::OutOfBounds);
        }
        buf.copy_from_slice(&self.buffer[start..end]);
        Ok(())
    }

    fn write_sector(&mut self, lba: u64, buf: &[u8; 512]) -> Result<(), StorageError> {
        let start = (lba * 512) as usize;
        let end = start + 512;
        if end > self.buffer.len() {
            return Err(StorageError::OutOfBounds);
        }
        self.buffer[start..end].copy_from_slice(buf);
        Ok(())
    }
}
