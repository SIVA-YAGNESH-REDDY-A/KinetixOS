//! In-Memory Boot Filesystem (RamFS) & Virtual Filesystem Subsystem.
//!
//! Implements a zero-copy, read-only in-memory filesystem backed by standard
//! POSIX ustar archives. Powers early system configuration, metadata discovery,
//! and native user executable loading.

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

/// Tests whether a file or directory exists.
pub fn exists(path: &str) -> bool {
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
