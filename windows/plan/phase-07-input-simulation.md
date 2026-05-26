# Phase 7: Windows Input Simulation & Smart Pasting

**Version**: v3.0.0-input  
**Effort**: 3-4 days  
**Dependencies**: Phase 5 (Tauri GUI)

---

## Objective

Implement the paste functionality on Windows. When a user selects a clipboard item and triggers "paste," the app must:
1. Set the selected item as the current clipboard content
2. Simulate `Ctrl+V` (or equivalent) to paste into the foreground app

This is the most platform-sensitive phase due to UIPI (User Interface Privilege Isolation), which blocks input injection into elevated/admin processes.

---

## Tasks

### 7.1 Implement InputSimulator for Windows

**`crates/platform/src/windows/input.rs`:**

#### Primary Method: SendInput (Ctrl+V)

```rust
use enigo::{Enigo, Keyboard, Settings};

pub struct WindowsInputSimulator;

impl InputSimulator for WindowsInputSimulator {
    fn simulate_paste_shortcut(&self) -> Result<(), PlatformError> {
        let mut enigo = Enigo::new(&Settings::default())
            .map_err(|e| PlatformError::Input(e.to_string()))?;

        enigo.key(enigo::Key::Control, enigo::Direction::Press)
            .map_err(|e| PlatformError::Input(e.to_string()))?;
        std::thread::sleep(std::time::Duration::from_millis(10));
        enigo.key(enigo::Key::Layout('v'), enigo::Direction::Click)
            .map_err(|e| PlatformError::Input(e.to_string()))?;
        std::thread::sleep(std::time::Duration::from_millis(10));
        enigo.key(enigo::Key::Control, enigo::Direction::Release)
            .map_err(|e| PlatformError::Input(e.to_string()))?;

        Ok(())
    }

    fn type_text(&self, text: &str) -> Result<(), PlatformError> {
        let mut enigo = Enigo::new(&Settings::default())
            .map_err(|e| PlatformError::Input(e.to_string()))?;
        enigo.text(text)
            .map_err(|e| PlatformError::Input(e.to_string()))?;
        Ok(())
    }

    fn paste_text(&self, text: &str) -> Result<(), PlatformError> {
        // 1. Set clipboard content
        platform::clipboard::create_clipboard_provider()
            .map_err(|e| PlatformError::Input(e.to_string()))?
            .set_text(text)?;

        // Small delay for clipboard to propagate
        std::thread::sleep(std::time::Duration::from_millis(50));

        // 2. Simulate Ctrl+V
        self.simulate_paste_shortcut()
    }
}
```

#### Fallback: UI Automation (for elevated targets)

When SendInput fails (returns 0), use UI Automation:

```rust
fn paste_via_uia(text: &str) -> Result<(), PlatformError> {
    use windows::Win32::UI::Accessibility::*;

    unsafe {
        // Initialize COM for UIA
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)
            .map_err(|e| PlatformError::Input(format!("COM init: {e}")))?;

        // Get the focused element
        let uia = IUIAutomation::new()
            .map_err(|e| PlatformError::Input(format!("UIA create: {e}")))?;

        let focused = uia.GetFocusedElement()
            .map_err(|e| PlatformError::Input(format!("GetFocus: {e}")))?;

        // Set clipboard first
        set_clipboard_text(text)?;

        // Try to find and invoke the paste pattern
        // or use the legacy pattern: focus element → send Ctrl+V
        // This is complex — further research needed

        CoUninitialize();
    }
    Ok(())
}
```

### 7.2 UIPI Handling

**Problem:** When the foreground app is running as Administrator (high integrity), `SendInput` from a medium-integrity process is silently blocked.

**Solutions (in priority order):**

1. **`uiAccess=true` manifest** — Mark the application as UI Access. This requires:
   - Code signing certificate (OV or EV)
   - Installing the app in `Program Files` (or other secure location)
   - Adding `uiAccess="true"` to the app manifest
   - _This is the recommended approach_

2. **UI Automation fallback** — Use `IUIAutomation` to set focus and paste. Works across integrity levels but is slower and more complex.

3. **Explain limitation** — Show a user-facing message: "Cannot paste into elevated apps. Use Ctrl+V manually."

**Implementation:**

```rust
fn try_paste() -> Result<(), PlatformError> {
    // Try SendInput first (fast path)
    let sim = WindowsInputSimulator;
    if sim.simulate_paste_shortcut().is_ok() {
        return Ok(());
    }

    // SendInput failed — possibly UIPI blocked
    // Try UI Automation fallback
    tracing::warn!("SendInput failed (UIPI?), trying UI Automation");
    paste_via_uia(&current_clipboard_text)
}
```

**uiAccess manifest:**
```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity
    type="win32"
    name="LinVClipBoard"
    version="3.0.0.0"
    processorArchitecture="amd64"
  />
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

> **Note:** `uiAccess="true"` is NOT an elevation — the app still runs as the user. It just allows input injection across integrity levels.

### 7.3 Keyboard State Management

- [ ] Track current keyboard modifier state before simulation
- [ ] Release all modifiers before Ctrl+V
- [ ] Restore modifiers afterward
- [ ] Handle Windows key interference

```rust
fn release_all_modifiers() -> Result<(), PlatformError> {
    let keys = [
        enigo::Key::Control, enigo::Key::Shift, enigo::Key::Alt,
        enigo::Key::Meta,  // Windows key
    ];
    for key in &keys {
        let _ = enigo.key(*key, enigo::Direction::Release);
    }
    Ok(())
}
```

### 7.4 Active Window Detection

Linux uses `xdotool`/`swaymsg`/`hyprctl` for foreground window info. On Windows:

```rust
#[cfg(windows)]
fn get_foreground_window_info() -> Result<(String, bool), PlatformError> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd == HWND(0) {
            return Err(PlatformError::Input("No foreground window".into()));
        }

        // Get window title
        let mut title_buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut title_buf);
        let title = String::from_utf16_lossy(&title_buf[..len as usize]);

        // Get executable path
        let mut pid: DWORD = 0;
        let _ = GetWindowThreadProcessId(hwnd, &mut pid);
        let path = get_process_path(pid);

        // Check if elevated
        let elevated = is_process_elevated(pid);

        Ok((title, elevated))
    }
}

fn is_process_elevated(pid: DWORD) -> bool {
    unsafe {
        let h_process = OpenProcess(PROCESS_QUERY_INFORMATION, false, pid);
        if h_process.is_invalid() {
            return false;
        }
        let mut token: HANDLE = HANDLE(0);
        let mut elevation: TOKEN_ELEVATION = std::mem::zeroed();
        let mut size: u32 = 0;

        let result = OpenProcessToken(h_process, TOKEN_QUERY, &mut token)
            && GetTokenInformation(
                token,
                TokenElevation,
                &mut elevation as *mut _ as *mut c_void,
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut size,
            );

        CloseHandle(h_process);
        if !token.is_invalid() {
            CloseHandle(token);
        }

        result && elevation.TokenIsElevated != 0
    }
}
```

### 7.5 App Blacklist (Incognito Mode)

The existing "app blacklist" feature prevents clipboard capture from sensitive apps (password managers, etc.). On Windows:

```rust
#[tauri::command]
fn get_active_window_process() -> Result<String, String> {
    let title = get_foreground_window_info()
        .map_err(|e| e.to_string())?;
    Ok(title)
}
```

### 7.6 Paste Methods — User Choice

- [ ] "Smart paste" — Auto-detect best method (Ctrl+V, type text, UI Automation)
- [ ] "Paste as text" — Always type text (for terminal/editor)
- [ ] "Paste via shortcut" — Always Ctrl+V

```rust
#[tauri::command]
async fn paste(method: String, text: String) -> Result<(), String> {
    match method.as_str() {
        "type" => {
            let sim = platform::input::create_input_simulator()
                .map_err(|e| e.to_string())?;
            sim.type_text(&text)
        }
        "shortcut" => {
            platform::clipboard::create_clipboard_provider()
                .map_err(|e| e.to_string())?
                .set_text(&text)?;
            platform::input::create_input_simulator()
                .map_err(|e| e.to_string())?
                .simulate_paste_shortcut()
        }
        _ => Err("Unknown paste method".into()),
    }
    .map_err(|e| e.to_string())
}
```

### 7.7 Testing on Windows

- [ ] Paste into Notepad — works
- [ ] Paste into browser (Chrome, Firefox, Edge) — works
- [ ] Paste into terminal (Windows Terminal, PowerShell) — works
- [ ] Paste into elevated app (Run as Admin) — fallback works or shows message
- [ ] Rapid successive pastes — no race conditions
- [ ] Keyboard state restored after paste (no stuck modifiers)
- [ ] Non-ASCII text (Unicode, emoji) — pastes correctly

---

## Deliverables

1. Working `Ctrl+V` simulation via `enigo`/`SendInput`
2. UI Automation fallback for elevated targets
3. `uiAccess=true` manifest
4. Active window detection (title, path, elevation status)
5. Multiple paste methods (type, shortcut)
6. Keyboard state management (no stuck keys)

---

## Acceptance Criteria

- [ ] Pasting into foreground app works (Notepad, browser, terminal)
- [ ] Unicode text pastes correctly (including emoji)
- [ ] `uiAccess=true` bypasses UIPI for elevated apps
- [ ] No stuck modifier keys after paste
- [ ] GUI setting for paste method preference
- [ ] Linux paste functionality still works
- [ ] Phase 7 branch committed
