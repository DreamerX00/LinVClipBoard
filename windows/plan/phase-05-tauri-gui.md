# Phase 5: Tauri GUI — Windows Adaptation

**Version**: v3.0.0-gui  
**Effort**: 4-5 days  
**Dependencies**: Phase 2 (clipd), Phase 3 (IPC)

---

## Objective

Adapt the Tauri v2 GUI (`linvclip-ui`) to run natively on Windows. This involves:
1. Configuring `tauri.conf.json` for Windows bundles
2. Platform-gating Windows-specific Rust commands
3. Updating Tauri capabilities/permissions
4. Ensuring the React frontend works correctly
5. Implementing Windows system tray
6. Handling window management (overlay, always-on-top, transparency)

---

## Tasks

### 5.1 Update `tauri.conf.json`

**`crates/linvclip-ui/src-tauri/tauri.conf.json`:**

```json
{
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "windows": {
      "webviewInstallMode": {
        "type": "downloadBootstrapper"
      },
      "nsis": {
        "installMode": "perUser",
        "languages": ["en-US"],
        "displayLanguageSelector": false,
        "installerHooks": "./windows/hooks/postinstall.nsh"
      },
      "certificateThumbprint": "",
      "digestAlgorithm": "sha256",
      "timestampUrl": ""
    },
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/icon.ico"
    ],
    "createUpdaterArtifacts": true
  },
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "LinVClipBoard",
        "width": 800,
        "height": 600,
        "decorations": false,
        "resizable": true,
        "transparent": true,
        "alwaysOnTop": true,
        "visible": false,
        "skipTaskbar": true,
        "center": true
      }
    ]
  }
}
```

### 5.2 Add Windows Icons

- [ ] Create `crates/linvclip-ui/src-tauri/icons/icon.ico` (multi-size: 16×16, 32×32, 48×48, 256×256)
- [ ] Create NSIS installer icon (`installer-icon.ico`)
- [ ] Verify icon conversion: PNG → ICO via `magick convert icon-256.png icon.ico`

### 5.3 Platform-Gate Rust Commands

**`crates/linvclip-ui/src-tauri/src/lib.rs`:**

Identify all Linux-specific commands and gate them:

```rust
// Linux-specific: paste using wtype/xdotool
#[cfg(target_os = "linux")]
#[tauri::command]
async fn type_text(text: String) -> Result<(), String> {
    // Existing Linux implementation
}

// Windows-specific: paste using SendInput/enigo
#[cfg(target_os = "windows")]
#[tauri::command]
async fn type_text(text: String) -> Result<(), String> {
    use platform::input::create_input_simulator;
    let sim = create_input_simulator()
        .map_err(|e| e.to_string())?;
    sim.paste_text(&text)
        .map_err(|e| e.to_string())?;
    Ok(())
}
```

**Commands needing platform-specific implementations:**

| Command | Linux | Windows |
|---------|-------|---------|
| `type_text` | `wtype` / `xdotool` | `enigo` / `SendInput` |
| `install_update` | `pkexec dpkg -i` | Tauri updater |
| `check_for_updates` | GitHub Releases .deb | GitHub Releases .exe |
| `download_update` | `reqwest` → `~/Downloads/` | `reqwest` → `%TEMP%` |
| `run_silent` | Shell command | `CreateProcess` |
| `get_active_window` | `xdotool` / `swaymsg` / `hyprctl` | Win32 `GetForegroundWindow` |
| `system_theme` | D-Bus / gsettings | Windows registry |
| `get_config_dir` | `~/.config/linvclipboard` | `%APPDATA%\LinVClipBoard` |

### 5.4 Windows System Tray

```rust
// In setup function:
#[cfg(windows)]
fn setup_system_tray(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::tray::{TrayIconBuilder, MouseButton, MouseButtonState};

    let show = MenuItem::with_id(app, "show", "Show LinVClipBoard", true, None::<&str>)?;
    let separator = MenuItem::with_id(app, "sep", "", true, None::<&str>)?;
    separator.set_native_image(tauri::image::Image::new())
        .ok();
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &separator, &quit])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("LinVClipBoard")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => { app.exit(0); }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up, ..
            } = event {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}
```

### 5.5 Window Management — Overlay Behavior

Windows-specific window behavior:
- [ ] `skipTaskbar: true` — hide from taskbar (overlay behavior)
- [ ] Show/hide on global shortcut (Ctrl+Shift+V)
- [ ] Auto-hide on focus loss (click outside closes overlay)
- [ ] DPI awareness — handle per-monitor DPI scaling

```rust
// On focus lost, hide window
app.on_window_event(|window, event| {
    if let tauri::WindowEvent::Focused(false) = event {
        if let Some(label) = window.label() {
            if label == "main" && !settings.lock_window {
                window.hide().ok();
            }
        }
    }
});
```

### 5.6 Global Shortcuts on Windows

Register `Ctrl+Shift+V` as a global hotkey:

```rust
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutState};

app.handle().plugin(
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(move |app, shortcut, event| {
            if shortcut == &Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyV) {
                if event.state() == ShortcutState::Pressed {
                    if let Some(window) = app.get_webview_window("main") {
                        if window.is_visible().unwrap_or(false) {
                            window.hide().ok();
                        } else {
                            window.show().ok();
                            window.set_focus().ok();
                        }
                    }
                }
            }
        })
        .build(),
)?;
```

**Required capability:**
```json
"permissions": [
    "global-shortcut:allow-register",
    "global-shortcut:allow-unregister",
    "global-shortcut:allow-is-registered"
]
```

### 5.7 Update Capabilities

**`crates/linvclip-ui/src-tauri/capabilities/default.json`:**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "main-capability",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:default",
    "core:window:allow-show",
    "core:window:allow-hide",
    "core:window:allow-set-focus",
    "core:window:allow-set-always-on-top",
    "core:window:allow-close",
    "core:tray:default",
    "core:event:default",
    "core:app:default",
    "global-shortcut:allow-register",
    "global-shortcut:allow-unregister",
    "global-shortcut:allow-is-registered",
    "shell:allow-open",
    "updater:allow-check",
    "updater:allow-download-and-install",
    "updater:allow-download",
    "updater:allow-install"
  ]
}
```

### 5.8 Update Cargo.toml for Windows

**`crates/linvclip-ui/src-tauri/Cargo.toml`:**

- Add `enigo` for Windows input simulation:
```toml
[target.'cfg(windows)'.dependencies]
enigo = { version = "0.3", features = ["serde"] }
```

- Add `clipboard_win` for direct clipboard access:
```toml
[target.'cfg(windows)'.dependencies]
clipboard_win = { version = "5", features = ["monitor"] }
```

- Ensure Tauri deps have correct features:
```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
```

### 5.9 Frontend — No Changes Expected

The React frontend communicates with Rust via Tauri's IPC (`invoke` commands). Since the command interface should remain the same, no frontend changes are needed.

- [ ] Verify all `invoke` calls in frontend match Rust commands
- [ ] Test that window events (show/hide) fire correctly on Windows
- [ ] Test that tray icon menu works on Windows

### 5.10 Build and Test

- [ ] `npx tauri build --target x86_64-pc-windows-msvc` — succeeds
- [ ] The resulting `.exe` runs and shows the overlay
- [ ] System tray icon appears
- [ ] Global shortcut (Ctrl+Shift+V) toggles overlay
- [ ] Clipboard history is displayed (fetched from clipd via IPC)
- [ ] Overlay is transparent and click-through areas work

---

## Deliverables

1. `linvclip-ui.exe` that runs natively on Windows
2. System tray with show/quit menu
3. Global shortcut (Ctrl+Shift+V) to toggle overlay
4. Transparent overlay window
5. All Tauri commands working via named pipe IPC

---

## Acceptance Criteria

- [ ] `npx tauri build --target x86_64-pc-windows-msvc` produces a working `.exe`
- [ ] System tray icon appears in notification area
- [ ] Global shortcut shows/hides the overlay
- [ ] Clipboard history loads and displays
- [ ] Linux build still works (no regressions)
- [ ] All Tauri commands respond correctly
- [ ] Phase 5 branch committed
