# PowerShell build script for LinVClipBoard on Windows
# Usage: .\windows\scripts\build.ps1 [-Release] [-Target x86_64-pc-windows-msvc]

param(
    [switch]$Release = $true,
    [string]$Target = "x86_64-pc-windows-msvc",
    [switch]$BuildUi = $true
)

$ErrorActionPreference = "Stop"

Write-Host "=== LinVClipBoard Windows Build ===" -ForegroundColor Cyan
Write-Host "Target: $Target"
Write-Host "Release: $Release"
Write-Host "Build UI: $BuildUi"
Write-Host ""

$configFlag = if ($Release) { "--release" } else { "" }

# Step 1: Build all Rust binaries
Write-Host ">> Building clipboard daemon..." -ForegroundColor Yellow
cargo build --target $Target $configFlag -p clipd
if ($LASTEXITCODE -ne 0) { throw "clipd build failed" }

Write-Host ">> Building CLI client..." -ForegroundColor Yellow
cargo build --target $Target $configFlag -p clipctl
if ($LASTEXITCODE -ne 0) { throw "clipctl build failed" }

if ($BuildUi) {
    # Step 2: Prepare resources directory
    $targetDir = if ($Release) { "release" } else { "debug" }
    $binDir = "target/$Target/$targetDir"
    $resDir = "crates/linvclip-ui/src-tauri/resources"

    New-Item -ItemType Directory -Path $resDir -Force | Out-Null

    Write-Host ">> Copying binaries to resources..." -ForegroundColor Yellow
    Copy-Item "$binDir/clipd.exe"   "$resDir/clipd.exe" -Force
    Copy-Item "$binDir/clipctl.exe" "$resDir/clipctl.exe" -Force

    # Step 3: Build Tauri UI + NSIS installer
    Write-Host ">> Building Tauri GUI + NSIS installer..." -ForegroundColor Yellow
    Set-Location crates/linvclip-ui
    npm install
    npx tauri build --target $Target --bundles nsis
    if ($LASTEXITCODE -ne 0) { throw "Tauri build failed" }
    Set-Location ../..

    Write-Host ""
    Write-Host "=== Build Complete ===" -ForegroundColor Green
    Write-Host "Installer: crates/linvclip-ui/src-tauri/target/$Target/$targetDir/bundle/nsis/LinVClipBoard_*_x64-setup.exe"
} else {
    Write-Host ""
    Write-Host "=== Rust Binaries Built ===" -ForegroundColor Green
    Write-Host "clipd.exe and clipctl.exe are in $binDir/"
}
