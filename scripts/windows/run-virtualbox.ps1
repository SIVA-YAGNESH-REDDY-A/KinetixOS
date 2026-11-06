# Kinetix OS — VirtualBox VM Automation Script
$ErrorActionPreference = "Stop"

$vmName = "KinetixOS"
$repoRoot = (Resolve-Path "$PSScriptRoot\..\..").Path
$imgFile = "$repoRoot\artifacts\boot-images\kinetix-os.img"
$vdiFile = "$repoRoot\artifacts\boot-images\kinetix-os.vdi"
$serialLog = "$repoRoot\artifacts\boot-images\serial.log"

# Locate VBoxManage
$vboxManage = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
if (!(Test-Path $vboxManage)) {
    $found = Get-Command VBoxManage -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source
    if ($found) { $vboxManage = $found }
}

if (!(Test-Path $vboxManage)) {
    Write-Host "[ERROR] VBoxManage not found. Please ensure Oracle VirtualBox is installed." -ForegroundColor Red
    exit 1
}

Write-Host "Using VBoxManage: $vboxManage" -ForegroundColor Cyan

if (!(Test-Path $imgFile)) {
    Write-Host "[ERROR] Boot image not found ($imgFile). Run build-image.ps1 first." -ForegroundColor Red
    exit 1
}

# 1. Close medium if already registered, then convert raw image to VDI
$mediumList = & $vboxManage list hdds
if ($mediumList -match [regex]::Escape($vdiFile)) {
    & $vboxManage storageattach $vmName --storagectl "SATA" --port 0 --device 0 --type hdd --medium none 2>$null
    & $vboxManage closemedium disk $vdiFile 2>$null
}

if (Test-Path $vdiFile) {
    Remove-Item -Force $vdiFile
}
Write-Host "Converting raw image to VDI ($vdiFile)..." -ForegroundColor Cyan
& $vboxManage convertfromraw $imgFile $vdiFile --format VDI

# 2. Check if VM exists
$vmList = (& $vboxManage list vms | Out-String)
if ($vmList -notmatch "`"$vmName`"") {
    Write-Host "Creating VirtualBox VM: $vmName..." -ForegroundColor Cyan
    & $vboxManage createvm --name $vmName --ostype "Other_64" --register
    & $vboxManage modifyvm $vmName --firmware efi64
    & $vboxManage modifyvm $vmName --memory 2048
    & $vboxManage modifyvm $vmName --cpus 2
    & $vboxManage storagectl $vmName --name "SATA" --add sata --controller IntelAHCI --bootable on
    
    # Configure COM1 Serial Port to file for diagnostics
    & $vboxManage modifyvm $vmName --uart1 0x3F8 4
    & $vboxManage modifyvm $vmName --uartmode1 file $serialLog
}

# 3. Attach current VDI disk
& $vboxManage storageattach $vmName --storagectl "SATA" --port 0 --device 0 --type hdd --medium $vdiFile

# 4. Start the VM
Write-Host "Starting VirtualBox VM: $vmName..." -ForegroundColor Green
& $vboxManage startvm $vmName

Write-Host "[OK] VM launched. Serial diagnostics logged to: $serialLog" -ForegroundColor Green
