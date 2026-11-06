#![no_std]
#![no_main]

extern crate alloc;

mod loader;
mod serial;

use alloc::vec::Vec;
use core::panic::PanicInfo;
use kinetix_boot_protocol::{
    BootInfo, FramebufferInfo, MemoryMap, MemoryRegion, MemoryType as KinetixMemoryType,
    PixelFormat, KINETIX_BOOT_MAGIC, KINETIX_BOOT_VERSION, MAX_MEMORY_REGIONS,
};
use uefi::allocator::Allocator;
use uefi::boot::{self, AllocateType, MemoryType};
use uefi::cstr16;
use uefi::mem::memory_map::MemoryMap as UefiMemoryMapTrait;
use uefi::prelude::*;
use uefi::proto::console::gop::{GraphicsOutput, PixelFormat as UefiPixelFormat};
use uefi::proto::media::file::{File, FileAttribute, FileMode};

#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

static mut BOOT_INFO: BootInfo = BootInfo {
    magic: KINETIX_BOOT_MAGIC,
    version: KINETIX_BOOT_VERSION,
    _reserved: 0,
    memory_map: MemoryMap {
        regions: [MemoryRegion::EMPTY; MAX_MEMORY_REGIONS],
        region_count: 0,
    },
    framebuffer: FramebufferInfo {
        physical_address: 0,
        size: 0,
        width: 0,
        height: 0,
        stride: 0,
        pixel_format: PixelFormat::Unknown,
    },
    modules: kinetix_boot_protocol::ModuleList {
        modules: [kinetix_boot_protocol::ModuleDescriptor::EMPTY; kinetix_boot_protocol::MAX_MODULES],
        count: 0,
    },
    acpi_rsdp: 0,
};

#[entry]
fn main() -> Status {
    // 1. Initialize COM1 Serial Port (0x3F8)
    serial::init();
    serial_println!("================================================");
    serial_println!("                    KINETIX OS                  ");
    serial_println!("================================================");
    serial_println!("           Independent Rust Operating System    ");
    serial_println!("Architecture       : x86_64");
    serial_println!("Boot Firmware      : UEFI (64-bit)");
    serial_println!("Kernel             : Kinetix Kernel");
    serial_println!("Kernel Language    : Rust (core + no_std)");
    serial_println!("Bootloader         : Kinetix Bootloader");
    serial_println!("Status             : INITIALIZING (Phase 5 Handoff)");
    serial_println!("================================================");

    // 2. Display Banner on UEFI Console
    let _ = uefi::system::with_stdout(|stdout| {
        let _ = stdout.clear();
        let _ = stdout.output_string(cstr16!(
            "================================================\r\n"
        ));
        let _ = stdout.output_string(cstr16!(
            "                    KINETIX OS\r\n"
        ));
        let _ = stdout.output_string(cstr16!(
            "================================================\r\n\r\n"
        ));
        let _ = stdout.output_string(cstr16!(
            "           Independent Rust Operating System\r\n\r\n"
        ));
        let _ = stdout.output_string(cstr16!(
            "Architecture       : x86_64\r\n"
        ));
        let _ = stdout.output_string(cstr16!(
            "Boot Firmware      : UEFI\r\n"
        ));
        let _ = stdout.output_string(cstr16!(
            "Kernel             : Kinetix Kernel\r\n"
        ));
        let _ = stdout.output_string(cstr16!(
            "Kernel Language    : Rust\r\n"
        ));
        let _ = stdout.output_string(cstr16!(
            "Bootloader         : Kinetix Bootloader\r\n\r\n"
        ));
        let _ = stdout.output_string(cstr16!(
            "Status             : LOADING KERNEL...\r\n"
        ));
        let _ = stdout.output_string(cstr16!(
            "================================================\r\n"
        ));
    });

    // 3. Locate Boot Volume via SimpleFileSystem
    serial_println!("[BOOT] Opening boot filesystem...");
    let image_handle = boot::image_handle();
    let mut sfs = match boot::get_image_file_system(image_handle) {
        Ok(sfs) => sfs,
        Err(e) => {
            serial_println!("[BOOT ERROR] Failed to access boot volume: {:?}", e);
            return Status::LOAD_ERROR;
        }
    };

    let mut root_dir = match sfs.open_volume() {
        Ok(dir) => dir,
        Err(e) => {
            serial_println!("[BOOT ERROR] Failed to open root volume directory: {:?}", e);
            return Status::LOAD_ERROR;
        }
    };

    // 4. Read Kernel ELF File
    serial_println!("[BOOT] Locating kernel binary...");
    let file_handle = if let Ok(h) = root_dir.open(cstr16!("KERNEL.ELF"), FileMode::Read, FileAttribute::empty()) {
        serial_println!("[BOOT] Found /KERNEL.ELF in root directory.");
        h
    } else if let Ok(kdir_handle) = root_dir.open(cstr16!("KINETIX"), FileMode::Read, FileAttribute::empty()) {
        if let Some(mut kdir) = kdir_handle.into_directory() {
            if let Ok(kernel_dir_handle) = kdir.open(cstr16!("KERNEL"), FileMode::Read, FileAttribute::empty()) {
                if let Some(mut kernel_dir) = kernel_dir_handle.into_directory() {
                    match kernel_dir.open(cstr16!("KINETIX-KERNEL.ELF"), FileMode::Read, FileAttribute::empty()) {
                        Ok(h) => {
                            serial_println!("[BOOT] Found /KINETIX/KERNEL/KINETIX-KERNEL.ELF via directory traversal.");
                            h
                        }
                        Err(e) => {
                            serial_println!("[BOOT ERROR] Failed to open KINETIX-KERNEL.ELF in KERNEL dir: {:?}", e);
                            return Status::NOT_FOUND;
                        }
                    }
                } else {
                    serial_println!("[BOOT ERROR] KERNEL is not a directory!");
                    return Status::NOT_FOUND;
                }
            } else {
                serial_println!("[BOOT ERROR] Failed to open KERNEL directory!");
                return Status::NOT_FOUND;
            }
        } else {
            serial_println!("[BOOT ERROR] KINETIX is not a directory!");
            return Status::NOT_FOUND;
        }
    } else {
        serial_println!("[BOOT ERROR] Kernel binary not found!");
        return Status::NOT_FOUND;
    };

    let mut kernel_file = match file_handle.into_regular_file() {
        Some(file) => file,
        None => {
            serial_println!("[BOOT ERROR] Kernel file is not a regular file!");
            return Status::LOAD_ERROR;
        }
    };

    // Read kernel content into temporary buffer
    let mut kernel_buffer = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        match kernel_file.read(&mut chunk) {
            Ok(0) => break,
            Ok(bytes_read) => {
                kernel_buffer.extend_from_slice(&chunk[..bytes_read]);
            }
            Err(e) => {
                serial_println!("[BOOT ERROR] Reading kernel file failed: {:?}", e);
                return Status::LOAD_ERROR;
            }
        }
    }
    serial_println!("[BOOT] Read {} bytes of kernel ELF binary into RAM.", kernel_buffer.len());

    // 5. Parse and Validate ELF64 Headers
    let (entry_point, program_headers) = match loader::parse_and_validate_elf(&kernel_buffer) {
        Ok(res) => res,
        Err(e) => {
            serial_println!("[BOOT ERROR] Kernel ELF validation failed: {:?}", e);
            return Status::LOAD_ERROR;
        }
    };
    serial_println!("[BOOT] ELF validated. Entry point: 0x{:X}", entry_point);

    // 6. Allocate and Load PT_LOAD Segments
    for ph in program_headers {
        if ph.p_type == loader::PT_LOAD {
            let paddr = ph.p_paddr;
            let memsz = ph.p_memsz as usize;
            let filesz = ph.p_filesz as usize;
            let file_offset = ph.p_offset as usize;
            let page_count = (memsz + 4095) / 4096;

            serial_println!(
                "  PT_LOAD: 0x{:X}..0x{:X} (file: {} bytes, mem: {} bytes, pages: {})",
                paddr, paddr + memsz as u64, filesz, memsz, page_count
            );

            // Allocate physical memory pages at specified address
            let alloc_res = boot::allocate_pages(
                AllocateType::Address(paddr),
                MemoryType::LOADER_DATA,
                page_count,
            );

            if let Err(e) = alloc_res {
                serial_println!("[BOOT WARNING] Direct physical allocation at 0x{:X} returned: {:?}. Overwriting segment...", paddr, e);
            }

            // Copy segment data from file buffer
            unsafe {
                let dest = paddr as *mut u8;
                let src = kernel_buffer.as_ptr().add(file_offset);
                core::ptr::copy_nonoverlapping(src, dest, filesz);
                // Zero .bss tail
                if memsz > filesz {
                    core::ptr::write_bytes(dest.add(filesz), 0, memsz - filesz);
                }
            }
        }
    }

    // 7. Initialize Graphics Output Protocol (GOP)
    let gop_handle = boot::get_handle_for_protocol::<GraphicsOutput>().ok();
    if let Some(handle) = gop_handle {
        if let Ok(mut gop) = boot::open_protocol_exclusive::<GraphicsOutput>(handle) {
            let mode_info = gop.current_mode_info();
            let mut fb = gop.frame_buffer();
            let fb_ptr = fb.as_mut_ptr() as u64;
            let fb_size = fb.size();
            let (width, height) = mode_info.resolution();
            let stride = mode_info.stride() as u32;

            let pixel_format = match mode_info.pixel_format() {
                UefiPixelFormat::Rgb => PixelFormat::Rgb,
                UefiPixelFormat::Bgr => PixelFormat::Bgr,
                _ => PixelFormat::Unknown,
            };

            serial_println!(
                "[BOOT] GOP Framebuffer: {}x{} stride: {} format: {:?} at 0x{:X} ({} bytes)",
                width, height, stride, pixel_format, fb_ptr, fb_size
            );

            unsafe {
                BOOT_INFO.framebuffer = FramebufferInfo {
                    physical_address: fb_ptr,
                    size: fb_size,
                    width: width as u32,
                    height: height as u32,
                    stride,
                    pixel_format,
                };
            }
        }
    }

    // 8. Stage Native Applications (Stage A: Boot-Time Preloading)
    let app_path = cstr16!(r"KINETIX\APPS\HELLO-WORLD.ELF");
    if let Ok(file_handle) = root_dir.open(app_path, FileMode::Read, FileAttribute::empty()) {
        if let Some(mut afile) = file_handle.into_regular_file() {
            let mut app_buf = Vec::new();
            let mut chunk = [0u8; 8192];
            while let Ok(n) = afile.read(&mut chunk) {
                if n == 0 { break; }
                app_buf.extend_from_slice(&chunk[..n]);
            }
            if !app_buf.is_empty() {
                let pages = (app_buf.len() + 4095) / 4096;
                if let Ok(app_paddr) = boot::allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, pages) {
                    unsafe {
                        let ptr = app_paddr.as_ptr();
                        core::ptr::copy_nonoverlapping(app_buf.as_ptr(), ptr, app_buf.len());
                        let mut name = [0u8; 32];
                        let app_name = b"HELLO-WORLD.ELF";
                        name[..app_name.len()].copy_from_slice(app_name);
                        BOOT_INFO.modules.modules[0] = kinetix_boot_protocol::ModuleDescriptor {
                            physical_address: ptr as u64,
                            size: app_buf.len(),
                            name,
                        };
                        BOOT_INFO.modules.count = 1;
                        serial_println!("[BOOT] Staged module 'HELLO-WORLD.ELF' at 0x{:X} ({} bytes)", ptr as u64, app_buf.len());
                    }
                }
            }
        }
    }

    // Drop temporary vectors before exiting boot services
    drop(kernel_buffer);
    drop(root_dir);
    drop(sfs);

    // 9. Exit Boot Services (Point of No Return)
    serial_println!("[BOOT] Exiting UEFI Boot Services...");
    let memory_map_owned = unsafe { boot::exit_boot_services(None) };

    // 10. Populate Memory Map in BootInfo
    let mut region_count = 0;
    for desc in memory_map_owned.entries() {
        if region_count >= MAX_MEMORY_REGIONS {
            break;
        }

        let region_type = match desc.ty {
            MemoryType::CONVENTIONAL => KinetixMemoryType::Usable,
            MemoryType::LOADER_CODE | MemoryType::LOADER_DATA => KinetixMemoryType::BootloaderModule,
            MemoryType::ACPI_RECLAIM => KinetixMemoryType::AcpiReclaimable,
            MemoryType::ACPI_NON_VOLATILE => KinetixMemoryType::AcpiNvs,
            MemoryType::MMIO | MemoryType::MMIO_PORT_SPACE => KinetixMemoryType::Mmio,
            _ => KinetixMemoryType::Reserved,
        };

        unsafe {
            BOOT_INFO.memory_map.regions[region_count] = MemoryRegion {
                start: desc.phys_start,
                length: desc.page_count * 4096,
                region_type,
                _padding: 0,
            };
        }
        region_count += 1;
    }

    unsafe {
        BOOT_INFO.memory_map.region_count = region_count;
    }

    serial_println!("[BOOT] BootServices terminated. Transferring execution to Kinetix Kernel at 0x{:X}...", entry_point);

    // 11. Transfer Control to the Kinetix Kernel (System V AMD64 ABI)
    let kernel_entry: extern "sysv64" fn(&'static BootInfo) -> ! = unsafe {
        core::mem::transmute(entry_point)
    };

    unsafe {
        let boot_info_ref = &*(&raw const BOOT_INFO);
        kernel_entry(boot_info_ref);
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("[BOOTLOADER PANIC] {:?}", info);
    loop {}
}
