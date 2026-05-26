# PowerShell install script for LinVClipBoard (manual/dev install)
# Usage: .\windows\scripts\install.ps1 [-Uninstall]

param(
    [switch]$Uninstall = $false
)

$ErrorActionPreference = "Stop"
$AppName = "LinVClipBoard"
$AppData = "$env:APPDATA\$AppName"
$BinDir = "$env:LOCALAPPDATA\$AppName\bin"

if ($Uninstall) {
    Write-Host "Uninstalling $AppName..." -ForegroundColor Yellow

    # Remove HKCU Run entry
    Remove-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name $AppName -ErrorAction SilentlyContinue

    # Remove binaries
    if (Test-Path $BinDir) {
        Remove-Item -Recurse -Force $BinDir
    }

    Write-Host "$AppName uninstalled. User data kept at: $AppData" -ForegroundColor Green
    Write-Host "To remove user data: Remove-Item -Recurse -Force '$AppData'" -ForegroundColor DarkGray
    return
}

Write-Host "Installing $AppName (dev mode)..." -ForegroundColor Yellow

# Ensure binary directories exist
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
New-Item -ItemType Directory -Force -Path $AppData | Out-Null

# Copy binaries from build output
$targetDir = "target\x86_64-pc-windows-msvc\release"
if (-not (Test-Path "$targetDir\clipd.exe")) {
    Write-Error "Build first: .\windows\scripts\build.ps1"
    exit 1
}

Copy-Item "$targetDir\clipd.exe"   "$BinDir\" -Force
Copy-Item "$targetDir\clipctl.exe" "$BinDir\" -Force

# Add to PATH for current user (if not already)
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$BinDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$BinDir", "User")
    Write-Host "Added $BinDir to user PATH" -ForegroundColor Green
}

# Register HKCU Run for autostart
Set-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name $AppName -Value "$BinDir\clipd.exe"
Write-Host "Registered autostart: HKCU Run -> $BinDir\clipd.exe" -ForegroundColor Green

Write-Host ""
Write-Host "Installation complete!" -ForegroundColor Green
Write-Host "Start the daemon: clipd" -ForegroundColor Cyan
Write-Host "Try the CLI:     clipctl status" -ForegroundColor Cyan
