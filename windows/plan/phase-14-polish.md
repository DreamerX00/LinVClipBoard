# Phase 14: Post-Release Polish & Windows-Specific Features

**Version**: v3.0.0-windows  
**Effort**: 3-4 days  
**Dependencies**: Phase 12 (Testing fixes)

---

## Objective

Polish the Windows experience with Windows-specific features that go beyond the Linux version. This includes DPI awareness, Windows 11 snap layouts, toast notifications, jump lists, Windows Hello integration, and performance optimization.

---

## Tasks

### 14.1 DPI Awareness & High-DPI Support

**PerMonitorV2 DPI awareness** (already set in manifest from Phase 11):

Verify:
- [ ] Overlay scales correctly on 150%, 200%, 300% DPI
- [ ] Font renders crisply at all DPIs
- [ ] Icons look sharp (use vector/256px icons)
- [ ] Window position is correct on multi-monitor with mixed DPIs
- [ ] Tray icon handles DPI changes

```rust
// Handle DPI changes at runtime
#[cfg(windows)]
fn handle_dpi_change(window: &Window, new_dpi: u32) {
    let scale = new_dpi as f64 / 96.0;
    let _ = window.emit("dpi-changed", scale);
}
```

### 14.2 Windows 11 Visual Integration

- [ ] **Mica/acrylic background** — Use Windows 11's Mica material for the overlay background
- [ ] **Rounded corners** — Match Windows 11 rounded corner aesthetic
- [ ] **Snap layouts** — Support for Windows 11 snap layouts

```css
/* CSS — Windows 11 Mica effect */
.windows-mica {
  background: transparent;
  backdrop-filter: blur(30px);
  /* Fallback for non-Mica: semi-transparent dark */
  background-color: rgba(32, 32, 32, 0.85);
}
```

### 14.3 Windows Toast Notifications

Use Windows native toast notifications instead of in-app notifications:

```rust
#[cfg(windows)]
fn show_windows_toast(title: &str, body: &str) -> Result<(), PlatformError> {
    use windows::UI::Notifications::*;
    use windows::Data::Xml::Dom::*;

    // Create toast XML
    let doc = XmlDocument::new()
        .map_err(|e| PlatformError::Service(e.to_string()))?;
    doc.LoadXml(&format!(
        r#"<toast><visual><binding template="ToastGeneric">
            <text>{}</text><text>{}</text>
        </binding></visual></toast>"#,
        title, body
    ))?;

    let toast = ToastNotification::CreateToastNotification(&doc)
        .map_err(|e| PlatformError::Service(e.to_string()))?;

    ToastNotificationManager::CreateToastNotifierWithId("LinVClipBoard")?
        .Show(&toast)?;

    Ok(())
}
```

### 14.4 Windows Jump List

Add a jump list to the taskbar icon for quick actions:

```rust
#[cfg(windows)]
fn setup_jump_list() -> Result<(), PlatformError> {
    use windows::UI::Shell::JumpList;

    let jump_list = JumpList::GetForCurrentApp()
        .map_err(|e| PlatformError::Service(e.to_string()))?;

    // Clear existing items
    jump_list.RemoveAll()?;

    // Add jump list items
    let items = vec![
        JumpListItem::CreateWithArguments("show", "Show Clipboard")?,
        JumpListItem::CreateWithArguments("search", "Search History")?,
        JumpListItem::CreateWithArguments("clear", "Clear History")?,
    ];

    jump_list.AddUserTasks(&items)?;
    jump_list.Commit()?;

    Ok(())
}
```

### 14.5 Windows Hello / Device Lock Integration

Pause clipboard monitoring when the workstation is locked:

```rust
#[cfg(windows)]
fn register_session_notification() {
    unsafe {
        // Subscribe to session lock/unlock events
        WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION);

        // On WM_WTSSESSION_CHANGE:
        //   WTS_SESSION_LOCK → pause clipboard monitoring
        //   WTS_SESSION_UNLOCK → resume clipboard monitoring
    }
}
```

### 14.6 Single Instance Enforcement

Prevent multiple instances of the app:

```rust
#[cfg(windows)]
fn enforce_single_instance() -> bool {
    use windows::Win32::Foundation::*;
    use windows::Win32::System::LibraryLoader::*;

    unsafe {
        let mutex_name = h!("Local\\LinVClipBoard_SingleInstanceMutex");
        let handle = CreateMutexW(None, false, mutex_name);

        if GetLastError() == ERROR_ALREADY_EXISTS {
            // Another instance is running — activate its window
            let hwnd = FindWindowW(None, h!("LinVClipBoard"));
            if hwnd != HWND(0) {
                SetForegroundWindow(hwnd);
            }
            return false; // Exit this instance
        }
        true // First instance, continue
    }
}
```

### 14.7 Performance Optimization

- [ ] **Lazy DB loading** — Don't load full history on startup, load on demand
- [ ] **Image thumbnailing** — Generate smaller thumbnails for display, keep originals for paste
- [ ] **Named pipe buffer tuning** — Optimize pipe buffer sizes (PIPE_SIZE = 65536)
- [ ] **WebView2 warmup** — Pre-warm WebView2 to reduce overlay open time
- [ ] **Memory limits** — Implement Windows-specific memory limits (max 50MB daemon)

**Windows performance targets:**
| Metric | Target |
|--------|--------|
| Daemon memory (idle) | < 15 MB |
| Daemon CPU (idle) | 0% |
| Overlay show time (cold) | < 300 ms |
| Overlay show time (warm) | < 100 ms |
| Clipboard change → store | < 50 ms |
| DB query (10k items) | < 30 ms |

### 14.8 Windows Accessibility

- [ ] **Narrator support** — UI Automation patterns for screen reader
- [ ] **High contrast mode** — Detect and adapt to Windows high contrast theme
- [ ] **Keyboard navigation** — Full keyboard accessibility (Tab, Enter, Arrow keys)
- [ ] **Reduce motion** — Respect "Show animations in Windows" setting
- [ ] **Focus mode** — Don't steal focus when overlay is dismissed

### 14.9 Error Reporting & Diagnostics

- [ ] **Windows Event Log integration** — Log critical errors to Windows Event Log
- [ ] **Crash dump generation** — Use `MiniDumpWriteDump` on unhandled exceptions
- [ ] **Diagnostic report** — `clipctl diagnose` command that collects Windows-specific info
- [ ] **Debug logging** — File-based logging to `%APPDATA%\LinVClipBoard\logs\`

### 14.10 Windows-Specific Features (Future Roadmap)

Features for post-3.0.0 releases:

- [ ] **Microsoft Office integration** — Paste formatting-aware content into Office apps
- [ ] **PowerToys Run plugin** — Add LinVClipBoard as a PowerToys plugin
- [ ] **Auto-paste on selection** — Middle-click paste simulation (like Linux)
- [ ] **Clipboard sync via Microsoft account** — Integrate with Windows clipboard roaming
- [ ] **Windows Copilot integration** — AI-powered clipboard actions via Copilot

---

## Deliverables

1. High-DPI and multi-monitor support verified
2. Windows 11 visual integration (Mica, rounded corners)
3. Toast notifications on clipboard events
4. Jump list in taskbar
5. Single instance enforcement
6. Performance optimized to targets
7. Accessibility features implemented

---

## Acceptance Criteria

- [ ] App works correctly at 100%, 150%, 200%, 300% DPI
- [ ] Multi-monitor with mixed DPIs works correctly
- [ ] Windows 11 Mica/acrylic effect renders
- [ ] Taskbar jump list shows recent actions
- [ ] Single instance enforced (second instance activates first)
- [ ] Performance meets targets
- [ ] Narrator reads UI elements correctly
- [ ] High contrast mode works
- [ ] Phase 14 branch committed

---

## Windows Port — Complete

After Phase 14, LinVClipBoard is a first-class Windows application with:

- Feature parity with the Linux version
- Windows-native UI and behavior
- Published on winget and Microsoft Store
- CI/CD building and releasing Windows builds
- NSIS installer for easy distribution
- Auto-update via Tauri updater
- Code signed installer
- Accessible and high-DPI aware
