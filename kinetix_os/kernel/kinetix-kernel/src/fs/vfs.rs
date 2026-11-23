//! Virtual Filesystem (VFS) Abstraction & In-Memory RamFS for Kinetix OS.
//!
//! Provides unified path resolution, file metadata queries, directory listing,
//! and raw byte slice access across embedded and dynamically loaded archives.

use alloc::string::String;
use alloc::vec::Vec;
use super::ustar::{normalize_path, UstarArchive};

/// Basic file metadata descriptor.
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub name: String,
    pub path: String,
    pub size: usize,
    pub is_dir: bool,
    pub mode: u32,
}

/// Directory entry descriptor for directory listing operations.
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub size: usize,
    pub is_dir: bool,
}

/// Read-only in-memory RAM filesystem backed by POSIX ustar archives.
pub struct RamFs {
    embedded: UstarArchive<'static>,
    staged_module: Option<UstarArchive<'static>>,
}

impl RamFs {
    /// Creates a new `RamFs` initialized with the kernel's embedded bootfs
    /// and an optional external module archive.
    pub fn new(embedded_data: &'static [u8], staged_data: Option<&'static [u8]>) -> Self {
        Self {
            embedded: UstarArchive::new(embedded_data),
            staged_module: staged_data.map(UstarArchive::new),
        }
    }

    /// Tests if a file or directory exists at the specified path.
    pub fn exists(&self, path: &str) -> bool {
        self.stat(path).is_some()
    }

    /// Retrieves file metadata at the specified path.
    pub fn stat(&self, path: &str) -> Option<FileMetadata> {
        let norm = normalize_path(path);

        // Check staged module first (overlay priority)
        if let Some(ref staged) = self.staged_module {
            if let Some(entry) = staged.lookup(norm) {
                return Some(FileMetadata {
                    name: String::from(filename_of(entry.path)),
                    path: String::from(entry.path),
                    size: entry.size,
                    is_dir: entry.entry_type.is_dir(),
                    mode: entry.mode,
                });
            }
        }

        // Fallback to embedded bootfs
        if let Some(entry) = self.embedded.lookup(norm) {
            return Some(FileMetadata {
                name: String::from(filename_of(entry.path)),
                path: String::from(entry.path),
                size: entry.size,
                is_dir: entry.entry_type.is_dir(),
                mode: entry.mode,
            });
        }

        // Root directory check
        if norm.is_empty() {
            return Some(FileMetadata {
                name: String::from("/"),
                path: String::from("/"),
                size: 0,
                is_dir: true,
                mode: 0o755,
            });
        }

        None
    }

    /// Opens a file and returns a static slice to its contiguous byte content.
    pub fn open(&self, path: &str) -> Option<&'static [u8]> {
        let norm = normalize_path(path);

        // Check staged module first
        if let Some(ref staged) = self.staged_module {
            if let Some(entry) = staged.lookup(norm) {
                if entry.entry_type.is_file() {
                    return Some(entry.data);
                }
            }
        }

        // Fallback to embedded bootfs
        if let Some(entry) = self.embedded.lookup(norm) {
            if entry.entry_type.is_file() {
                return Some(entry.data);
            }
        }

        None
    }

    /// Lists directory entries within `dir_path`.
    pub fn list_dir(&self, dir_path: &str) -> Vec<DirEntry> {
        let mut results = Vec::new();
        let norm = normalize_path(dir_path);

        // Collect from embedded archive
        for entry in self.embedded.list_dir(norm) {
            let name = String::from(filename_of(entry.path));
            if !results.iter().any(|r: &DirEntry| r.name == name) {
                results.push(DirEntry {
                    name,
                    size: entry.size,
                    is_dir: entry.entry_type.is_dir(),
                });
            }
        }

        // Overlay entries from staged module if present
        if let Some(ref staged) = self.staged_module {
            for entry in staged.list_dir(norm) {
                let name = String::from(filename_of(entry.path));
                if let Some(existing) = results.iter_mut().find(|r| r.name == name) {
                    existing.size = entry.size;
                    existing.is_dir = entry.entry_type.is_dir();
                } else {
                    results.push(DirEntry {
                        name,
                        size: entry.size,
                        is_dir: entry.entry_type.is_dir(),
                    });
                }
            }
        }

        results
    }
}

/// Helper function to extract filename from a path.
fn filename_of(path: &str) -> &str {
    let norm = normalize_path(path);
    if let Some(pos) = norm.rfind('/') {
        &norm[pos + 1..]
    } else {
        norm
    }
}
