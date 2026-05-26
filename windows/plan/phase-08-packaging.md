# Phase 8: Windows NSIS Installer & Packaging

**Version**: v3.0.0-pkg  
**Effort**: 3-4 days  
**Dependencies**: Phase 5 (Tauri GUI), Phase 6 (Autostart)

---

## Objective

Create a production-ready Windows installer using Tauri v2's NSIS bundler. The installer must:
- Install all components (clipd.exe, clipctl.exe, linvclip-ui.exe)
- Register autostart (optional, user chooses)
- Add clipctl to PATH
- Handle upgrades gracefully
- Support silent install (for enterprise deployment)
- Be code signed

---

## Tasks

### 8.1 Configure Tauri NSIS Bundle

**`crates/linvclip-ui/src-tauri/tauri.conf.json`:**

```json
{
  "bundle": {
    "identifier": "com.linvclipboard.app",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/icon.ico"
    ],
    "targets": ["nsis"],
    "windows": {
      "webviewInstallMode": {
        "type": "downloadBootstrapper"
      },
      "nsis": {
        "installMode": "perUser",
        "languages": ["en-US"],
        "displayLanguageSelector": false,
        "installerIcon": "./windows/assets/installer-icon.ico",
        "installerHooks": "./windows/hooks/installer.nsh",
        "template": null
      }
    },
    "createUpdaterArtifacts": true
  }
}
```

### 8.2 Create NSIS Installer Hooks

**`windows/hooks/installer.nsh`:**

Custom NSIS pages and logic:

```nsis
!macro preInit
  ; Check if already installed — get previous install dir
  ReadRegStr $INSTDIR HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LinVClipBoard" \
    "InstallLocation"
  IfErrors +2
    StrCpy $INSTDIR $INSTDIR
!macroend

!macro customHeader
  !insertmacro MUI_HEADER_TEXT "LinVClipBoard Setup" \
    "The clipboard manager Linux deserves — now on Windows"
!macroend

!macro customInstall
  ; Create clipd data directory
  CreateDirectory "$APPDATA\LinVClipBoard"

  ; Add clipctl to PATH (per-user)
  WriteRegExpandStr HKCU "Environment" "PATH" \
    "$INSTDIR;$PROFILE\.cargo\bin"

  ; Register autostart (optional)
  ${If} ${Cmd} `MessageBox MB_YESNO "Launch at startup?" IDYES`
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" \
      "LinVClipBoard Daemon" "$INSTDIR\clipd.exe"
  ${EndIf}
!macroend

!macro customUnInstall
  ; Remove clipd data directory
  RMDir /r "$APPDATA\LinVClipBoard"

  ; Remove autostart
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" \
    "LinVClipBoard Daemon"

  ; Clean PATH
  ; (requires reading PATH, removing entry, rewriting — complex)
!macroend
```

### 8.3 Bundle All Binaries

Tauri's NSIS bundler packages the Tauri GUI binary and resources, but our project has three binaries. We need to include clipd.exe and clipctl.exe in the bundle.

**Using `bundle.resources`:**
```json
{
  "bundle": {
    "resources": {
      "../target/release/clipd.exe": "clipd.exe",
      "../target/release/clipctl.exe": "clipctl.exe"
    }
  }
}
```

Or use a custom build script that copies them to the Tauri resources directory:

```bash
#!/bin/bash
# Pre-build: compile all binaries
cargo build --release -p clipd -p clipctl -p linvclip-ui
# Copy clipd and clipctl to Tauri's resource directory
cp target/release/clipd.exe crates/linvclip-ui/src-tauri/resources/
cp target/release/clipctl.exe crates/linvclip-ui/src-tauri/resources/
# Then run npx tauri build
npx tauri build --bundles nsis
```

### 8.4 Code Signing

**Prerequisites:**
- Purchase OV Code Signing Certificate (~$250-300/year from DigiCert, Sectigo, etc.)
- Or use Microsoft Trusted Signing ($9.99-99.99/month) for cloud-based signing via Azure

**Signing process (manual):**
```powershell
# Sign the installer
signtool sign /fd SHA256 /a /tr http://timestamp.digicert.com /td SHA256 ^
  target/release/bundle/nsis/LinVClipBoard_3.0.0_x64-setup.exe
```

**Signing process (CI — using Azure Key Vault + Microsoft Trusted Signing):**
```yaml
- name: Sign Windows Installer
  shell: pwsh
  run: |
    $cert = "LinVClipBoardSigningCert"
    $timestamp = "http://timestamp.acs.microsoft.com"
    & "C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64\signtool.exe" sign `
      /fd SHA256 /a /tr $timestamp /td SHA256 `
      /v "$env:TAURI_BUILD_DIR\bundle\nsis\*.exe"
```

### 8.5 Create Build Script

**`windows/scripts/build.ps1`:**

```powershell
#!/usr/bin/env pwsh
# Build script for Windows release
param(
    [switch]$Release = $true,
    [string]$Target = "x86_64-pc-windows-msvc"
)

$ErrorActionPreference = "Stop"

Write-Host "=== Building LinVClipBoard for Windows ===" -ForegroundColor Green

# Step 1: Build all Rust binaries
Write-Host "`n[1/4] Building Rust binaries..." -ForegroundColor Cyan
cargo build --release -p clipd -p clipctl
if ($LASTEXITCODE -ne 0) { throw "clipd/clipctl build failed" }

# Step 2: Copy binaries to Tauri resources
Write-Host "`n[2/4] Copying binaries to resources..." -ForegroundColor Cyan
Copy-Item "target/release/clipd.exe" "crates/linvclip-ui/src-tauri/resources/" -Force
Copy-Item "target/release/clipctl.exe" "crates/linvclip-ui/src-tauri/resources/" -Force

# Step 3: Build Tauri app
Write-Host "`n[3/4] Building Tauri GUI..." -ForegroundColor Cyan
Set-Location "crates/linvclip-ui"
npm install
npx tauri build --bundles nsis
if ($LASTEXITCODE -ne 0) { throw "Tauri build failed" }
Set-Location ../..

# Step 4: Code signing (if certificate available)
Write-Host "`n[4/4] Code signing..." -ForegroundColor Cyan
$installer = Get-Item "crates/linvclip-ui/src-tauri/target/release/bundle/nsis/*.exe"
if ($env:CODE_SIGNING_THUMBPRINT) {
    & "C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64\signtool.exe" sign `
        /fd SHA256 /a /sha1 $env:CODE_SIGNING_THUMBPRINT `
        /tr http://timestamp.digicert.com /td SHA256 `
        $installer.FullName
    Write-Host "Signed: $($installer.Name)" -ForegroundColor Green
} else {
    Write-Host "Skipping signing (no certificate configured)" -ForegroundColor Yellow
}

Write-Host "`n=== Build complete! ===" -ForegroundColor Green
Write-Host "Installer: $($installer.FullName)"
```

### 8.6 Create Linux Cross-Compile Script

**`windows/scripts/build-cross.sh`:**

```bash
#!/bin/bash
# Cross-compile Windows build from Linux
set -euo pipefail

echo "=== Cross-compiling LinVClipBoard for Windows ==="

# Install Windows target
rustup target add x86_64-pc-windows-msvc

# Build Rust binaries with cargo-xwin
cargo install cargo-xwin

echo "[1/4] Building clipd..."
cargo xwin build --release --target x86_64-pc-windows-msvc -p clipd

echo "[2/4] Building clipctl..."
cargo xwin build --release --target x86_64-pc-windows-msvc -p clipctl

echo "[3/4] Copying binaries..."
cp target/x86_64-pc-windows-msvc/release/clipd.exe crates/linvclip-ui/src-tauri/resources/
cp target/x86_64-pc-windows-msvc/release/clipctl.exe crates/linvclip-ui/src-tauri/resources/

echo "[4/4] Building Tauri UI..."
cd crates/linvclip-ui
npm install
npx tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis
cd ../..

echo "=== Done! ==="
ls -lh crates/linvclip-ui/src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/
```

### 8.7 Add Windows Build Targets to Makefile

```makefile
.PHONY: build-windows
build-windows: ## Build Windows binaries (cross-compile from Linux)
	cargo xwin build --release --target x86_64-pc-windows-msvc -p clipd -p clipctl
	@echo "Windows binaries: target/x86_64-pc-windows-msvc/release/*.exe"

.PHONY: build-windows-installer
build-windows-installer: build-windows ## Build Windows NSIS installer
	cp target/x86_64-pc-windows-msvc/release/clipd.exe crates/linvclip-ui/src-tauri/resources/
	cp target/x86_64-pc-windows-msvc/release/clipctl.exe crates/linvclip-ui/src-tauri/resources/
	cd crates/linvclip-ui && npm install && npx tauri build $(TAURI_ARGS)
	@echo "Windows installer built"

.PHONY: sign-windows
sign-windows: ## Sign Windows installer
	signtool sign /fd SHA256 /a /tr http://timestamp.digicert.com \
		crates/linvclip-ui/src-tauri/target/release/bundle/nsis/*.exe
```

### 8.8 Version Info Resource

**`windows/resources/app.rc`:**

```rc
#include <winver.h>

VS_VERSION_INFO VERSIONINFO
FILEVERSION     3,0,0,0
PRODUCTVERSION  3,0,0,0
FILEFLAGSMASK   0x3fL
FILEFLAGS       0x0L
FILEOS          VOS_NT_WINDOWS32
FILETYPE        VFT_APP
FILESUBTYPE     0x0L
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904B0"
        BEGIN
            VALUE "CompanyName", "LinVClipBoard"
            VALUE "FileDescription", "LinVClipBoard - Clipboard Manager"
            VALUE "FileVersion", "3.0.0"
            VALUE "InternalName", "linvclipboard"
            VALUE "LegalCopyright", "Copyright 2026"
            VALUE "OriginalFilename", "linvclip-ui.exe"
            VALUE "ProductName", "LinVClipBoard"
            VALUE "ProductVersion", "3.0.0"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x409, 1200
    END
END
```

### 8.9 NSIS Uninstaller

- [ ] Remove files and directories
- [ ] Remove autostart registry key
- [ ] Remove PATH entry
- [ ] Prompt to delete user data (clipboard history DB)
- [ ] Kill clipd process before uninstall

### 8.10 Test Installation

- [ ] Clean install on Windows 10
- [ ] Clean install on Windows 11
- [ ] Upgrade install (previous version → new version)
- [ ] Uninstall (with and without data deletion)
- [ ] Silent install: `LinVClipBoard_3.0.0_x64-setup.exe /S`
- [ ] Install without admin (perUser mode)
- [ ] Verify all binaries are in %LOCALAPPDATA%\Programs\LinVClipBoard
- [ ] Verify clipd.exe starts on login (if autostart selected)
- [ ] Verify clipctl.exe is in PATH

---

## Deliverables

1. Windows NSIS installer (signed or unsigned)
2. Build scripts for Windows (PowerShell) and cross-compilation (Bash)
3. Makefile targets for Windows builds
4. Version info resource for all binaries

---

## Acceptance Criteria

- [ ] NSIS installer runs and installs all components
- [ ] All three binaries (clipd, clipctl, linvclip-ui) are installed
- [ ] Autostart option works
- [ ] clipctl is accessible from command prompt after install
- [ ] Uninstaller removes all files (optionally user data)
- [ ] Upgrade install works without data loss
- [ ] Silent install works (`/S` flag)
- [ ] Phase 8 branch committed
