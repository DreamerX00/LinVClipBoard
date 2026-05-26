# Phase 11: Windows Icons, Assets & Desktop Integration

**Version**: v3.0.0-assets  
**Effort**: 1-2 days  
**Dependencies**: Phase 5 (Tauri GUI)

---

## Objective

Create all Windows-specific visual assets: application icons (`.ico` format in multiple sizes), installer graphics, desktop shortcuts, file type associations, and proper Windows application metadata.

---

## Tasks

### 11.1 Generate Windows Icons

**Source:** The existing `crates/linvclip-ui/src-tauri/icons/icon.png` (128x128)

**Generate multi-resolution ICO:**
```bash
# Using ImageMagick
convert icons/icon.png \
  \( -clone 0 -resize 16x16 \) \
  \( -clone 0 -resize 32x32 \) \
  \( -clone 0 -resize 48x48 \) \
  \( -clone 0 -resize 64x64 \) \
  \( -clone 0 -resize 128x128 \) \
  \( -clone 0 -resize 256x256 \) \
  -delete 0 -colors 256 \
  icons/icon.ico
```

**Required icon files:**
| File | Size | Purpose |
|------|------|---------|
| `icons/icon.ico` | 16×16 to 256×256 | Main app icon |
| `windows/assets/installer-icon.ico` | 32×32, 48×48 | NSIS installer icon |
| `windows/assets/uninstaller-icon.ico` | 32×32 | NSIS uninstaller icon |

### 11.2 Generate App Icon PNGs for Bundling

Tauri v2 requires PNG icons for bundling:
```bash
convert icons/icon.ico -resize 32x32 icons/32x32.png
convert icons/icon.ico -resize 128x128 icons/128x128.png
convert icons/icon.ico -resize 256x256 icons/256x256.png
```

### 11.3 Create NSIS Installer Graphics

- [ ] `windows/assets/installer-banner.bmp` — 150×57 or 164×314 (NSIS modern UI banner)
- [ ] `windows/assets/welcome.bmp` — Optional welcome page graphic

### 11.4 Application Manifest

**`windows/resources/app.manifest`:**

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity
    type="win32"
    name="LinVClipBoard"
    version="3.0.0.0"
    processorArchitecture="*"
  />

  <!-- Windows 10/11 compatibility -->
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}" /> <!-- Windows 10 -->
      <supportedOS Id="{1f676c76-80e1-4239-95bb-83d0f6d0da78}" /> <!-- Windows 11 -->
    </application>
  </compatibility>

  <!-- DPI awareness (per-monitor) -->
  <asmv3:application xmlns:asmv3="urn:schemas-microsoft-com:asm.v3">
    <asmv3:windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    </asmv3:windowsSettings>
  </asmv3:application>

  <!-- UIPI bypass (requires code signing + Program Files install) -->
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v2">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel
          level="asInvoker"
          uiAccess="true"
        />
      </requestedPrivileges>
    </security>
  </trustInfo>
</assembly>
```

### 11.5 Desktop Shortcut

Configure NSIS installer to create a desktop shortcut:

```nsis
; In NSIS install section
CreateShortCut "$DESKTOP\LinVClipBoard.lnk" "$INSTDIR\linvclip-ui.exe" \
  "" "$INSTDIR\linvclip-ui.exe" 0
```

**Tauri config approach:**
```json
{
  "bundle": {
    "windows": {
      "nsis": {
        "installerHooks": "./windows/hooks/installer.nsh",
        "createDesktopShortcut": true,
        "createStartMenuShortcut": true
      }
    }
  }
}
```

### 11.6 Start Menu Integration

```nsis
; Create Start Menu entry
CreateDirectory "$SMPROGRAMS\LinVClipBoard"
CreateShortCut "$SMPROGRAMS\LinVClipBoard\LinVClipBoard.lnk" \
  "$INSTDIR\linvclip-ui.exe" "" "$INSTDIR\linvclip-ui.exe" 0
CreateShortCut "$SMPROGRAMS\LinVClipBoard\Uninstall.lnk" \
  "$INSTDIR\Uninstall.exe" "" "$INSTDIR\Uninstall.exe" 0
```

### 11.7 File Type Associations (Optional)

If we want to register clipboard files:

```json
{
  "bundle": {
    "windows": {
      "nsis": {
        "installMode": "perUser"
      },
      "wix": {
        "fragmentPaths": ["./windows/fragments/file-types.wxs"]
      }
    }
  }
}
```

For now, skip file associations — clipboard manager doesn't need them.

### 11.8 Application User Model ID (AppUserModelID)

For proper taskbar grouping and jump lists:

```rust
#[cfg(windows)]
fn set_app_user_model_id() {
    unsafe {
        windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(
            "LinVClipBoard.LinVClipBoard",
        ).ok();
    }
}
```

### 11.9 Window Class Registration

```rust
#[cfg(windows)]
fn register_window_class() {
    unsafe {
        let class_name = h!("LinVClipBoardOverlay\0");
        let hinstance = GetModuleHandleA(None).unwrap();

        let wc = WNDCLASSEXA {
            cbSize: std::mem::size_of::<WNDCLASSEXA>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(DefWindowProcA),
            hInstance: hinstance,
            hIcon: LoadIconA(None, IDI_APPLICATION),
            hCursor: LoadCursorA(None, IDC_ARROW),
            lpszClassName: class_name.as_ptr(),
            ..Default::default()
        };

        RegisterClassExA(&wc);
    }
}
```

---

## Deliverables

1. Multi-resolution `.ico` file
2. Application manifest with uiAccess, DPI awareness, Windows 10/11 compat
3. NSIS installer graphics (banner icons)
4. Desktop shortcut and Start Menu integration
5. AppUserModelID set correctly

---

## Acceptance Criteria

- [ ] App icon displays correctly in taskbar and system tray
- [ ] DPI scaling works correctly on high-DPI displays
- [ ] Desktop shortcut is created on install
- [ ] Start Menu entry exists after install
- [ ] Taskbar grouping works correctly
- [ ] Phase 11 branch committed
