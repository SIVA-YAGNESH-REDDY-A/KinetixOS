//! Sovereign Block Storage Subsystem for Kinetix OS.
//!
//! Provides abstract block-device interfaces, ATA/IDE PIO drivers,
//! MBR/GPT partition parsers, and volume discovery.

pub mod ata;
pub mod partition;
pub mod ramdisk;

use alloc::vec::Vec;
use spin::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageError {
    DeviceNotPresent,
    BusyTimeout,
    IoError,
    OutOfBounds,
    WriteProtected,
    InvalidSector,
    NotSupported,
}

/// Abstract Block Device Trait representing a storage device capable of 512-byte sector I/O.
pub trait BlockDevice: Send + Sync {
    fn name(&self) -> &'static str;
    fn sector_count(&self) -> u64;
    fn read_sector(&mut self, lba: u64, buf: &mut [u8; 512]) -> Result<(), StorageError>;
    fn write_sector(&mut self, lba: u64, buf: &[u8; 512]) -> Result<(), StorageError>;
    fn is_read_only(&self) -> bool { false }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionFsType {
    Ntfs,
    Fat32,
    Unknown(u8),
}

#[derive(Debug, Clone)]
pub struct DiscoveredPartition {
    pub device_name: &'static str,
    pub part_num: usize,
    pub start_lba: u64,
    pub sector_count: u64,
    pub fs_type: PartitionFsType,
    pub is_ntfs: bool,
    pub is_kinetix_drive: bool,
    pub volume_label: [u8; 32],
}

pub struct StorageManager {
    pub ata_drives: Vec<ata::AtaDevice>,
    pub partitions: Vec<DiscoveredPartition>,
    pub kinetix_drive_mounted: bool,
    pub kinetix_drive_part_idx: Option<usize>,
}

impl StorageManager {
    pub const fn new() -> Self {
        Self {
            ata_drives: Vec::new(),
            partitions: Vec::new(),
            kinetix_drive_mounted: false,
            kinetix_drive_part_idx: None,
        }
    }
}

pub static STORAGE: Mutex<StorageManager> = Mutex::new(StorageManager::new());

/// Initializes the Block Storage subsystem:
/// 1. Probes Primary & Secondary ATA/IDE buses for physical and virtual hard disks.
/// 2. Discovers partition tables (MBR & GPT) on all detected block devices.
/// 3. Locates any partition formatted with NTFS and labeled "KinetixDrive".
pub fn init() {
    crate::println!("[STORAGE] Initializing Block Storage Subsystem & Controller Bus...");
    let mut mgr = STORAGE.lock();

    // 1. Probe ATA Devices on Primary (0x1F0) and Secondary (0x170) buses
    let detected_drives = ata::probe_all();
    crate::println!("[STORAGE] Discovered {} ATA block storage device(s).", detected_drives.len());
    for dev in &detected_drives {
        crate::println!(
            "  [{}] ATA Model: '{}' | Sectors: {} ({} MiB)",
            dev.name,
            dev.model_name(),
            dev.sector_count,
            (dev.sector_count * 512) / (1024 * 1024)
        );
    }
    mgr.ata_drives = detected_drives;

    // 2. Scan each drive for MBR and GPT partitions
    mgr.partitions.clear();
    for i in 0..mgr.ata_drives.len() {
        let drive = &mut mgr.ata_drives[i];
        let parts = partition::scan_partitions(drive);
        crate::println!("  [{}] Scanned partition table: {} partition(s) found.", drive.name, parts.len());
        for p in parts {
            crate::println!(
                "    Part {}: Start LBA: {} | Sectors: {} ({} MiB) | Type: {:?}",
                p.part_num,
                p.start_lba,
                p.sector_count,
                (p.sector_count * 512) / (1024 * 1024),
                p.fs_type
            );
            mgr.partitions.push(p);
        }
    }
}
