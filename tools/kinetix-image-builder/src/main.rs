mod gpt;

use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use fatfs::{format_volume, FileSystem, FormatVolumeOptions, FsOptions};
use fscommon::StreamSlice;

const DISK_SIZE_BYTES: u64 = 64 * 1024 * 1024; // 64 MiB test image
const SECTOR_SIZE: u64 = gpt::SECTOR_SIZE;
const ESP_START_LBA: u64 = gpt::ESP_START_LBA;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: kinetix-image-builder <efi_path> <output_img> [kernel_path] [app_path]");
        std::process::exit(1);
    }

    let efi_path = &args[1];
    let output_img = &args[2];
    let kernel_path = args.get(3);
    let app_path = args.get(4);

    println!("[IMAGE BUILDER] Creating raw disk image: {}", output_img);

    // Ensure output directory exists
    if let Some(parent) = Path::new(output_img).parent() {
        fs::create_dir_all(parent)?;
    }

    // 1. Create raw disk file of fixed size
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(output_img)?;
    file.set_len(DISK_SIZE_BYTES)?;

    // 2. Write Protective MBR and GPT partition table
    let total_sectors = DISK_SIZE_BYTES / SECTOR_SIZE;
    let esp_end_lba = total_sectors - 34; // Reserved for backup GPT

    println!(
        "[IMAGE BUILDER] Writing Protective MBR & GPT (Total sectors: {}, ESP: LBA {}..{})",
        total_sectors, ESP_START_LBA, esp_end_lba
    );

    gpt::write_protective_mbr(&mut file, total_sectors)?;
    gpt::write_gpt(&mut file, total_sectors)?;

    // 3. Format ESP partition as FAT32
    let esp_start_byte = ESP_START_LBA * SECTOR_SIZE;
    let esp_length_bytes = (esp_end_lba - ESP_START_LBA + 1) * SECTOR_SIZE;

    println!(
        "[IMAGE BUILDER] Formatting FAT32 volume (Offset: 0x{:X}, Size: {} MiB)...",
        esp_start_byte,
        esp_length_bytes / (1024 * 1024)
    );

    let partition_slice = StreamSlice::new(&mut file, esp_start_byte, esp_start_byte + esp_length_bytes)?;
    let mut buf_stream = fscommon::BufStream::new(partition_slice);

    format_volume(
        &mut buf_stream,
        FormatVolumeOptions::new()
            .volume_label(*b"KINETIX_OS ")
            .fat_type(fatfs::FatType::Fat32),
    )?;

    // 4. Mount FAT32 filesystem and write directories & files
    println!("[IMAGE BUILDER] Populating directories and boot files...");
    let fs = FileSystem::new(buf_stream, FsOptions::new())?;
    let root = fs.root_dir();

    // Create /EFI/BOOT/
    root.create_dir("EFI")?;
    let efi_dir = root.open_dir("EFI")?;
    efi_dir.create_dir("BOOT")?;
    let boot_dir = efi_dir.open_dir("BOOT")?;

    // Copy BOOTX64.EFI
    println!("[IMAGE BUILDER] Writing /EFI/BOOT/BOOTX64.EFI from {}", efi_path);
    let mut bootx64_file = boot_dir.create_file("BOOTX64.EFI")?;
    let efi_bytes = fs::read(efi_path)?;
    bootx64_file.write_all(&efi_bytes)?;

    // Create /KINETIX/ directories
    root.create_dir("KINETIX")?;
    let kinetix_dir = root.open_dir("KINETIX")?;
    kinetix_dir.create_dir("KERNEL")?;
    kinetix_dir.create_dir("APPS")?;
    kinetix_dir.create_dir("CONFIG")?;

    // Copy kernel if supplied
    if let Some(kpath) = kernel_path {
        if Path::new(kpath).exists() {
            println!("[IMAGE BUILDER] Writing /KINETIX/KERNEL/KINETIX-KERNEL.ELF from {}", kpath);
            let kernel_dir = kinetix_dir.open_dir("KERNEL")?;
            let mut kfile = kernel_dir.create_file("KINETIX-KERNEL.ELF")?;
            let kbytes = fs::read(kpath)?;
            kfile.write_all(&kbytes)?;

            // Also place a root-level copy for UEFI firmware implementations without multi-level path resolution
            println!("[IMAGE BUILDER] Writing /KERNEL.ELF from {}", kpath);
            let mut root_kfile = root.create_file("KERNEL.ELF")?;
            root_kfile.write_all(&kbytes)?;
        }
    }

    // Copy application if supplied
    if let Some(apath) = app_path {
        if Path::new(apath).exists() {
            let filename = Path::new(apath).file_name().unwrap().to_str().unwrap().to_uppercase();
            println!("[IMAGE BUILDER] Writing /KINETIX/APPS/{} from {}", filename, apath);
            let apps_dir = kinetix_dir.open_dir("APPS")?;
            let mut afile = apps_dir.create_file(&filename)?;
            let abytes = fs::read(apath)?;
            afile.write_all(&abytes)?;
        }
    }

    println!("[IMAGE BUILDER] Successfully created bootable UEFI image: {}", output_img);
    Ok(())
}
