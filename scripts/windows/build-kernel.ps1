# Kinetix OS — Kernel Build Script
$ErrorActionPreference = "Stop"

$env:RUSTUP_HOME = "D:\Rust\rustup"
$env:CARGO_HOME = "D:\Rust\cargo"
$env:PATH = "D:\Rust\cargo\bin;$env:PATH"

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
