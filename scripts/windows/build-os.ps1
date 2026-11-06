# Kinetix OS — Full System Build Script
$ErrorActionPreference = "Stop"

$env:RUSTUP_HOME = "D:\Rust\rustup"
$env:CARGO_HOME = "D:\Rust\cargo"
$env:PATH = "D:\Rust\cargo\bin;$env:PATH"

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "         BUILDING KINETIX OS              " -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

# 1. Build Bootloader
& "$PSScriptRoot\build-bootloader.ps1"

# 2. Build Kernel
& "$PSScriptRoot\build-kernel.ps1"

# 3. Build Native Application
Write-Host "Building Native App (hello-world)..." -ForegroundColor Cyan
cargo build -p hello-world --target x86_64-unknown-none --release

Write-Host "`nAll Kinetix OS components compiled successfully!" -ForegroundColor Green
