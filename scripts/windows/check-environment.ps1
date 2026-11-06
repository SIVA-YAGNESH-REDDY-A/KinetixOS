# Kinetix OS — Environment Verification Script
$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "    KINETIX OS ENVIRONMENT CHECK       " -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

$env:RUSTUP_HOME = "D:\Rust\rustup"
$env:CARGO_HOME = "D:\Rust\cargo"
$env:PATH = "D:\Rust\cargo\bin;$env:PATH"

# 1. Rust Toolchain
try {
    $rustcVer = & rustc --version
    Write-Host "[OK] rustc: $rustcVer" -ForegroundColor Green
} catch {
    Write-Host "[FAIL] rustc not found" -ForegroundColor Red
    exit 1
}

# 2. Cargo
try {
    $cargoVer = & cargo --version
    Write-Host "[OK] cargo: $cargoVer" -ForegroundColor Green
} catch {
    Write-Host "[FAIL] cargo not found" -ForegroundColor Red
    exit 1
}

# 3. Targets
$installedTargets = & rustup target list --installed
foreach ($target in 'x86_64-unknown-uefi', 'x86_64-unknown-none') {
    if ($installedTargets -contains $target) {
        Write-Host "[OK] target: $target" -ForegroundColor Green
    } else {
        Write-Host "[WARN] target missing: $target" -ForegroundColor Yellow
    }
}

# 4. Git
try {
    $gitVer = & git --version
    Write-Host "[OK] git: $gitVer" -ForegroundColor Green
} catch {
    Write-Host "[WARN] git not on PATH" -ForegroundColor Yellow
}

# 5. VirtualBox
$vbox = Get-Command VBoxManage -ErrorAction SilentlyContinue
if ($vbox) {
    Write-Host "[OK] VBoxManage: $($vbox.Source)" -ForegroundColor Green
} else {
    $defaultVBox = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
    if (Test-Path $defaultVBox) {
        Write-Host "[OK] VBoxManage found at $defaultVBox" -ForegroundColor Green
    } else {
        Write-Host "[INFO] VirtualBox not detected on PATH yet." -ForegroundColor Yellow
    }
}

Write-Host "`nEnvironment verification finished." -ForegroundColor Cyan
