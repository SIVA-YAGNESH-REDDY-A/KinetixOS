mod gpt;
mod iso;

use std::env;
use std::fs::{self, OpenOptions};
use std::io::{Cursor, Write};
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

            // Also write HELLO-WORLD.ELF if the name is HELLO-WORLD
            if filename == "HELLO-WORLD" {
                println!("[IMAGE BUILDER] Writing /KINETIX/APPS/HELLO-WORLD.ELF from {}", apath);
                let mut elf_file = apps_dir.create_file("HELLO-WORLD.ELF")?;
                elf_file.write_all(&abytes)?;
            }

            // Also stage other apps if present in release directory
            let sysinfo_path = "target/x86_64-unknown-none/release/sysinfo";
            if Path::new(sysinfo_path).exists() {
                println!("[IMAGE BUILDER] Writing /KINETIX/APPS/SYSINFO.ELF from {}", sysinfo_path);
                let mut sfile = apps_dir.create_file("SYSINFO.ELF")?;
                sfile.write_all(&fs::read(sysinfo_path)?)?;
            }
            let bench_path = "target/x86_64-unknown-none/release/bench";
            if Path::new(bench_path).exists() {
                println!("[IMAGE BUILDER] Writing /KINETIX/APPS/BENCH.ELF from {}", bench_path);
                let mut bfile = apps_dir.create_file("BENCH.ELF")?;
                bfile.write_all(&fs::read(bench_path)?)?;
            }
        }
    }

    // Copy BOOTFS.TAR if present
    let bootfs_tar_paths = [
        "artifacts/boot-images/bootfs.tar",
        "kernel/kinetix-kernel/bootfs.tar",
    ];
    for bpath in &bootfs_tar_paths {
        if Path::new(bpath).exists() {
            println!("[IMAGE BUILDER] Writing /KINETIX/BOOTFS.TAR from {}", bpath);
            let mut bfile = kinetix_dir.create_file("BOOTFS.TAR")?;
            let bbytes = fs::read(bpath)?;
            bfile.write_all(&bbytes)?;

            println!("[IMAGE BUILDER] Writing /BOOTFS.TAR from {}", bpath);
            let mut root_bfile = root.create_file("BOOTFS.TAR")?;
            root_bfile.write_all(&bbytes)?;
            break;
        }
    }

    println!("[IMAGE BUILDER] Successfully created bootable UEFI image: {}", output_img);

    // 5. Also build compact 34 MiB FAT32 ESP and package into UEFI Bootable ISO (.iso)
    let iso_path = Path::new(output_img)
        .with_extension("iso")
        .to_string_lossy()
        .into_owned();
    let esp_iso_size: usize = 16 * 1024 * 1024; // 16 MiB FAT16 UEFI ESP (32768 x 512B sectors)
    let mut esp_buf = vec![0u8; esp_iso_size];
    {
        let mut cursor = Cursor::new(&mut esp_buf[..]);
        let mut buf_stream = fscommon::BufStream::new(&mut cursor);
        format_volume(
            &mut buf_stream,
            FormatVolumeOptions::new()
                .volume_label(*b"KINETIX_OS ")
                .fat_type(fatfs::FatType::Fat16),
        )?;
        let iso_fs = FileSystem::new(buf_stream, FsOptions::new())?;
        let iso_root = iso_fs.root_dir();
        iso_root.create_dir("EFI")?;
        let efi_d = iso_root.open_dir("EFI")?;
        efi_d.create_dir("BOOT")?;
        let boot_d = efi_d.open_dir("BOOT")?;
        boot_d.create_file("BOOTX64.EFI")?.write_all(&fs::read(efi_path)?)?;

        iso_root.create_dir("KINETIX")?;
        let k_d = iso_root.open_dir("KINETIX")?;
        k_d.create_dir("KERNEL")?;
        k_d.create_dir("APPS")?;
        if let Some(kpath) = kernel_path {
            if Path::new(kpath).exists() {
                let kbytes = fs::read(kpath)?;
                k_d.open_dir("KERNEL")?.create_file("KINETIX-KERNEL.ELF")?.write_all(&kbytes)?;
                iso_root.create_file("KERNEL.ELF")?.write_all(&kbytes)?;
            }
        }
        if let Some(apath) = app_path {
            if Path::new(apath).exists() {
                let apps_d = k_d.open_dir("APPS")?;
                let abytes = fs::read(apath)?;
                apps_d.create_file("HELLO-WORLD.ELF")?.write_all(&abytes)?;
                if Path::new("target/x86_64-unknown-none/release/sysinfo").exists() {
                    apps_d.create_file("SYSINFO.ELF")?.write_all(&fs::read("target/x86_64-unknown-none/release/sysinfo")?)?;
                }
                if Path::new("target/x86_64-unknown-none/release/bench").exists() {
                    apps_d.create_file("BENCH.ELF")?.write_all(&fs::read("target/x86_64-unknown-none/release/bench")?)?;
                }
            }
        }
        for bpath in &bootfs_tar_paths {
            if Path::new(bpath).exists() {
                let bbytes = fs::read(bpath)?;
                k_d.create_file("BOOTFS.TAR")?.write_all(&bbytes)?;
                iso_root.create_file("BOOTFS.TAR")?.write_all(&bbytes)?;
                break;
            }
        }
    }
    iso::write_uefi_iso(&iso_path, &esp_buf)?;
    Ok(())
}
