# Phase 13: Microsoft Store & Winget Publishing

**Version**: v3.0.0-store  
**Effort**: 2-3 days  
**Dependencies**: Phase 8 (Packaging), Phase 11 (Icons/Assets), Phase 12 (Testing)

---

## Objective

Publish LinVClipBoard to the Microsoft Store and Windows Package Manager (winget) for maximum discoverability and ease of installation on Windows.

---

## Tasks

### 13.1 Winget Publishing

**Prerequisites:**
- GitHub repository with releases
- Stable version tag (v3.0.0)
- Direct URL to the NSIS installer (from GitHub Releases)

**Submit package manifest:**

1. Fork https://github.com/microsoft/winget-pkgs
2. Create manifest at `manifests/l/LinVClipBoard/LinVClipBoard/3.0.0/`

**`LinVClipBoard.installer.yaml`:**
```yaml
# yaml-language-server: $schema=https://aka.ms/winget-manifest.installer.1.6.0.schema.json
PackageIdentifier: LinVClipBoard.LinVClipBoard
PackageVersion: 3.0.0
InstallerType: exe
InstallModes:
  - silent
  - silentWithProgress
InstallerSwitches:
  Silent: /S
  SilentWithProgress: /S
UpgradeBehavior: install
Installers:
  - Architecture: x64
    InstallerUrl: https://github.com/akash-singh/LinVClipBoard/releases/download/v3.0.0/LinVClipBoard_3.0.0_x64-setup.exe
    InstallerSha256: <SHA256_HASH>
    InstallerLocale: en-US
    Platform:
      - Windows.Desktop
    MinimumOSVersion: 10.0.17763.0
ReleaseDate: 2026-05-26
AppsAndFeaturesEntries:
  - DisplayName: LinVClipBoard
    Publisher: LinVClipBoard
    ProductCode: "{...GUID...}"
```

**`LinVClipBoard.locale.en-US.yaml`:**
```yaml
# yaml-language-server: $schema=https://aka.ms/winget-manifest.defaultLocale.1.6.0.schema.json
PackageIdentifier: LinVClipBoard.LinVClipBoard
PackageVersion: 3.0.0
PackageLocale: en-US
Publisher: LinVClipBoard
PublisherUrl: https://github.com/akash-singh/LinVClipBoard
PackageName: LinVClipBoard
PackageUrl: https://github.com/akash-singh/LinVClipBoard
License: MIT
LicenseUrl: https://github.com/akash-singh/LinVClipBoard/blob/main/LICENSE
ShortDescription: The clipboard manager Linux deserves — now on Windows.
Description: >
  LinVClipBoard is a feature-rich clipboard manager with clipboard history,
  full-text search, emoji picker, image support, snippets, and more.
Tags:
  - clipboard
  - clipboard-manager
  - productivity
  - utility
ReleaseNotesUrl: https://github.com/akash-singh/LinVClipBoard/releases/tag/v3.0.0
```

**`LinVClipBoard.yaml`:**
```yaml
# yaml-language-server: $schema=https://aka.ms/winget-manifest.version.1.6.0.schema.json
PackageIdentifier: LinVClipBoard.LinVClipBoard
PackageVersion: 3.0.0
DefaultLocale: en-US
ManifestType: version
ManifestVersion: 1.6.0
```

**Submission steps:**
1. Run `wingetcreate update LinVClipBoard.LinVClipBoard -u https://github.com/akash-singh/LinVClipBoard/releases/download/v3.0.0/LinVClipBoard_3.0.0_x64-setup.exe -v 3.0.0`
2. Fork winget-pkgs, add manifests, create PR
3. Wait for validation (typically 1-3 days)
4. Once accepted: `winget install LinVClipBoard.LinVClipBoard`

### 13.2 Microsoft Store Submission

**Prerequisites:**
- Microsoft Partner Center account (individual: $19 one-time, company: $99/year)
- Code signing certificate
- App passes certification checks

**Package for Store:**
Tauri creates NSIS installers, but the Microsoft Store prefers MSIX packaging. Two options:

**Option A: Submit NSIS installer via Store**
- Supported since 2022 — submit the signed .exe
- Microsoft wraps it in a lightweight MSIX for distribution
- Simpler but less integrated (no automatic updates via Store)

**Option B: Create MSIX package**
- Requires `MakeAppx.exe` and `SignTool.exe`
- Provides better Store integration (automatic updates, in-app purchases)
- More complex build process

**Option A is recommended for initial release.**

**Store submission checklist:**
| Item | Status |
|------|--------|
| Partner Center account created | |
| App name reserved (LinVClipBoard) | |
| App description written | |
| Screenshots (at least 5, 1366×768 or larger) | |
| App icon (300×300 PNG) | |
| Store logo (150×150 PNG) | |
| Small tile (71×71 PNG) | |
| Screenshots show Windows overlay UI | |
| Privacy policy URL | |
| Support contact info | |
| Age rating (3+ — no restrictions) | |
| Pricing (Free) | |

**Screenshots to capture:**
1. Main clipboard history overlay
2. Search functionality in action
3. Settings panel (themes, languages)
4. Emoji picker
5. Image clipboard with preview

**Privacy policy (required by Store):**
Minimal statement:

> **LinVClipBoard Privacy Policy**
> 
> LinVClipBoard operates entirely locally. All clipboard data is stored exclusively on your device in a local SQLite database. No data is transmitted to any external server.
> 
> The GIF search feature uses the KLIPY API. When you search for GIFs, the search query is sent to KLIPY's servers to retrieve results. No clipboard content is transmitted.
> 
> We do not collect, store, or share any personal information, usage data, or analytics.
> 
> Last updated: May 2026

### 13.3 Automated Store Submission (Optional)

For automated Store submissions from CI:

```yaml
- name: Submit to Microsoft Store
  shell: pwsh
  run: |
    # Install Store Submission CLI
    dotnet tool install --global Microsoft.MsixPackaging.Cli
    # Submit the package
    msstore submit `
      --app-id ${{ secrets.MS_STORE_APP_ID }} `
      --client-id ${{ secrets.MS_STORE_CLIENT_ID }} `
      --client-secret ${{ secrets.MS_STORE_CLIENT_SECRET }} `
      --tenant-id ${{ secrets.MS_STORE_TENANT_ID }} `
      --package-path target/release/bundle/nsis/LinVClipBoard_3.0.0_x64-setup.exe
```

### 13.4 Scoop Manifest (Bonus)

Scoop is a popular command-line installer for Windows:

```powershell
# bucket/extras bucket manifest
{
    "version": "3.0.0",
    "description": "The clipboard manager Linux deserves — now on Windows",
    "homepage": "https://github.com/akash-singh/LinVClipBoard",
    "license": "MIT",
    "architecture": {
        "64bit": {
            "url": "https://github.com/akash-singh/LinVClipBoard/releases/download/v3.0.0/LinVClipBoard_3.0.0_x64-setup.exe#/dl.7z",
            "hash": "SHA256_HASH"
        }
    },
    "shortcuts": [
        ["linvclip-ui.exe", "LinVClipBoard"]
    ],
    "bin": ["clipctl.exe"],
    "checkver": "github",
    "autoupdate": {
        "architecture": {
            "64bit": {
                "url": "https://github.com/akash-singh/LinVClipBoard/releases/download/v$version/LinVClipBoard_$version_x64-setup.exe#/dl.7z"
            }
        }
    }
}
```

### 13.5 Chocolatey Package (Bonus)

```powershell
# chocolateyInstall.ps1
$ErrorActionPreference = 'Stop'

$packageName = 'linvclipboard'
$url = 'https://github.com/akash-singh/LinVClipBoard/releases/download/v3.0.0/LinVClipBoard_3.0.0_x64-setup.exe'
$checksum = 'SHA256_HASH'
$checksumType = 'sha256'

$packageArgs = @{
  packageName   = $packageName
  fileType      = 'exe'
  url           = $url
  checksum      = $checksum
  checksumType  = $checksumType
  silentArgs    = '/S'
  validExitCodes= @(0)
}

Install-ChocolateyPackage @packageArgs
```

### 13.6 Post-Publishing Checklist

- [ ] `winget install LinVClipBoard.LinVClipBoard` — installs successfully
- [ ] Microsoft Store listing shows screenshots correctly
- [ ] Store app downloads and installs
- [ ] Auto-update from Store works
- [ ] Winget version auto-updates when new release is tagged
- [ ] README updated with Windows installation instructions

---

## Deliverables

1. Winget manifest submitted (PR to winget-pkgs)
2. Microsoft Store submission (pending certification)
3. Scoop manifest (optional)
4. Chocolatey package (optional)
5. README updated with Windows install instructions

---

## Acceptance Criteria

- [ ] `winget install LinVClipBoard.LinVClipBoard` works
- [ ] Microsoft Store listing is live (or in certification)
- [ ] App is discoverable on Windows via `winget search clipboard`
- [ ] Phase 13 branch committed
