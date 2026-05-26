# PowerShell build script for LinVClipBoard on Windows
# Usage: .\windows\scripts\build.ps1 [-Release] [-Target x86_64-pc-windows-msvc]

param(
    [switch]$Release = $true,
    [string]$Target = "x86_64-pc-windows-msvc",
    [switch]$BuildUi = $false
)

$ErrorActionPreference = "Stop"

Write-Host "=== LinVClipBoard Windows Build ===" -ForegroundColor Cyan
Write-Host "Target: $Target"
Write-Host "Release: $Release"
Write-Host "Build UI: $BuildUi"
Write-Host ""

$configFlag = if ($Release) { "--release" } else { "" }

# Step 1: Build shared library crate
Write-Host ">> Building shared crate..." -ForegroundColor Yellow
cargo build --target $Target $configFlag -p shared
if ($LASTEXITCODE -ne 0) { throw "shared build failed" }

# Step 2: Build clipd daemon
Write-Host ">> Building clipd daemon..." -ForegroundColor Yellow
cargo build --target $Target $configFlag -p clipd
if ($LASTEXITCODE -ne 0) { throw "clipd build failed" }

# Step 3: Build clipctl CLI
Write-Host ">> Building clipctl CLI..." -ForegroundColor Yellow
cargo build --target $Target $configFlag -p clipctl
if ($LASTEXITCODE -ne 0) { throw "clipctl build failed" }

if ($BuildUi) {
    # Step 4: Copy binaries to Tauri resources
    $targetDir = if ($Release) { "release" } else { "debug" }
    $binDir = "target/$Target/$targetDir"
    Copy-Item "$binDir/clipd.exe"   "crates/linvclip-ui/src-tauri/resources/" -Force
    Copy-Item "$binDir/clipctl.exe" "crates/linvclip-ui/src-tauri/resources/" -Force

    # Step 5: Build Tauri UI
    Write-Host ">> Building Tauri UI..." -ForegroundColor Yellow
    Set-Location crates/linvclip-ui
    npm install
    npx tauri build --target $Target
    if ($LASTEXITCODE -ne 0) { throw "Tauri build failed" }
    Set-Location ../..
}

Write-Host ""
Write-Host "=== Build Complete ===" -ForegroundColor Green
