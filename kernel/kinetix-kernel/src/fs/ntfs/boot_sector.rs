//! NTFS Boot Sector (VBR) Parser for Kinetix OS.
//!
//! Validates the 512-byte NTFS Volume Boot Record, extracts volume geometry,
//! cluster dimensions, Master File Table ($MFT) location, and volume capacity.

#[derive(Debug, Clone, Copy)]
pub struct NtfsBootSector {
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub total_sectors: u64,
    pub mft_cluster: u64,
    pub mft_mirr_cluster: u64,
    pub mft_record_size: u32,
    pub index_record_size: u32,
    pub serial_number: u64,
    pub cluster_size_bytes: u64,
}

impl NtfsBootSector {
    pub fn parse(buf: &[u8; 512]) -> Option<Self> {
        // 1. Check OEM ID "NTFS    " at offset 3
        if &buf[3..11] != b"NTFS    " {
            return None;
        }

        // 2. Check 0x55 0xAA boot signature at offset 510
        if buf[510] != 0x55 || buf[511] != 0xAA {
            return None;
        }

        let bytes_per_sector = u16::from_le_bytes([buf[11], buf[12]]);
        if bytes_per_sector != 512 && bytes_per_sector != 1024 && bytes_per_sector != 2048 && bytes_per_sector != 4096 {
            return None;
        }

        let sectors_per_cluster = buf[13];
        if sectors_per_cluster == 0 {
            return None;
        }

        let total_sectors = u64::from_le_bytes(buf[40..48].try_into().ok()?);
        let mft_cluster = u64::from_le_bytes(buf[48..56].try_into().ok()?);
        let mft_mirr_cluster = u64::from_le_bytes(buf[56..64].try_into().ok()?);

        // Byte 64: Clusters per MFT record.
        // If < 0, size is 2^|val| bytes (e.g., -10 -> 2^10 = 1024 bytes)
        let raw_mft_size = buf[64] as i8;
        let mft_record_size = if raw_mft_size < 0 {
            1u32 << (-raw_mft_size as u32)
        } else {
            (raw_mft_size as u32) * (sectors_per_cluster as u32) * (bytes_per_sector as u32)
        };

        let raw_idx_size = buf[68] as i8;
        let index_record_size = if raw_idx_size < 0 {
            1u32 << (-raw_idx_size as u32)
        } else {
            (raw_idx_size as u32) * (sectors_per_cluster as u32) * (bytes_per_sector as u32)
        };

        let serial_number = u64::from_le_bytes(buf[72..80].try_into().ok()?);
        let cluster_size_bytes = (bytes_per_sector as u64) * (sectors_per_cluster as u64);

        Some(Self {
            bytes_per_sector,
            sectors_per_cluster,
            total_sectors,
            mft_cluster,
            mft_mirr_cluster,
            mft_record_size,
            index_record_size,
            serial_number,
            cluster_size_bytes,
        })
    }
}
