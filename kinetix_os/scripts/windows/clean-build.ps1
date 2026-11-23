# Kinetix OS — Clean Artifacts Script
$ErrorActionPreference = "SilentlyContinue"

Write-Host "Cleaning build target and artifacts..." -ForegroundColor Yellow
cargo clean
if (Test-Path "artifacts") {
    Remove-Item -Recurse -Force "artifacts"
}
Write-Host "Clean complete." -ForegroundColor Green
