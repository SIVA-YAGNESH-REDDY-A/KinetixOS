//! UEFI El Torito ISO-9660 Bootable Image Generator for Kinetix OS.
//!
//! Packages the FAT32 EFI System Partition (`efiboot.img`) into a standard
//! ISO-9660 + El Torito (Platform ID 0xEF) bootable `.iso` binary.

use std::fs::OpenOptions;
use std::io::Write;

const ISO_SECTOR: usize = 2048;

fn write_both_u16(buf: &mut [u8], offset: usize, val: u16) {
    buf[offset..offset + 2].copy_from_slice(&val.to_le_bytes());
    buf[offset + 2..offset + 4].copy_from_slice(&val.to_be_bytes());
}

fn write_both_u32(buf: &mut [u8], offset: usize, val: u32) {
    buf[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
    buf[offset + 4..offset + 8].copy_from_slice(&val.to_be_bytes());
}

fn fill_dir_record(
    buf: &mut [u8],
    offset: usize,
    lba: u32,
    size: u32,
    flags: u8,
    name: &[u8],
) -> usize {
    let rec_len = (33 + name.len() + if (33 + name.len()) % 2 == 1 { 1 } else { 0 }) as u8;
    buf[offset] = rec_len;
    buf[offset + 1] = 0; // Extended attribute record length
    write_both_u32(buf, offset + 2, lba);
    write_both_u32(buf, offset + 10, size);
    // Date/Time (2026-11-16 16:00:00)
    buf[offset + 18] = 126; // Years since 1900
    buf[offset + 19] = 11;
    buf[offset + 20] = 16;
    buf[offset + 21] = 16;
    buf[offset + 22] = 0;
    buf[offset + 23] = 0;
    buf[offset + 24] = 0;
    buf[offset + 25] = flags;
    buf[offset + 26] = 0;
    buf[offset + 27] = 0;
    write_both_u16(buf, offset + 28, 1); // Volume sequence number
    buf[offset + 32] = name.len() as u8;
    buf[offset + 33..offset + 33 + name.len()].copy_from_slice(name);
    rec_len as usize
}

/// Writes a complete UEFI-bootable ISO-9660 image containing `efiboot_img`.
pub fn write_uefi_iso(iso_path: &str, efiboot_img: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let efi_sectors = (efiboot_img.len() + ISO_SECTOR - 1) / ISO_SECTOR;
    let efi_start_lba: u32 = 23;
    let total_iso_sectors: u32 = efi_start_lba + (efi_sectors as u32);

    let mut out = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(iso_path)?;

    // 1. Sectors 0..15: System Area (32 KiB zeros)
    let zeros = [0u8; ISO_SECTOR];
    for _ in 0..16 {
        out.write_all(&zeros)?;
    }

    // 2. Sector 16: Primary Volume Descriptor (PVD)
    let mut pvd = [0u8; ISO_SECTOR];
    pvd[0] = 0x01; // Primary Volume Descriptor
    pvd[1..6].copy_from_slice(b"CD001");
    pvd[6] = 0x01; // Version
    pvd[8..40].fill(b' ');
    pvd[8..18].copy_from_slice(b"KINETIX_OS");
    pvd[40..72].fill(b' ');
    pvd[40..50].copy_from_slice(b"KINETIX_OS");
    write_both_u32(&mut pvd, 80, total_iso_sectors);
    write_both_u16(&mut pvd, 120, 1); // Volume Set Size
    write_both_u16(&mut pvd, 124, 1); // Volume Sequence Number
    write_both_u16(&mut pvd, 128, ISO_SECTOR as u16); // Logical Block Size
    write_both_u32(&mut pvd, 132, 10); // Path Table Size
    pvd[140..144].copy_from_slice(&20u32.to_le_bytes()); // Type-L Path Table LBA
    pvd[148..152].copy_from_slice(&21u32.to_be_bytes()); // Type-M Path Table LBA
    fill_dir_record(&mut pvd, 156, 22, ISO_SECTOR as u32, 0x02, &[0x00]); // Root dir record
    pvd[881] = 0x01; // File Structure Version
    out.write_all(&pvd)?;

    // 3. Sector 17: El Torito Boot Record Volume Descriptor
    let mut brvd = [0u8; ISO_SECTOR];
    brvd[0] = 0x00; // Boot Record
    brvd[1..6].copy_from_slice(b"CD001");
    brvd[6] = 0x01;
    brvd[7..30].copy_from_slice(b"EL TORITO SPECIFICATION");
    brvd[71..75].copy_from_slice(&19u32.to_le_bytes()); // Boot Catalog LBA = 19
    out.write_all(&brvd)?;

    // 4. Sector 18: Volume Descriptor Set Terminator
    let mut term = [0u8; ISO_SECTOR];
    term[0] = 0xFF;
    term[1..6].copy_from_slice(b"CD001");
    term[6] = 0x01;
    out.write_all(&term)?;

    // 5. Sector 19: El Torito Boot Catalog (UEFI Platform 0xEF)
    let mut catalog = [0u8; ISO_SECTOR];
    // Validation Entry (bytes 0..32)
    catalog[0] = 0x01; // Header ID
    catalog[1] = 0xEF; // Platform ID: EFI
    catalog[4..28].copy_from_slice(b"KINETIX OS UEFI ELTORITO");
    catalog[30] = 0x55;
    catalog[31] = 0xAA;
    let mut sum: u16 = 0;
    for chunk in catalog[0..32].chunks_exact(2) {
        sum = sum.wrapping_add(u16::from_le_bytes([chunk[0], chunk[1]]));
    }
    let checksum = (0u16).wrapping_sub(sum);
    catalog[28..30].copy_from_slice(&checksum.to_le_bytes());

    // Initial/Default Entry (bytes 32..64)
    catalog[32] = 0x88; // Bootable
    catalog[33] = 0x00; // No Emulation
    catalog[36] = 0xEF; // System Type: EFI
    let virtual_sectors_512 = ((efiboot_img.len() + 511) / 512) as u16;
    catalog[38..40].copy_from_slice(&virtual_sectors_512.to_le_bytes());
    catalog[40..44].copy_from_slice(&efi_start_lba.to_le_bytes());
    out.write_all(&catalog)?;

    // 6. Sector 20: L-Path Table & Sector 21: M-Path Table
    let mut lpath = [0u8; ISO_SECTOR];
    lpath[0] = 1; // Name length
    lpath[2..6].copy_from_slice(&22u32.to_le_bytes());
    lpath[6..8].copy_from_slice(&1u16.to_le_bytes());
    out.write_all(&lpath)?;

    let mut mpath = [0u8; ISO_SECTOR];
    mpath[0] = 1;
    mpath[2..6].copy_from_slice(&22u32.to_be_bytes());
    mpath[6..8].copy_from_slice(&1u16.to_be_bytes());
    out.write_all(&mpath)?;

    // 7. Sector 22: Root Directory
    let mut root_dir = [0u8; ISO_SECTOR];
    let mut off = 0;
    off += fill_dir_record(&mut root_dir, off, 22, ISO_SECTOR as u32, 0x02, &[0x00]);
    off += fill_dir_record(&mut root_dir, off, 22, ISO_SECTOR as u32, 0x02, &[0x01]);
    fill_dir_record(
        &mut root_dir,
        off,
        efi_start_lba,
        efiboot_img.len() as u32,
        0x00,
        b"EFIBOOT.IMG;1",
    );
    out.write_all(&root_dir)?;

    // 8. Sector 23..: Embedded FAT32 EFI System Partition Image
    out.write_all(efiboot_img)?;
    let rem = efiboot_img.len() % ISO_SECTOR;
    if rem != 0 {
        out.write_all(&zeros[..ISO_SECTOR - rem])?;
    }

    println!(
        "[IMAGE BUILDER] Successfully created UEFI Bootable ISO: {} ({} KiB)",
        iso_path,
        (total_iso_sectors as usize * ISO_SECTOR) / 1024
    );
    Ok(())
}
