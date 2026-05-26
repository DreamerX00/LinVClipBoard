# Phase 6: Windows Autostart & Service Management

**Version**: v3.0.0-autostart  
**Effort**: 1-2 days  
**Dependencies**: Phase 5 (Tauri GUI)

---

## Objective

Implement Windows startup behavior: launch clipd automatically at user login, manage the daemon lifecycle, and provide a `Settings → Launch at startup` toggle in the GUI. On Windows, this is done via the `HKCU\...\Run` registry key (no admin elevation needed).

---

## Tasks

### 6.1 Implement ServiceManager for Windows

**`crates/platform/src/windows/service.rs`:**

```rust
use winreg::enums::*;
use winreg::RegKey;
use std::path::PathBuf;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APP_NAME: &str = "LinVClipBoard";

pub struct WindowsServiceManager;

impl ServiceManager for WindowsServiceManager {
    fn register_autostart(&self, app_path: &str) -> Result<(), PlatformError> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu.open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE)
            .map_err(|e| PlatformError::Service(format!("Open run key: {e}")))?;
        key.set_value(APP_NAME, &app_path)
            .map_err(|e| PlatformError::Service(format!("Set value: {e}")))?;
        Ok(())
    }

    fn unregister_autostart(&self) -> Result<(), PlatformError> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu.open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE)
            .map_err(|e| PlatformError::Service(format!("Open run key: {e}")))?;
        key.delete_value(APP_NAME)
            .map_err(|e| PlatformError::Service(format!("Delete value: {e}")))?;
        Ok(())
    }

    fn is_autostart_enabled(&self) -> Result<bool, PlatformError> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu.open_subkey_with_flags(RUN_KEY, KEY_READ)
            .map_err(|e| PlatformError::Service(format!("Open run key: {e}")))?;
        let value: Result<String, _> = key.get_value(APP_NAME);
        Ok(value.is_ok())
    }
}
```

### 6.2 Find clipd Path at Runtime

During installation, clipd location is known. At runtime, find it:

```rust
fn get_clipd_path() -> PathBuf {
    // clipd is in the same directory as the Tauri GUI
    let mut path = std::env::current_exe()
        .unwrap_or_default();
    path.set_file_name("clipd");
    path.set_extension("exe");
    path
}
```

### 6.3 Add Autostart Toggle to GUI

**In Rust `lib.rs`**, add Tauri commands:

```rust
#[tauri::command]
async fn set_launch_at_startup(enabled: bool) -> Result<(), String> {
    let service_manager = platform::service::create_service_manager()
        .map_err(|e| e.to_string())?;
    let clipd_path = get_clipd_path()
        .to_string_lossy()
        .to_string();

    if enabled {
        service_manager.register_autostart(&clipd_path)
    } else {
        service_manager.unregister_autostart()
    }
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn is_launch_at_startup_enabled() -> Result<bool, String> {
    let service_manager = platform::service::create_service_manager()
        .map_err(|e| e.to_string())?;
    service_manager.is_autostart_enabled()
        .map_err(|e| e.to_string())
}
```

**In React frontend `SettingsPanel.jsx`**, add a toggle:
```jsx
<SettingsRow label="Launch at startup">
  <ToggleSwitch
    checked={startupEnabled}
    onChange={async (val) => {
      await invoke('set_launch_at_startup', { enabled: val });
      setStartupEnabled(val);
    }}
  />
</SettingsRow>
```

### 6.4 Spawn clipd from Tauri GUI

On Windows, the Tauri GUI process should start clipd as a child process:

```rust
#[cfg(windows)]
fn spawn_clipd() -> Result<(), String> {
    let clipd_path = get_clipd_path();
    if !clipd_path.exists() {
        return Err("clipd.exe not found".into());
    }

    // Check if clipd is already running
    if is_clipd_running() {
        return Ok(());
    }

    let child = std::process::Command::new(&clipd_path)
        .creation_flags(winapi::um::winbase::CREATE_NO_WINDOW) // No console window
        .spawn()
        .map_err(|e| format!("Failed to start clipd: {e}"))?;

    // Don't wait — let clipd run independently
    std::mem::forget(child);

    Ok(())
}

fn is_clipd_running() -> bool {
    // Check named pipe exists
    let pipe = std::path::Path::new(r"\\.\pipe\LinVClipBoard");
    pipe.exists()
}
```

### 6.5 Graceful Shutdown Hook

When the GUI exits, ensure clipd continues running (unless user is uninstalling):

```rust
app.on_window_event(|window, event| {
    if let WindowEvent::CloseRequested { .. } = event {
        // On Windows, don't kill clipd — it runs independently
        // Just hide the window (it's always in tray)
        window.hide().ok();
    }
});
```

### 6.6 Install-Time Autostart Registration

In the NSIS installer (Phase 8), add:
```nsis
Section "Autostart" SEC_AUTOSTART
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" \
    "LinVClipBoard" "$INSTDIR\clipd.exe"
SectionEnd
```

### 6.7 Registry Cleanup on Uninstall

```nsis
Section "Uninstall"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" \
    "LinVClipBoard"
SectionEnd
```

---

## Deliverables

1. `HKCU\...\Run` autostart registration/unregistration
2. GUI toggle for "Launch at startup"
3. clipd spawns automatically with Tauri GUI
4. clipd continues running when GUI is closed

---

## Acceptance Criteria

- [ ] Toggle "Launch at startup" writes HKCU registry key
- [ ] After login, clipd starts automatically
- [ ] GUI starts clipd if not already running
- [ ] Closing GUI does not stop clipd
- [ ] Uninstaller removes registry key
- [ ] Setting syncs immediately (no restart needed)
- [ ] Linux autostart (systemd) still works
- [ ] Phase 6 branch committed
