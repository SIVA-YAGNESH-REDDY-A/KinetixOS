//! NTFS Master File Table (MFT) Record & Attribute Parser for Kinetix OS.
//!
//! Decodes 1024-byte MFT records, applies Update Sequence Array (USA) fixups,
//! and parses $STANDARD_INFORMATION, $FILE_NAME, $VOLUME_NAME, $DATA, and $INDEX_ROOT.

use alloc::string::String;
use alloc::vec::Vec;

pub const MFT_MAGIC: &[u8; 4] = b"FILE";
pub const ATTR_STANDARD_INFORMATION: u32 = 0x10;
pub const ATTR_ATTRIBUTE_LIST: u32        = 0x20;
pub const ATTR_FILE_NAME: u32             = 0x30;
pub const ATTR_VOLUME_NAME: u32           = 0x60;
pub const ATTR_VOLUME_INFORMATION: u32    = 0x70;
pub const ATTR_DATA: u32                  = 0x80;
pub const ATTR_INDEX_ROOT: u32            = 0x90;
pub const ATTR_INDEX_ALLOCATION: u32      = 0xA0;
pub const ATTR_BITMAP: u32                = 0xB0;
pub const ATTR_END: u32                   = 0xFFFF_FFFF;

#[derive(Debug, Clone)]
pub struct MftAttributeHeader {
    pub type_id: u32,
    pub length: u32,
    pub non_resident: bool,
    pub name_length: u8,
    pub name_offset: u16,
    pub flags: u16,
    pub attribute_id: u16,
    pub data_offset: usize, // Relative to start of attribute
    pub data_length: usize,
}

#[derive(Debug, Clone)]
pub struct ClusterRun {
    pub lcn: i64,
    pub count: u64,
}

/// Applies NTFS Update Sequence Array (USA) fixups to a 1024-byte MFT record.
pub fn apply_mft_fixups(record: &mut [u8; 1024]) -> bool {
    if &record[0..4] != MFT_MAGIC {
        return false;
    }

    let usa_offset = u16::from_le_bytes([record[4], record[5]]) as usize;
    let usa_count = u16::from_le_bytes([record[6], record[7]]) as usize;

    if usa_offset + usa_count * 2 > record.len() || usa_count < 2 {
        return false;
    }

    let fixup_key = [record[usa_offset], record[usa_offset + 1]];

    // Verify and apply fixups to 512-byte sectors (sectors 0 and 1)
    for i in 1..usa_count {
        let sector_end = i * 512 - 2;
        if record[sector_end] != fixup_key[0] || record[sector_end + 1] != fixup_key[1] {
            return false; // Fixup key mismatch -> corrupted sector
        }
        let saved_val = [record[usa_offset + i * 2], record[usa_offset + i * 2 + 1]];
        record[sector_end] = saved_val[0];
        record[sector_end + 1] = saved_val[1];
    }

    true
}

/// Parses raw attributes in an MFT record.
pub fn parse_attributes(record: &[u8]) -> Vec<(MftAttributeHeader, usize)> {
    let mut attrs = Vec::new();
    let first_attr_offset = u16::from_le_bytes([record[20], record[21]]) as usize;
    let mut offset = first_attr_offset;

    while offset + 16 <= record.len() {
        let type_id = u32::from_le_bytes(record[offset..offset + 4].try_into().unwrap_or([0xFF; 4]));
        if type_id == ATTR_END {
            break;
        }

        let length = u32::from_le_bytes(record[offset + 4..offset + 8].try_into().unwrap_or([0; 4]));
        if length < 16 || offset + length as usize > record.len() {
            break;
        }

        let non_resident = record[offset + 8] != 0;
        let name_length = record[offset + 9];
        let name_offset = u16::from_le_bytes([record[offset + 10], record[offset + 11]]);
        let flags = u16::from_le_bytes([record[offset + 12], record[offset + 13]]);
        let attribute_id = u16::from_le_bytes([record[offset + 14], record[offset + 15]]);

        let (data_offset, data_length) = if !non_resident {
            let val_len = u32::from_le_bytes(record[offset + 16..offset + 20].try_into().unwrap_or([0; 4])) as usize;
            let val_off = u16::from_le_bytes([record[offset + 20], record[offset + 21]]) as usize;
            (val_off, val_len)
        } else {
            let run_list_off = u16::from_le_bytes([record[offset + 32], record[offset + 33]]) as usize;
            let real_size = u64::from_le_bytes(record[offset + 48..offset + 56].try_into().unwrap_or([0; 8])) as usize;
            (run_list_off, real_size)
        };

        attrs.push((
            MftAttributeHeader {
                type_id,
                length,
                non_resident,
                name_length,
                name_offset,
                flags,
                attribute_id,
                data_offset,
                data_length,
            },
            offset,
        ));

        offset += length as usize;
    }

    attrs
}

/// Decodes non-resident run-lists into cluster runs.
pub fn decode_run_list(data: &[u8]) -> Vec<ClusterRun> {
    let mut runs = Vec::new();
    let mut idx = 0;
    let mut prev_lcn: i64 = 0;

    while idx < data.len() {
        let b = data[idx];
        if b == 0 {
            break;
        }
        idx += 1;

        let len_bytes = (b & 0x0F) as usize;
        let off_bytes = ((b >> 4) & 0x0F) as usize;

        if idx + len_bytes + off_bytes > data.len() {
            break;
        }

        let mut count: u64 = 0;
        for i in 0..len_bytes {
            count |= (data[idx + i] as u64) << (i * 8);
        }
        idx += len_bytes;

        let mut lcn_offset: i64 = 0;
        for i in 0..off_bytes {
            lcn_offset |= (data[idx + i] as i64) << (i * 8);
        }
        // Sign-extend offset if negative
        if off_bytes > 0 && (data[idx + off_bytes - 1] & 0x80) != 0 {
            for i in off_bytes..8 {
                lcn_offset |= (0xFF as i64) << (i * 8);
            }
        }
        idx += off_bytes;

        prev_lcn += lcn_offset;
        runs.push(ClusterRun {
            lcn: prev_lcn,
            count,
        });
    }

    runs
}

/// Extracts UTF-16LE string from an attribute slice.
pub fn decode_utf16_name(slice: &[u8]) -> String {
    let mut s = String::new();
    for chunk in slice.chunks_exact(2) {
        let c = u16::from_le_bytes([chunk[0], chunk[1]]);
        if c == 0 {
            break;
        }
        if let Some(ch) = char::from_u32(c as u32) {
            s.push(ch);
        }
    }
    s
}
