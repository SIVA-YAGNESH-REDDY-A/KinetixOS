//! MBR & GPT Partition Table Parser for Kinetix OS.
//!
//! Scans block storage devices for Master Boot Record (MBR) and GUID Partition Table (GPT)
//! structures, locating data volumes and inspecting boot sectors for NTFS identification.

use alloc::vec::Vec;
use super::{BlockDevice, DiscoveredPartition, PartitionFsType};

const MBR_SIGNATURE_OFFSET: usize = 510;
const GPT_HEADER_LBA: u64 = 1;
const GPT_SIGNATURE: &[u8; 8] = b"EFI PART";
const NTFS_OEM_ID: &[u8; 8] = b"NTFS    ";
const FAT32_OEM_ID: &[u8; 8] = b"FAT32   ";

/// Scans a block device for all MBR and GPT partitions.
pub fn scan_partitions<D: BlockDevice>(drive: &mut D) -> Vec<DiscoveredPartition> {
    let mut partitions = Vec::new();
    let mut sector0 = [0u8; 512];

    if drive.read_sector(0, &mut sector0).is_err() {
        return partitions;
    }

    // Verify MBR Signature 0x55 0xAA
    if sector0[MBR_SIGNATURE_OFFSET] != 0x55 || sector0[MBR_SIGNATURE_OFFSET + 1] != 0xAA {
        return partitions;
    }

    let mut is_gpt = false;

    // Check MBR partition entries (4 entries at 0x1BE, 0x1CE, 0x1DE, 0x1EE)
    for i in 0..4 {
        let offset = 0x1BE + i * 16;
        let part_type = sector0[offset + 4];
        if part_type == 0xEE {
            is_gpt = true;
            break;
        }
    }

    if is_gpt {
        // Parse GPT Header at LBA 1
        let mut gpt_header = [0u8; 512];
        if drive.read_sector(GPT_HEADER_LBA, &mut gpt_header).is_ok() {
            if &gpt_header[0..8] == GPT_SIGNATURE {
                let part_entries_lba = u64::from_le_bytes(gpt_header[72..80].try_into().unwrap_or([0; 8]));
                let num_part_entries = u32::from_le_bytes(gpt_header[80..84].try_into().unwrap_or([0; 4])) as usize;
                let entry_size = u32::from_le_bytes(gpt_header[84..88].try_into().unwrap_or([0; 4])) as usize;

                if entry_size >= 128 && num_part_entries > 0 && part_entries_lba > 0 {
                    let entries_per_sector = 512 / entry_size;
                    let sectors_to_read = (num_part_entries + entries_per_sector - 1) / entries_per_sector;
                    let max_sectors = sectors_to_read.min(32); // Limit to first 32 sectors

                    let mut part_idx = 0;
                    for sec in 0..max_sectors {
                        let mut sec_buf = [0u8; 512];
                        if drive.read_sector(part_entries_lba + sec as u64, &mut sec_buf).is_err() {
                            break;
                        }

                        for e in 0..entries_per_sector {
                            if part_idx >= num_part_entries {
                                break;
                            }
                            let e_off = e * entry_size;
                            // Check if Type GUID is non-zero
                            let is_empty = sec_buf[e_off..e_off + 16].iter().all(|&b| b == 0);
                            if !is_empty {
                                let start_lba = u64::from_le_bytes(sec_buf[e_off + 32..e_off + 40].try_into().unwrap_or([0; 8]));
                                let end_lba = u64::from_le_bytes(sec_buf[e_off + 40..e_off + 48].try_into().unwrap_or([0; 8]));
                                if end_lba >= start_lba && start_lba > 0 {
                                    let sector_count = end_lba - start_lba + 1;

                                    // Extract partition name (UTF-16LE, 36 chars = 72 bytes at 0x38..0x78)
                                    let mut label = [0u8; 32];
                                    let name_bytes = &sec_buf[e_off + 56..e_off + 128];
                                    let mut l_idx = 0;
                                    for ch in 0..36 {
                                        let c = u16::from_le_bytes([name_bytes[ch * 2], name_bytes[ch * 2 + 1]]);
                                        if c == 0 || l_idx >= 31 {
                                            break;
                                        }
                                        label[l_idx] = (c & 0x7F) as u8;
                                        l_idx += 1;
                                    }

                                    let (fs_type, is_ntfs) = inspect_partition_fs(drive, start_lba);
                                    let label_str = core::str::from_utf8(&label).unwrap_or("");
                                    let is_kinetix = is_ntfs && (label_str.contains("Kinetix") || label_str.contains("KINETIX"));

                                    partitions.push(DiscoveredPartition {
                                        device_name: drive.name(),
                                        part_num: part_idx + 1,
                                        start_lba,
                                        sector_count,
                                        fs_type,
                                        is_ntfs,
                                        is_kinetix_drive: is_kinetix,
                                        volume_label: label,
                                    });
                                }
                            }
                            part_idx += 1;
                        }
                    }
                }
            }
        }
    } else {
        // Standard MBR Partition parsing
        for i in 0..4 {
            let offset = 0x1BE + i * 16;
            let part_type = sector0[offset + 4];
            if part_type == 0 {
                continue;
            }

            let start_lba = u32::from_le_bytes(sector0[offset + 8..offset + 12].try_into().unwrap_or([0; 4])) as u64;
            let sector_count = u32::from_le_bytes(sector0[offset + 12..offset + 16].try_into().unwrap_or([0; 4])) as u64;

            if start_lba > 0 && sector_count > 0 {
                let (fs_type, is_ntfs) = inspect_partition_fs(drive, start_lba);

                partitions.push(DiscoveredPartition {
                    device_name: drive.name(),
                    part_num: i + 1,
                    start_lba,
                    sector_count,
                    fs_type,
                    is_ntfs,
                    is_kinetix_drive: is_ntfs, // Checked further by NTFS volume header
                    volume_label: *b"KinetixDrive\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
                });
            }
        }
    }

    partitions
}

/// Reads Sector 0 of a partition to determine its filesystem type (NTFS or FAT32).
fn inspect_partition_fs<D: BlockDevice>(drive: &mut D, start_lba: u64) -> (PartitionFsType, bool) {
    let mut part_boot = [0u8; 512];
    if drive.read_sector(start_lba, &mut part_boot).is_ok() {
        if &part_boot[3..11] == NTFS_OEM_ID {
            return (PartitionFsType::Ntfs, true);
        }
        if &part_boot[0x52..0x5A] == FAT32_OEM_ID || &part_boot[3..11] == b"MSWIN4.1" {
            return (PartitionFsType::Fat32, false);
        }
    }
    (PartitionFsType::Unknown(0), false)
}
