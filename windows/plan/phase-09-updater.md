# Phase 9: Windows Auto-Updater

**Version**: v3.0.0-update  
**Effort**: 2-3 days  
**Dependencies**: Phase 8 (Packaging, NSIS installer)

---

## Objective

Implement automatic updates for the Windows version using Tauri v2's built-in updater plugin. On Linux, updates are handled via `.deb` download + `pkexec dpkg -i`. On Windows, we will use Tauri's updater with NSIS installer.

---

## Tasks

### 9.1 Add Tauri Updater Plugin

```bash
cd crates/linvclip-ui
npm run tauri add updater
```

This adds:
- `tauri-plugin-updater` to Rust dependencies
- `@tauri-apps/plugin-updater` to npm dependencies
- Updates `tauri.conf.json` with updater config

### 9.2 Generate Signing Keys

```bash
npm run tauri signer generate -- -w ~/.tauri/linvclipboard.key
```

This produces:
- Private key: `~/.tauri/linvclipboard.key`
- Public key (displayed in terminal)

**Important:** Save the public key — it goes in `tauri.conf.json`.

### 9.3 Configure Updater in tauri.conf.json

```json
{
  "plugins": {
    "updater": {
      "pubkey": "CONTENT_OF_PUBLIC_KEY",
      "endpoints": [
        "https://github.com/akash-singh/LinVClipBoard/releases/latest/download/update-windows-x86_64.json"
      ],
      "windows": {
        "installMode": "passive"
      }
    }
  },
  "bundle": {
    "createUpdaterArtifacts": true
  }
}
```

`installMode` options:
- `"passive"` — silent install with progress bar (recommended)
- `"basicUi"` — shows install dialog
- `"quiet"` — completely silent

### 9.4 Build with Updater Artifacts

```bash
export TAURI_SIGNING_PRIVATE_KEY="~/.tauri/linvclipboard.key"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
npm run tauri build --bundles nsis
```

This produces:
- `target/release/bundle/nsis/LinVClipBoard_3.0.0_x64-setup.exe`
- `target/release/bundle/nsis/LinVClipBoard_3.0.0_x64-setup.exe.sig`
- `target/release/bundle/nsis/LinVClipBoard_3.0.0_x64-setup.nsis.zip`

### 9.5 Create Update JSON Manifest

The updater reads a JSON manifest to check for new versions. Hosted on GitHub Releases:

**`update-windows-x86_64.json`:**

```json
{
  "version": "3.0.0",
  "notes": "See the assets to download this version and install.",
  "pub_date": "2026-05-26T00:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "CONTENT_OF_SIG_FILE",
      "url": "https://github.com/akash-singh/LinVClipBoard/releases/download/v3.0.0/LinVClipBoard_3.0.0_x64-setup.exe"
    }
  }
}
```

### 9.6 Add Update Check to CI (Phase 10)

In the release workflow, after building the NSIS installer:

```yaml
- name: Generate update manifest
  run: |
    VERSION=$(git describe --tags | sed 's/^v//')
    {
      echo "version": "$VERSION",
      echo "notes": "Release $VERSION of LinVClipBoard",
      echo "pub_date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
      echo "platforms": {
        echo "windows-x86_64": {
          echo "signature": "$(cat target/release/bundle/nsis/*.sig)",
          echo "url": "https://github.com/akash-singh/LinVClipBoard/releases/download/v$VERSION/LinVClipBoard_${VERSION}_x64-setup.exe"
        }
      }
    } > update-windows-x86_64.json
    mv update-windows-x86_64.json target/release/bundle/nsis/
```

### 9.7 Frontend Update UI

The existing Linux frontend has update check UI (UpdateModal.jsx). Adapt it for Windows:

```jsx
// In UpdateModal.jsx — already uses Tauri invoke for check/install
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

async function checkForUpdates() {
  const update = await check();
  if (update?.available) {
    setUpdateInfo(update);
  }
}

async function installUpdate() {
  const update = await check();
  if (update) {
    await update.downloadAndInstall((event) => {
      // Handle Started, Progress, Finished
    });
    await relaunch();
  }
}
```

### 9.8 Background Update Check

The existing update check infrastructure:
- `install/linvclip-update-check.sh` (Linux shell script)
- `install/linvclip-update-check.service` (systemd timer)

On Windows, replace this with Tauri's built-in updater:
- Check for updates on app startup (configurable)
- Check periodically via `setInterval` or background task
- Show notification when update is available

```rust
#[tauri::command]
async fn check_for_updates(app: tauri::AppHandle) -> Result<UpdateInfo, String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app.updater()?;
    let update = updater.check().await
        .map_err(|e| e.to_string())?;

    match update {
        Some(update) => Ok(UpdateInfo {
            available: true,
            version: update.version,
            notes: update.body.unwrap_or_default(),
        }),
        None => Ok(UpdateInfo { available: false, ..Default::default() }),
    }
}
```

### 9.9 Handle Update Signing in CI

**GitHub Actions setup:**

```yaml
- name: Build Windows installer
  env:
    TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
    TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
  run: npm run tauri build --bundles nsis

- name: Upload artifacts
  uses: actions/upload-artifact@v4
  with:
    name: windows-installer
    path: |
      target/release/bundle/nsis/*.exe
      target/release/bundle/nsis/*.exe.sig
      target/release/bundle/nsis/*.nsis.zip
```

### 9.10 Verify the Update Flow

- [ ] Build with updater artifacts
- [ ] Host the update JSON and new installer on GitHub Releases
- [ ] Install the old version
- [ ] Open the app — it should detect the new version
- [ ] Click "Install Update" — downloads and installs silently
- [ ] App restarts with new version
- [ ] Test with a downgrade attempt (should be rejected)
- [ ] Test with network offline (graceful error)

---

## Deliverables

1. Tauri updater plugin configured for Windows
2. Update JSON manifest generation in CI
3. Frontend update detection and installation UI
4. Signed update artifacts

---

## Acceptance Criteria

- [ ] `npx tauri build --bundles nsis` produces `.exe.sig` and `.nsis.zip`
- [ ] App detects available update from GitHub Releases
- [ ] `install passive` works without user interaction
- [ ] App relaunches after update
- [ ] Signature verification prevents tampering
- [ ] Linux update system (deb + pkexec) still works
- [ ] Phase 9 branch committed
