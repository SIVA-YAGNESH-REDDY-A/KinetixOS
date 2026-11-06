//! Pure Rust GUID Partition Table (GPT) and Protective MBR Generator
//!
//! Generates standard UEFI-compliant GPT structures without external C or dynamic library dependencies.

use std::fs::File;
use std::io::{Seek, SeekFrom, Write};

pub const SECTOR_SIZE: u64 = 512;
pub const ESP_START_LBA: u64 = 2048; // Standard 1 MiB alignment

/// UEFI EFI System Partition Type GUID: `C12A7328-F81F-11D2-BA4B-00A0C93EC93B`
/// Represented in UEFI mixed-endian format.
pub const EFI_SYSTEM_PARTITION_GUID: [u8; 16] = [
    0x28, 0x73, 0x2A, 0xC1, // Data1 (u32 little-endian)
    0x1F, 0xF8,             // Data2 (u16 little-endian)
    0xD2, 0x11,             // Data3 (u16 little-endian)
    0xBA, 0x4B, 0x00, 0xA0, 0xC9, 0x3E, 0xC9, 0x3B, // Data4 (big-endian)
];

/// Computes the IEEE 802.3 32-bit Cyclic Redundancy Check (CRC32).
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if (crc & 1) != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// Writes a standard Protective MBR at LBA 0.
pub fn write_protective_mbr(file: &mut File, total_sectors: u64) -> std::io::Result<()> {
    let mut mbr = [0u8; 512];

    // Partition Entry 1 starts at offset 446 (0x1BE)
    let p = &mut mbr[446..462];
    p[0] = 0x00; // Boot indicator (non-bootable)
    p[1] = 0x00; // Starting CHS
    p[2] = 0x02;
    p[3] = 0x00;
    p[4] = 0xEE; // Partition type: GPT Protective MBR
    p[5] = 0xFF; // Ending CHS
    p[6] = 0xFF;
    p[7] = 0xFF;

    // Starting LBA: 1 (little-endian u32)
    p[8..12].copy_from_slice(&1u32.to_le_bytes());

    // Size in LBA: total_sectors - 1 (capped at u32::MAX)
    let lba_count = (total_sectors - 1).min(u32::MAX as u64) as u32;
    p[12..16].copy_from_slice(&lba_count.to_le_bytes());

    // Boot signature at end of MBR
    mbr[510] = 0x55;
    mbr[511] = 0xAA;

    file.seek(SeekFrom::Start(0))?;
    file.write_all(&mbr)?;
    Ok(())
}

/// Generates and writes Primary and Backup GPT headers and partition entry arrays.
pub fn write_gpt(file: &mut File, total_sectors: u64) -> std::io::Result<()> {
    let last_lba = total_sectors - 1;
    let first_usable_lba = 34u64;
    let last_usable_lba = total_sectors - 34;

    // 1. Construct Partition Entries (128 entries * 128 bytes = 16,384 bytes = 32 sectors)
    let mut entries = [0u8; 128 * 128];

    // Entry 0: EFI System Partition (ESP)
    let entry0 = &mut entries[0..128];
    entry0[0..16].copy_from_slice(&EFI_SYSTEM_PARTITION_GUID);
    // Unique partition GUID (deterministic for reproducibility)
    let unique_guid: [u8; 16] = [
        0x50, 0x48, 0x41, 0x53, 0x45, 0x33, 0x2D, 0x4B,
        0x49, 0x4E, 0x45, 0x54, 0x49, 0x58, 0x4F, 0x53,
    ];
    entry0[16..32].copy_from_slice(&unique_guid);
    entry0[32..40].copy_from_slice(&ESP_START_LBA.to_le_bytes());
    entry0[40..48].copy_from_slice(&last_usable_lba.to_le_bytes());
    entry0[48..56].copy_from_slice(&0u64.to_le_bytes()); // Attributes

    // Name: "EFI System Partition" in UTF-16LE
    let name = "EFI System Partition";
    for (i, c) in name.encode_utf16().enumerate() {
        if i * 2 + 1 < 72 {
            entry0[56 + i * 2..56 + i * 2 + 2].copy_from_slice(&c.to_le_bytes());
        }
    }

    let entries_crc = crc32(&entries);

    // 2. Construct Primary GPT Header (at LBA 1)
    let mut primary_header = [0u8; 92];
    primary_header[0..8].copy_from_slice(b"EFI PART");
    primary_header[8..12].copy_from_slice(&0x00010000u32.to_le_bytes()); // Revision 1.0
    primary_header[12..16].copy_from_slice(&92u32.to_le_bytes()); // Header size
    primary_header[16..20].copy_from_slice(&0u32.to_le_bytes()); // CRC placeholder
    primary_header[20..24].copy_from_slice(&0u32.to_le_bytes()); // Reserved
    primary_header[24..32].copy_from_slice(&1u64.to_le_bytes()); // Current LBA (1)
    primary_header[32..40].copy_from_slice(&last_lba.to_le_bytes()); // Backup LBA
    primary_header[40..48].copy_from_slice(&first_usable_lba.to_le_bytes());
    primary_header[48..56].copy_from_slice(&last_usable_lba.to_le_bytes());
    // Disk GUID
    let disk_guid: [u8; 16] = [
        0x4B, 0x49, 0x4E, 0x45, 0x54, 0x49, 0x58, 0x2D,
        0x4F, 0x53, 0x2D, 0x44, 0x49, 0x53, 0x4B, 0x31,
    ];
    primary_header[56..72].copy_from_slice(&disk_guid);
    primary_header[72..80].copy_from_slice(&2u64.to_le_bytes()); // Partition entries starting LBA (2)
    primary_header[80..84].copy_from_slice(&128u32.to_le_bytes()); // Number of entries
    primary_header[84..88].copy_from_slice(&128u32.to_le_bytes()); // Size of each entry
    primary_header[88..92].copy_from_slice(&entries_crc.to_le_bytes());

    let primary_crc = crc32(&primary_header);
    primary_header[16..20].copy_from_slice(&primary_crc.to_le_bytes());

    // 3. Write Primary GPT (LBA 1) and Entries (LBA 2..33)
    file.seek(SeekFrom::Start(1 * SECTOR_SIZE))?;
    file.write_all(&primary_header)?;
    let padding = [0u8; 512 - 92];
    file.write_all(&padding)?;

    file.seek(SeekFrom::Start(2 * SECTOR_SIZE))?;
    file.write_all(&entries)?;

    // 4. Construct and Write Backup GPT (LBA last_lba) and Backup Entries (LBA last_lba - 32)
    let backup_entries_lba = last_lba - 32;
    let mut backup_header = primary_header;
    backup_header[16..20].copy_from_slice(&0u32.to_le_bytes());
    backup_header[24..32].copy_from_slice(&last_lba.to_le_bytes()); // Current LBA (backup)
    backup_header[32..40].copy_from_slice(&1u64.to_le_bytes()); // Backup LBA (primary)
    backup_header[72..80].copy_from_slice(&backup_entries_lba.to_le_bytes());

    let backup_crc = crc32(&backup_header);
    backup_header[16..20].copy_from_slice(&backup_crc.to_le_bytes());

    file.seek(SeekFrom::Start(backup_entries_lba * SECTOR_SIZE))?;
    file.write_all(&entries)?;

    file.seek(SeekFrom::Start(last_lba * SECTOR_SIZE))?;
    file.write_all(&backup_header)?;
    file.write_all(&padding)?;

    Ok(())
}
