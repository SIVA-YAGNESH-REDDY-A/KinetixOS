# Kinetix OS — Bootloader Build Script
$ErrorActionPreference = "Stop"

$env:RUSTUP_HOME = "D:\Rust\rustup"
$env:CARGO_HOME = "D:\Rust\cargo"
$env:PATH = "D:\Rust\cargo\bin;$env:PATH"

Write-Host "Building Kinetix Bootloader (x86_64-unknown-uefi)..." -ForegroundColor Cyan
cargo build -p kinetix-bootloader --target x86_64-unknown-uefi --release

$outputEfi = "target\x86_64-unknown-uefi\release\kinetix-bootloader.efi"
if (Test-Path $outputEfi) {
    $size = (Get-Item $outputEfi).Length
    Write-Host "[SUCCESS] Bootloader built: $outputEfi ($size bytes)" -ForegroundColor Green
} else {
    Write-Host "[FAIL] Output binary not found: $outputEfi" -ForegroundColor Red
    exit 1
}
