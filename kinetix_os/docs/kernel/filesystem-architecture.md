# Kinetix OS Filesystem Architecture & In-Memory Boot RamFS

**Document**: Kernel Architecture Specification: In-Memory Boot Filesystem  
**Author**: Kinetix OS Systems Architecture Team  
**Status**: Implemented & Verified in VirtualBox 7.2 (Phase 13)  
**Date**: November 2026  

---

## 1. Executive Overview

Operating system bootstrap requires access to configuration files, system metadata, driver parameterizations, and initial native application binaries (`init`, shells, system diagnostics) before secondary storage controllers (AHCI SATA, NVMe, USB mass storage) are initialized. 

Kinetix OS solves this problem via an **In-Memory Boot Filesystem (RamFS)** backed by the standard **POSIX ustar (Unix Standard TAR)** archive format. The filesystem architecture features:
1. **Zero-Copy Memory Mapping**: Contiguous byte-slice referencing directly into RAM with zero intermediate heap copying.
2. **Dual-Source Unified VFS**: Seamlessly unifies compile-time embedded archives (`include_bytes!`) and bootloader-staged external modules (`BootInfo.modules`).
3. **Standard Metadata & Directory Trees**: Full path normalization, hierarchical directory listing (`list_dir`), file status queries (`stat`), and existence validation (`exists`).

---

## 2. POSIX ustar Archive Specification

The ustar format specifies a sequence of fixed-size 512-byte header blocks, each immediately followed by the file's data rounded up to the nearest 512-byte boundary.

### 2.1 512-Byte Header Structure

| Offset | Length | Field Name | Description | Type / Encoding |
| :--- | :--- | :--- | :--- | :--- |
| `0` | 100 | `name` | File path and name | ASCII string (null-terminated) |
| `100` | 8 | `mode` | File permissions | Octal ASCII (e.g., `0000666\0`) |
| `108` | 8 | `uid` | User numeric ID | Octal ASCII |
| `116` | 8 | `gid` | Group numeric ID | Octal ASCII |
| `124` | 12 | `size` | File size in bytes | Octal ASCII (e.g., `00000002237\0`) |
| `136` | 12 | `mtime` | Modification timestamp | Octal ASCII |
| `148` | 8 | `chksum` | Header block checksum | Octal ASCII (sum of all 512 bytes) |
| `156` | 1 | `typeflag` | File classification | ASCII char (`'0'` file, `'5'` dir) |
| `157` | 100 | `linkname` | Target link path | ASCII string |
| `257` | 6 | `magic` | POSIX ustar signature | `"ustar\0"` or `"ustar "` |
| `263` | 2 | `version` | POSIX version | `"00"` |
| `265` | 32 | `uname` | Owner user name | ASCII string |
| `297` | 32 | `gname` | Owner group name | ASCII string |
| `329` | 8 | `devmajor` | Device major number | Octal ASCII |
| `337` | 8 | `devminor` | Device minor number | Octal ASCII |
| `345` | 155 | `prefix` | Extended path prefix | ASCII string |
| `500` | 12 | `_pad` | Reserved zero padding | Binary zeroes |

### 2.2 Mathematical Checksum Verification
The checksum field (`chksum` at offset 148..156) is validated by treating all 8 bytes of `chksum` as ASCII spaces (`0x20`) and computing the unsigned byte sum across the entire 512-byte block:

$$\text{Calculated Checksum} = \sum_{i=0}^{511} \begin{cases} 0x20 & \text{if } 148 \le i < 156 \\ \text{header}[i] & \text{otherwise} \end{cases}$$

The archive terminates when encountering two consecutive 512-byte blocks filled completely with zeros (`0x00`).

---

## 3. Subsystem Architecture

```
+-------------------------------------------------------------------------+
|                       KINETIX KERNEL SUBSYSTEMS                         |
|   (main.rs, app loader, system call ABI, interactive shell commands)     |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
|                   VIRTUAL FILESYSTEM LAYER (fs/mod.rs)                  |
|    fs::open(), fs::read(), fs::stat(), fs::list_dir(), fs::exists()     |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
|                         RamFs DISPATCHER (vfs.rs)                       |
|   - Path normalization (/etc/os-release -> etc/os-release)              |
|   - Priority overlay: Staged Modules > Embedded BootFS                  |
+-------------------------------------------------------------------------+
                  |                                     |
                  v                                     v
+-----------------------------------+ +-----------------------------------+
|     EMBEDDED ARCHIVE (ustar.rs)   | |      STAGED MODULE ARCHIVE        |
|  - include_bytes!("../bootfs.tar")| |  - BootInfo.modules (BOOTFS.TAR)  |
|  - Guaranteed fallback in ROM/RAM | |  - Dynamically discovered at boot |
+-----------------------------------+ +-----------------------------------+
```

---

## 4. Boot RamFS Contents

The initial bundled archive (`bootfs.tar`) contains the canonical root filesystem hierarchy:

```
/
├── bin/
│   └── hello-world        (1,144 bytes, native freestanding ELF64 executable)
├── etc/
│   ├── os-release         (175 bytes, system metadata and version)
│   └── motd               (387 bytes, login welcome message of the day)
└── README.txt             (567 bytes, filesystem documentation)
```

---

## 5. Verification Results (VirtualBox 7.2 Live Boot)

```text
[FS] Initializing In-Memory Boot Filesystem (RamFS / ustar)...
  Embedded BootFS Span : 7680 bytes (15 blocks)
  Discovered staged module 'BOOTFS.TAR' at 0x7D483000 (7680 bytes)
  Root Directory ('/') populated with 3 top-level entries.
    [DIR ] bin              (0 bytes)
    [DIR ] etc              (0 bytes)
    [FILE] README.txt       (567 bytes)
[FS] In-memory boot filesystem initialized successfully.

[TEST] Running In-Memory Boot Filesystem (ustar) Verification Tests...
  [PASS] Root directory listing contains 3 entries.
  [PASS] stat('etc/os-release' -> 'os-release'): size = 175 bytes, mode = 0o666
  [PASS] verify_checksum() on first USTAR header block passed.
  [PASS] read('/etc/os-release') verified content: 'NAME="Kinetix OS"' found.
  [PASS] read('/etc/motd') verified: 'Welcome to Kinetix OS' found.
  [PASS] list_dir('/etc') discovered 2 child entries (os-release, motd).
  [PASS] read('/bin/hello-world'): Valid ELF64 binary detected (1144 bytes, magic: 0x7F 'ELF').
  [PASS] Non-existent path lookup safely returned None.
[TEST] All In-Memory Boot Filesystem Verification Tests PASSED!
```
