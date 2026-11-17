# Kinetix OS — Kernel Build Script
$ErrorActionPreference = "Stop"

$env:RUSTUP_HOME = "D:\Rust\rustup"
$env:CARGO_HOME = "D:\Rust\cargo"
$env:PATH = "D:\Rust\cargo\bin;$env:PATH"

# Pack initial ramdisk into USTAR archive
Write-Host "Packaging Boot Filesystem (bootfs.tar)..." -ForegroundColor Cyan
if (Test-Path "artifacts\ramdisk") {
    tar --format ustar -cf kernel\kinetix-kernel\bootfs.tar -C artifacts\ramdisk bin etc README.txt
    Copy-Item -Force kernel\kinetix-kernel\bootfs.tar artifacts\boot-images\bootfs.tar
}

Write-Host "Building Kinetix Kernel (x86_64-unknown-none)..." -ForegroundColor Cyan
cargo rustc -p kinetix-kernel --target x86_64-unknown-none --release -- -C link-arg=-Tkernel/kinetix-kernel/linker.ld

$outputKernel = "target\x86_64-unknown-none\release\kinetix-kernel"
if (Test-Path $outputKernel) {
    $size = (Get-Item $outputKernel).Length
    Write-Host "[SUCCESS] Kernel built: $outputKernel ($size bytes)" -ForegroundColor Green
} else {
    Write-Host "[FAIL] Output binary not found: $outputKernel" -ForegroundColor Red
    exit 1
}
