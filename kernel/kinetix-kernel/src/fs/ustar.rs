//! POSIX ustar (Unix Standard TAR) Archive Parser.
//!
//! Provides a safe, zero-copy parser for contiguous memory-mapped ustar archives
//! used by the Kinetix OS in-memory boot filesystem (bootfs).

use alloc::vec::Vec;

/// Standard TAR block size (512 bytes).
pub const TAR_BLOCK_SIZE: usize = 512;

/// POSIX ustar file entry type flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UstarType {
    RegularFile,
    HardLink,
    SymLink,
    CharacterDevice,
    BlockDevice,
    Directory,
    Fifo,
    Other(u8),
}

impl UstarType {
    pub fn from_flag(byte: u8) -> Self {
        match byte {
            b'0' | 0 => UstarType::RegularFile,
            b'1' => UstarType::HardLink,
            b'2' => UstarType::SymLink,
            b'3' => UstarType::CharacterDevice,
            b'4' => UstarType::BlockDevice,
            b'5' => UstarType::Directory,
            b'6' => UstarType::Fifo,
            other => UstarType::Other(other),
        }
    }

    pub fn is_dir(&self) -> bool {
        matches!(self, UstarType::Directory)
    }

    pub fn is_file(&self) -> bool {
        matches!(self, UstarType::RegularFile)
    }
}

/// A parsed zero-copy entry referencing an item within a ustar archive.
#[derive(Debug, Clone)]
pub struct UstarEntry<'a> {
    pub path: &'a str,
    pub size: usize,
    pub mode: u32,
    pub entry_type: UstarType,
    pub data: &'a [u8],
}

/// Parses an ASCII octal string into an integer.
pub fn parse_octal(bytes: &[u8]) -> Option<u64> {
    let mut val: u64 = 0;
    let mut found_digit = false;

    for &b in bytes {
        if b == 0 || b == b' ' {
            if found_digit {
                break;
            } else {
                continue;
            }
        }
        if (b'0'..=b'7').contains(&b) {
            found_digit = true;
            val = val.checked_mul(8)?.checked_add((b - b'0') as u64)?;
        } else {
            return None;
        }
    }

    if found_digit {
        Some(val)
    } else {
        Some(0)
    }
}

/// Verifies the checksum of a 512-byte header block.
pub fn verify_checksum(header: &[u8; TAR_BLOCK_SIZE]) -> bool {
    let recorded_chksum = match parse_octal(&header[148..156]) {
        Some(v) => v,
        None => return false,
    };

    let mut calc_chksum: u64 = 0;
    for (i, &b) in header.iter().enumerate() {
        if (148..156).contains(&i) {
            calc_chksum += b' ' as u64; // Treat checksum field as 8 ASCII spaces
        } else {
            calc_chksum += b as u64;
        }
    }

    calc_chksum == recorded_chksum
}

/// Extracts a null-terminated string slice from a byte slice.
fn extract_str(bytes: &[u8]) -> &str {
    let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    core::str::from_utf8(&bytes[..len]).unwrap_or("")
}

/// Zero-copy POSIX ustar archive wrapper over an in-memory byte slice.
pub struct UstarArchive<'a> {
    data: &'a [u8],
}

impl<'a> UstarArchive<'a> {
    /// Creates a new `UstarArchive` from a contiguous byte slice.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Returns an iterator over all valid entries in the archive.
    pub fn entries(&self) -> UstarIter<'a> {
        UstarIter {
            data: self.data,
            offset: 0,
        }
    }

    /// Looks up a file or directory entry by path.
    /// Leading slashes and trailing slashes are normalized.
    pub fn lookup(&self, target_path: &str) -> Option<UstarEntry<'a>> {
        let norm_target = normalize_path(target_path);

        for entry in self.entries() {
            let norm_entry = normalize_path(entry.path);
            if norm_entry == norm_target {
                return Some(entry);
            }
        }
        None
    }

    /// Lists entries immediately contained within `dir_path`.
    pub fn list_dir(&self, dir_path: &str) -> Vec<UstarEntry<'a>> {
        let norm_dir = normalize_path(dir_path);
        let mut results = Vec::new();

        for entry in self.entries() {
            let norm_entry = normalize_path(entry.path);
            if norm_entry == norm_dir {
                continue; // Skip the directory itself
            }

            if norm_dir.is_empty() {
                // Root listing: accept items with no '/'
                if !norm_entry.contains('/') {
                    results.push(entry);
                } else {
                    let first_slash = norm_entry.find('/').unwrap();
                    let top_dir = &norm_entry[..first_slash];
                    // Add directory once
                    if !results.iter().any(|r| normalize_path(r.path) == top_dir) {
                        results.push(entry);
                    }
                }
            } else if norm_entry.starts_with(norm_dir) && norm_entry.len() > norm_dir.len() {
                let remainder = &norm_entry[norm_dir.len()..];
                if remainder.starts_with('/') {
                    let sub = &remainder[1..];
                    if !sub.contains('/') || (sub.ends_with('/') && sub.matches('/').count() == 1) {
                        results.push(entry);
                    }
                }
            }
        }
        results
    }
}

/// Normalizes a path by removing leading and trailing slashes.
pub fn normalize_path(path: &str) -> &str {
    let mut p = path;
    while p.starts_with('/') {
        p = &p[1..];
    }
    while p.ends_with('/') {
        p = &p[..p.len() - 1];
    }
    p
}

/// Iterator over entries in a ustar archive.
pub struct UstarIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for UstarIter<'a> {
    type Item = UstarEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.offset + TAR_BLOCK_SIZE <= self.data.len() {
            let header_slice = &self.data[self.offset..self.offset + TAR_BLOCK_SIZE];
            let header_block: &[u8; TAR_BLOCK_SIZE] = header_slice.try_into().ok()?;

            // Check if block is all zeros (end-of-archive marker)
            if header_block.iter().all(|&b| b == 0) {
                return None;
            }

            // Check magic: must contain "ustar"
            let magic = extract_str(&header_block[257..263]);
            if !magic.starts_with("ustar") {
                // Skip invalid block
                self.offset += TAR_BLOCK_SIZE;
                continue;
            }

            let path = extract_str(&header_block[0..100]);
            let size = parse_octal(&header_block[124..136]).unwrap_or(0) as usize;
            let mode = parse_octal(&header_block[100..108]).unwrap_or(0) as u32;
            let typeflag = header_block[156];
            let entry_type = UstarType::from_flag(typeflag);

            let data_offset = self.offset + TAR_BLOCK_SIZE;
            let data_end = data_offset + size;

            if data_end > self.data.len() {
                return None;
            }

            let data = &self.data[data_offset..data_end];

            // Next block starts at 512-byte aligned boundary
            let padded_size = (size + TAR_BLOCK_SIZE - 1) & !(TAR_BLOCK_SIZE - 1);
            self.offset = data_offset + padded_size;

            return Some(UstarEntry {
                path,
                size,
                mode,
                entry_type,
                data,
            });
        }
        None
    }
}
