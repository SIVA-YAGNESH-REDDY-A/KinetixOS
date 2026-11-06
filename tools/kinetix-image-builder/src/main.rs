//! Kinetix OS Disk Image Builder
//!
//! Assembles a raw GPT disk image containing an EFI System Partition (ESP) formatted
//! with FAT32, placing `BOOTX64.EFI`, `KINETIX-KERNEL.ELF`, and native applications
//! in their standard filesystem hierarchy.

fn main() {
    println!("Kinetix OS Disk Image Builder v0.1.0");
    println!("Usage: kinetix-image-builder <efi_path> <kernel_path> <output_img>");
}
