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

# Stop VM if running
$vmInfo = (cmd /c "`"$vboxManage`" showvminfo $vmName --machinereadable 2>nul") -join "`n"
if ($vmInfo -match 'VMState="running"') {
    Write-Host "Stopping running VM..." -ForegroundColor Yellow
    cmd /c "`"$vboxManage`" controlvm $vmName poweroff >nul 2>&1"
    Start-Sleep -Seconds 2
}

# 1. Close medium if already registered, then convert raw image to VDI
$mediumList = (cmd /c "`"$vboxManage`" list hdds 2>nul") -join "`n"
if ($mediumList -match [regex]::Escape($vdiFile)) {
    cmd /c "`"$vboxManage`" storageattach $vmName --storagectl SATA --port 0 --device 0 --type hdd --medium none >nul 2>&1"
    cmd /c "`"$vboxManage`" closemedium disk `"$vdiFile`" >nul 2>&1"
}

if (Test-Path $vdiFile) {
    Remove-Item -Force $vdiFile
}
Write-Host "Converting raw image to VDI ($vdiFile)..." -ForegroundColor Cyan
& $vboxManage convertfromraw $imgFile $vdiFile --format VDI
# Restore fixed UUID expected by KinetixOS.vbox to avoid UUID mismatch errors
cmd /c "`"$vboxManage`" internalcommands sethduuid `"$vdiFile`" `"09841583-70b8-49bc-91f4-83bc76bbe6e3`" >nul 2>&1"
cmd /c "`"$vboxManage`" modifyvm $vmName --usbxhci on >nul 2>&1"

# 2. Check if VM exists
$vmList = (& $vboxManage list vms | Out-String)
if ($vmList -notmatch "`"$vmName`"") {
    Write-Host "Creating VirtualBox VM: $vmName..." -ForegroundColor Cyan
    & $vboxManage createvm --name $vmName --ostype "Other_64" --register
    & $vboxManage modifyvm $vmName --firmware efi64
    & $vboxManage modifyvm $vmName --memory 2048
    & $vboxManage modifyvm $vmName --cpus 1
    & $vboxManage storagectl $vmName --name "SATA" --add sata --controller IntelAHCI --bootable on
    
    # Configure COM1 Serial Port to file for diagnostics
    & $vboxManage modifyvm $vmName --uart1 0x3F8 4
    & $vboxManage modifyvm $vmName --uartmode1 file $serialLog
}

# 3. Attach current VDI disk
& $vboxManage storageattach $vmName --storagectl "SATA" --port 0 --device 0 --type hdd --medium $vdiFile

Write-Host "[OK] Virtual disk updated and attached to '$vmName'." -ForegroundColor Green
Write-Host "     Now click the green 'Start' button in your VirtualBox Manager window!" -ForegroundColor Yellow
Write-Host "     Serial diagnostics will be logged to: $serialLog" -ForegroundColor Cyan
