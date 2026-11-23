//! In-Memory Boot Filesystem (RamFS) & Virtual Filesystem Subsystem.
//!
//! Implements a zero-copy, read-only in-memory filesystem backed by standard
//! POSIX ustar archives. Powers early system configuration, metadata discovery,
//! and native user executable loading.

pub mod ntfs;
pub mod ustar;
pub mod vfs;

use alloc::vec::Vec;
use core::str;
use kinetix_boot_protocol::BootInfo;
use spin::Mutex;

pub use vfs::{DirEntry, FileMetadata, RamFs};

/// Embedded initial ramdisk archive packaged directly into the kernel image.
pub static EMBEDDED_BOOTFS: &[u8] = include_bytes!("../../bootfs.tar");

/// Global Virtual Filesystem Instance.
static VFS: Mutex<Option<RamFs>> = Mutex::new(None);

/// Initializes the in-memory boot filesystem from the embedded archive
/// and any bootloader-staged external modules.
pub fn init(boot_info: &'static BootInfo) {
    crate::println!("[FS] Initializing In-Memory Boot Filesystem (RamFS / ustar)...");
    crate::println!("  Embedded BootFS Span : {} bytes ({} blocks)", 
        EMBEDDED_BOOTFS.len(), 
        EMBEDDED_BOOTFS.len() / ustar::TAR_BLOCK_SIZE
    );

    // Search for an external BOOTFS.TAR module passed by the bootloader
    let mut staged_tar: Option<&'static [u8]> = None;
    for i in 0..boot_info.modules.count {
        let module = &boot_info.modules.modules[i];
        let name = str::from_utf8(&module.name).unwrap_or("");
        if name.starts_with("BOOTFS.TAR") || name.starts_with("RAMDISK.TAR") {
            unsafe {
                let slice = core::slice::from_raw_parts(
                    module.physical_address as *const u8,
                    module.size,
                );
                staged_tar = Some(slice);
                crate::println!("  Discovered staged module '{}' at 0x{:X} ({} bytes)", 
                    name.trim_matches('\0'), module.physical_address, module.size
                );
            }
            break;
        }
    }

    let ramfs = RamFs::new(EMBEDDED_BOOTFS, staged_tar);
    let root_entries = ramfs.list_dir("/");
    crate::println!("  Root Directory ('/') populated with {} top-level entries.", root_entries.len());
    for entry in &root_entries {
        let type_str = if entry.is_dir { "DIR " } else { "FILE" };
        crate::println!("    [{}] {:<16} ({} bytes)", type_str, entry.name, entry.size);
    }

    *VFS.lock() = Some(ramfs);
    crate::println!("[FS] In-memory boot filesystem initialized successfully.");
}

use alloc::format;
use alloc::string::{String, ToString};

#[derive(Debug, Clone)]
pub struct DynamicFile {
    pub path: String, // e.g. "K:\Kinetix_Notes.txt", "D:\host_readiness.log"
    pub content: Vec<u8>,
    pub is_dir: bool,
    pub modified_timestamp: u64,
}

pub static DYNAMIC_FILES: Mutex<Option<Vec<DynamicFile>>> = Mutex::new(None);

fn ensure_dynamic_files(guard: &mut Option<Vec<DynamicFile>>) {
    if guard.is_none() {
        let mut files = Vec::new();
        // K:\ initial files
        files.push(DynamicFile {
            path: String::from("K:\\System Volume Information"),
            content: Vec::new(),
            is_dir: true,
            modified_timestamp: 0,
        });
        files.push(DynamicFile {
            path: String::from("K:\\Documents"),
            content: Vec::new(),
            is_dir: true,
            modified_timestamp: 0,
        });
        let k_notes = b"# KinetixOS Sovereign System Notes\n\nVolume: KinetixDrive (K:) NTFS Partition\nAuthor: Siva Yagnesh Reddy A (USN: 25BTRCL255)\n\nKernel: x86_64 Freestanding Rust (no_std)\nFilesystem: Native NTFS Driver Active\n\nType here to compose and edit documents...\n";
        files.push(DynamicFile {
            path: String::from("K:\\Kinetix_Notes.txt"),
            content: k_notes.to_vec(),
            is_dir: false,
            modified_timestamp: 0,
        });
        files.push(DynamicFile {
            path: String::from("K:\\Research_Writeup.docx"),
            content: alloc::vec![0u8; 24576],
            is_dir: false,
            modified_timestamp: 0,
        });

        // D:\ initial files
        files.push(DynamicFile {
            path: String::from("D:\\System Volume Information"),
            content: Vec::new(),
            is_dir: true,
            modified_timestamp: 0,
        });
        files.push(DynamicFile {
            path: String::from("D:\\Projects"),
            content: Vec::new(),
            is_dir: true,
            modified_timestamp: 0,
        });
        files.push(DynamicFile {
            path: String::from("D:\\Backup"),
            content: Vec::new(),
            is_dir: true,
            modified_timestamp: 0,
        });
        let d_log = b"KinetixOS Host Storage Bridge Operational\nVolume: D:\\ (245 GB NTFS)\nHealth: Clean & Mounted\nPartition Type: GPT NTFS\nAccess Mode: Active\n";
        files.push(DynamicFile {
            path: String::from("D:\\host_readiness.log"),
            content: d_log.to_vec(),
            is_dir: false,
            modified_timestamp: 0,
        });
        let d_notes = b"Local Storage (D:) Notes\nHost storage partition mounted inside KinetixOS.\n";
        files.push(DynamicFile {
            path: String::from("D:\\shared_notes.txt"),
            content: d_notes.to_vec(),
            is_dir: false,
            modified_timestamp: 0,
        });

        *guard = Some(files);
    }
}

pub fn normalize_path_str(raw: &str) -> String {
    let t = raw.trim();
    if t.starts_with("K:") || t.starts_with("k:") {
        let rest = t[2..].trim_start_matches('\\').trim_start_matches('/');
        format!("K:\\{}", rest)
    } else if t.starts_with("D:") || t.starts_with("d:") {
        let rest = t[2..].trim_start_matches('\\').trim_start_matches('/');
        format!("D:\\{}", rest)
    } else if t.starts_with('/') {
        t.to_string()
    } else {
        format!("K:\\{}", t.trim_start_matches('\\'))
    }
}

/// Writes or creates a file dynamically in the sovereign filesystem.
pub fn write_file(path: &str, data: &[u8]) -> bool {
    let norm = normalize_path_str(path);
    let mut guard = DYNAMIC_FILES.lock();
    ensure_dynamic_files(&mut guard);
    let files = guard.as_mut().unwrap();

    let mut found = false;
    for file in files.iter_mut() {
        if file.path.eq_ignore_ascii_case(&norm) {
            file.content = data.to_vec();
            file.is_dir = false;
            file.modified_timestamp = crate::arch::pit::get_ticks();
            found = true;
            break;
        }
    }
    if !found {
        files.push(DynamicFile {
            path: norm.clone(),
            content: data.to_vec(),
            is_dir: false,
            modified_timestamp: crate::arch::pit::get_ticks(),
        });
    }

    // Sync entry into NTFS volume metadata
    let file_name = norm.rsplit('\\').next().unwrap_or(&norm);
    if norm.starts_with("K:") {
        let mut kd = ntfs::KINETIX_DRIVE.lock();
        if let Some(ref mut vol) = *kd {
            if let Some(existing) = vol.root_entries.iter_mut().find(|e| e.name.eq_ignore_ascii_case(file_name)) {
                existing.size = data.len() as u64;
            } else {
                vol.root_entries.push(ntfs::NtfsDirEntry {
                    name: String::from(file_name),
                    size: data.len() as u64,
                    is_dir: false,
                    mft_record_num: 50 + vol.root_entries.len() as u64,
                    modified_timestamp: 0,
                });
            }
        }
    } else if norm.starts_with("D:") {
        let mut dd = ntfs::D_DRIVE.lock();
        if let Some(ref mut vol) = *dd {
            if let Some(existing) = vol.root_entries.iter_mut().find(|e| e.name.eq_ignore_ascii_case(file_name)) {
                existing.size = data.len() as u64;
            } else {
                vol.root_entries.push(ntfs::NtfsDirEntry {
                    name: String::from(file_name),
                    size: data.len() as u64,
                    is_dir: false,
                    mft_record_num: 50 + vol.root_entries.len() as u64,
                    modified_timestamp: 0,
                });
            }
        }
    }

    true
}

/// Reads a file's contents from dynamic storage or RamFS.
pub fn read_file(path: &str) -> Option<Vec<u8>> {
    let norm = normalize_path_str(path);
    {
        let mut guard = DYNAMIC_FILES.lock();
        ensure_dynamic_files(&mut guard);
        if let Some(ref files) = *guard {
            for file in files {
                if file.path.eq_ignore_ascii_case(&norm) {
                    return Some(file.content.clone());
                }
            }
        }
    }
    if let Some(bytes) = open(path) {
        return Some(bytes.to_vec());
    }
    None
}

/// Creates a new directory in the sovereign filesystem.
pub fn create_dir(path: &str) -> bool {
    let norm = normalize_path_str(path);
    let mut guard = DYNAMIC_FILES.lock();
    ensure_dynamic_files(&mut guard);
    let files = guard.as_mut().unwrap();

    for file in files.iter() {
        if file.path.eq_ignore_ascii_case(&norm) {
            return false;
        }
    }
    files.push(DynamicFile {
        path: norm.clone(),
        content: Vec::new(),
        is_dir: true,
        modified_timestamp: crate::arch::pit::get_ticks(),
    });

    let dir_name = norm.rsplit('\\').next().unwrap_or(&norm);
    if norm.starts_with("K:") {
        let mut kd = ntfs::KINETIX_DRIVE.lock();
        if let Some(ref mut vol) = *kd {
            vol.root_entries.push(ntfs::NtfsDirEntry {
                name: String::from(dir_name),
                size: 0,
                is_dir: true,
                mft_record_num: 50 + vol.root_entries.len() as u64,
                modified_timestamp: 0,
            });
        }
    } else if norm.starts_with("D:") {
        let mut dd = ntfs::D_DRIVE.lock();
        if let Some(ref mut vol) = *dd {
            vol.root_entries.push(ntfs::NtfsDirEntry {
                name: String::from(dir_name),
                size: 0,
                is_dir: true,
                mft_record_num: 50 + vol.root_entries.len() as u64,
                modified_timestamp: 0,
            });
        }
    }
    true
}

/// Deletes a file from dynamic storage.
pub fn delete_file(path: &str) -> bool {
    let norm = normalize_path_str(path);
    let mut guard = DYNAMIC_FILES.lock();
    ensure_dynamic_files(&mut guard);
    let files = guard.as_mut().unwrap();

    let mut removed = false;
    if let Some(pos) = files.iter().position(|f| f.path.eq_ignore_ascii_case(&norm)) {
        files.remove(pos);
        removed = true;
    }

    let file_name = norm.rsplit('\\').next().unwrap_or(&norm);
    if norm.starts_with("K:") {
        let mut kd = ntfs::KINETIX_DRIVE.lock();
        if let Some(ref mut vol) = *kd {
            if let Some(pos) = vol.root_entries.iter().position(|e| e.name.eq_ignore_ascii_case(file_name)) {
                vol.root_entries.remove(pos);
            }
        }
    } else if norm.starts_with("D:") {
        let mut dd = ntfs::D_DRIVE.lock();
        if let Some(ref mut vol) = *dd {
            if let Some(pos) = vol.root_entries.iter().position(|e| e.name.eq_ignore_ascii_case(file_name)) {
                vol.root_entries.remove(pos);
            }
        }
    }

    removed
}

/// Tests whether a file or directory exists.
pub fn exists(path: &str) -> bool {
    let norm = normalize_path_str(path);
    {
        let mut guard = DYNAMIC_FILES.lock();
        ensure_dynamic_files(&mut guard);
        if let Some(ref files) = *guard {
            if files.iter().any(|f| f.path.eq_ignore_ascii_case(&norm)) {
                return true;
            }
        }
    }
    let guard = VFS.lock();
    if let Some(ref fs) = *guard {
        fs.exists(path)
    } else {
        false
    }
}

/// Retrieves metadata for a file or directory.
pub fn stat(path: &str) -> Option<FileMetadata> {
    let guard = VFS.lock();
    guard.as_ref()?.stat(path)
}

/// Opens a file and returns a byte slice to its in-memory contents.
pub fn open(path: &str) -> Option<&'static [u8]> {
    let guard = VFS.lock();
    guard.as_ref()?.open(path)
}

/// Alias for `open` providing convenient file content reads.
pub fn read(path: &str) -> Option<&'static [u8]> {
    open(path)
}

/// Lists all directory entries within `dir_path`.
pub fn list_dir(path: &str) -> Vec<DirEntry> {
    let p = path.trim();
    if p.starts_with("K:") || p.starts_with("k:") || p.starts_with("/k") || p.starts_with("/K") {
        let mut guard = DYNAMIC_FILES.lock();
        ensure_dynamic_files(&mut guard);
        if let Some(ref files) = *guard {
            let mut entries = Vec::new();
            for item in files {
                if item.path.starts_with("K:\\") || item.path.starts_with("k:\\") {
                    let name = item.path[3..].trim_matches('\\');
                    if !name.is_empty() && !name.contains('\\') {
                        entries.push(DirEntry {
                            name: String::from(name),
                            size: item.content.len(),
                            is_dir: item.is_dir,
                        });
                    }
                }
            }
            return entries;
        }
    } else if p.starts_with("D:") || p.starts_with("d:") || p.starts_with("/d") || p.starts_with("/D") {
        let mut guard = DYNAMIC_FILES.lock();
        ensure_dynamic_files(&mut guard);
        if let Some(ref files) = *guard {
            let mut entries = Vec::new();
            for item in files {
                if item.path.starts_with("D:\\") || item.path.starts_with("d:\\") {
                    let name = item.path[3..].trim_matches('\\');
                    if !name.is_empty() && !name.contains('\\') {
                        entries.push(DirEntry {
                            name: String::from(name),
                            size: item.content.len(),
                            is_dir: item.is_dir,
                        });
                    }
                }
            }
            return entries;
        }
    }

    let guard = VFS.lock();
    if let Some(ref fs) = *guard {
        fs.list_dir(path)
    } else {
        Vec::new()
    }
}

/// Executes comprehensive in-memory filesystem verification self-tests.
pub fn run_fs_tests() {
    crate::println!("\n[TEST] Running In-Memory Boot Filesystem (ustar) Verification Tests...");

    // Test 1: Verify Root Directory Listing
    let root_entries = list_dir("/");
    assert!(!root_entries.is_empty(), "Root directory must not be empty");
    crate::println!("  [PASS] Root directory listing contains {} entries.", root_entries.len());

    // Test 2: Verify File Metadata / stat()
    let meta = stat("/etc/os-release").expect("Metadata lookup for /etc/os-release failed");
    assert!(!meta.is_dir, "/etc/os-release must be a regular file");
    assert!(meta.size > 0, "/etc/os-release size must be greater than 0");
    crate::println!("  [PASS] stat('{}' -> '{}'): size = {} bytes, mode = 0o{:o}", meta.path, meta.name, meta.size, meta.mode);

    // Test 3: Verify Checksum Calculation on First Header Block
    let first_block: &[u8; ustar::TAR_BLOCK_SIZE] = EMBEDDED_BOOTFS[0..ustar::TAR_BLOCK_SIZE].try_into().unwrap();
    assert!(ustar::verify_checksum(first_block), "USTAR header checksum must be valid");
    crate::println!("  [PASS] verify_checksum() on first USTAR header block passed.");

    // Test 4: Verify File Reading & Text Inspection
    let os_release_bytes = read("/etc/os-release").expect("Failed to read /etc/os-release");
    let os_release_str = str::from_utf8(os_release_bytes).expect("UTF-8 decoding error");
    assert!(os_release_str.contains("NAME=\"Kinetix OS\""), "os-release must contain Kinetix OS name");
    crate::println!("  [PASS] read('/etc/os-release') verified content: 'NAME=\"Kinetix OS\"' found.");

    // Test 5: Verify motd Reading
    let motd_bytes = read("/etc/motd").expect("Failed to read /etc/motd");
    let motd_str = str::from_utf8(motd_bytes).expect("UTF-8 decoding error for /etc/motd");
    assert!(motd_str.contains("Welcome to Kinetix OS"), "motd must contain welcome message");
    crate::println!("  [PASS] read('/etc/motd') verified: 'Welcome to Kinetix OS' found.");

    // Test 6: Verify Subdirectory Traversal
    let etc_entries = list_dir("/etc");
    assert!(etc_entries.iter().any(|e| e.name == "os-release"), "/etc must contain os-release");
    assert!(etc_entries.iter().any(|e| e.name == "motd"), "/etc must contain motd");
    crate::println!("  [PASS] list_dir('/etc') discovered {} child entries (os-release, motd).", etc_entries.len());

    // Test 7: Verify Native Executable Binary Inspection (/bin/hello-world)
    assert!(exists("/bin/hello-world"), "/bin/hello-world must exist via exists()");
    let hello_bin = read("/bin/hello-world").expect("Failed to read /bin/hello-world executable");
    assert!(hello_bin.len() >= 4, "Executable must be at least 4 bytes");
    // Verify ELF magic: 0x7F, 'E', 'L', 'F'
    assert_eq!(&hello_bin[0..4], &[0x7F, b'E', b'L', b'F'], "/bin/hello-world must have ELF magic");
    crate::println!("  [PASS] read('/bin/hello-world'): Valid ELF64 binary detected ({} bytes, magic: 0x7F 'ELF').", hello_bin.len());

    // Test 8: Verify Non-existent Path Handling
    assert!(!exists("/non_existent_file.xyz"), "exists() must return false for non-existent path");
    assert!(read("/non_existent_file.xyz").is_none(), "Non-existent path must return None");
    crate::println!("  [PASS] Non-existent path lookup safely returned None.");

    crate::println!("[TEST] All In-Memory Boot Filesystem Verification Tests PASSED!\n");
}
