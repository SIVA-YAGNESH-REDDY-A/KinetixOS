# Kinetix OS — Disk Image Generation Script
$ErrorActionPreference = "Stop"

$env:RUSTUP_HOME = "D:\Rust\rustup"
$env:CARGO_HOME = "D:\Rust\cargo"
$env:PATH = "D:\Rust\cargo\bin;$env:PATH"

$efi = "target\x86_64-unknown-uefi\release\kinetix-bootloader.efi"
if (!(Test-Path $efi)) {
    $efi = "target\x86_64-unknown-uefi\debug\kinetix-bootloader.efi"
}

if (!(Test-Path $efi)) {
    Write-Host "[ERROR] Bootloader EFI binary not found. Build it first with build-bootloader.ps1" -ForegroundColor Red
    exit 1
}

$outputImg = "artifacts\boot-images\kinetix-os.img"
$kernelElf = "target\x86_64-unknown-none\release\kinetix-kernel"
$appElf = "target\x86_64-unknown-none\release\hello-world"

Write-Host "Assembling Kinetix OS Bootable Image..." -ForegroundColor Cyan
cargo run -p kinetix-image-builder -- $efi $outputImg $kernelElf $appElf

$sizeMb = [math]::Round((Get-Item $outputImg).Length / 1MB, 2)
Write-Host "[SUCCESS] Boot image created: $outputImg ($sizeMb MiB)" -ForegroundColor Green
