# Kinetix OS — Full System Build Script
$ErrorActionPreference = "Stop"

$env:RUSTUP_HOME = "D:\Rust\rustup"
$env:CARGO_HOME = "D:\Rust\cargo"
$env:PATH = "D:\Rust\cargo\bin;$env:PATH"

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "         BUILDING KINETIX OS              " -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

# 1. Build Native Application
Write-Host "Building Native App (hello-world)..." -ForegroundColor Cyan
cargo rustc -p hello-world --target x86_64-unknown-none --release -- -C link-arg=-Tapps/hello-world/app.ld
if (Test-Path "artifacts\ramdisk\bin") {
    Copy-Item -Force "target\x86_64-unknown-none\release\hello-world" "artifacts\ramdisk\bin\hello-world"
}

# 2. Build Bootloader
& "$PSScriptRoot\build-bootloader.ps1"

# 3. Build Kernel (packs bootfs.tar with latest hello-world)
& "$PSScriptRoot\build-kernel.ps1"

Write-Host "`nAll Kinetix OS components compiled successfully!" -ForegroundColor Green
