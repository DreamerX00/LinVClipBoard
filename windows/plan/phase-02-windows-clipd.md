# Phase 2: Windows Clipboard Monitor Daemon (clipd)

**Version**: v3.0.0-daemon  
**Effort**: 4-5 days  
**Dependencies**: Phase 1 (PAL traits)

---

## Objective

Implement the Windows-native clipboard daemon (`clipd`) using the PAL traits. This is the core of the clipboard manager — it monitors clipboard changes, stores history in SQLite, and serves IPC requests. On Windows, it runs as a user-level background process (not a system service).

---

## Tasks

### 2.1 Windows Clipboard Monitor Implementation

Implement `ClipboardMonitor` for Windows in `crates/platform/src/windows/monitor.rs`:

```rust
use clipboard_win::monitor::Monitor;

pub struct WindowsClipboardMonitor {
    monitor: Option<Monitor>,
    shutdown_tx: Option<Sender<()>>,
}

impl ClipboardMonitor for WindowsClipboardMonitor {
    fn wait_for_change(&mut self) -> Result<(), PlatformError> {
        loop {
            match self.monitor.as_mut().unwrap().recv() {
                Ok(true) => return Ok(()),
                Ok(false) => return Err(PlatformError::Clipboard("shutdown".into())),
                Err(e) => {
                    tracing::warn!("Clipboard monitor error: {e}");
                    continue;
                }
            }
        }
    }

    fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

pub fn create_clipboard_monitor() -> Result<WindowsClipboardMonitor, PlatformError> {
    let monitor = Monitor::new()
        .map_err(|e| PlatformError::Clipboard(format!("Failed to create monitor: {e}")))?;
    let (tx, _rx) = channel();
    Ok(WindowsClipboardMonitor {
        monitor: Some(monitor),
        shutdown_tx: Some(tx),
    })
}
```

### 2.2 Clipboard Data Reading

Implement `ClipboardProvider` for Windows in `crates/platform/src/windows/clipboard.rs`:

```rust
use clipboard_win::{formats, get_clipboard, set_clipboard, Clipboard};

pub struct WindowsClipboardProvider;

impl ClipboardProvider for WindowsClipboardProvider {
    fn get_text(&self) -> Result<Option<String>, PlatformError> {
        let _clip = Clipboard::new_attempts(5)
            .map_err(|e| PlatformError::Clipboard(format!("Open: {e}")))?;

        if clipboard_win::is_format_avail(formats::CF_UNICODETEXT) {
            let text: String = get_clipboard(formats::Unicode)
                .map_err(|e| PlatformError::Clipboard(format!("GetText: {e}")))?;
            Ok(Some(text))
        } else {
            Ok(None)
        }
    }

    fn get_image(&self) -> Result<Option<Vec<u8>>, PlatformError> {
        // Read CF_DIB or CF_DIBV5, convert to PNG bytes
        // Requires image crate for format conversion
        unimplemented!("Phase 2.5")
    }

    fn get_html(&self) -> Result<Option<String>, PlatformError> {
        let html_format = clipboard_win::raw::register_format("HTML Format")
            .map_err(|e| PlatformError::Clipboard(format!("RegFormat: {e}")))?;
        if clipboard_win::is_format_avail(html_format) {
            let html: String = get_clipboard(formats::RawData(html_format))
                .map_err(|e| PlatformError::Clipboard(format!("GetHTML: {e}")))?;
            // Parse CF_HTML header to extract fragment
            Ok(Some(extract_html_fragment(&html)))
        } else {
            Ok(None)
        }
    }

    fn set_text(&self, text: &str) -> Result<(), PlatformError> {
        set_clipboard(formats::Unicode, text)
            .map_err(|e| PlatformError::Clipboard(format!("SetText: {e}")))?;
        Ok(())
    }
}
```

### 2.3 Windows clipd Main Entry Point

Implement `crates/clipd/src/windows_main.rs` (or add `#[cfg(windows)]` to existing main):

```rust
#[cfg(windows)]
mod platform {
    use tokio::net::windows::named_pipe::ServerOptions;
    // ...

    pub async fn run_daemon() -> Result<()> {
        // 1. Initialize clipboard monitor
        // 2. Create named pipe server
        // 3. Run event loop:
        //    - On clipboard change: read content, deduplicate, store in DB
        //    - On IPC connect: handle requests (list, search, pin, delete)
        //    - On shutdown signal: graceful exit
    }
}
```

**Key differences from Linux version:**
- No SIGTERM handling — use `Ctrl-C` or named shutdown pipe
- No systemd integration — process stays alive via `main()`
- Named pipe for IPC instead of Unix socket
- Use `SetConsoleCtrlHandler` for graceful shutdown

### 2.4 Windows Signal Handling

```rust
#[cfg(windows)]
fn setup_signal_handler() {
    unsafe {
        SetConsoleCtrlHandler(Some(handler), TRUE);
    }
}

#[cfg(windows)]
unsafe extern "system" fn handler(ctrl_type: DWORD) -> BOOL {
    match ctrl_type {
        CTRL_C_EVENT | CTRL_BREAK_EVENT | CTRL_CLOSE_EVENT => {
            // Signal graceful shutdown
            TRUE
        }
        _ => FALSE,
    }
}
```

### 2.5 Daemon Configuration on Windows

Update `crates/shared/src/config.rs`:

```rust
pub fn data_dir() -> PathBuf {
    #[cfg(unix)]
    {
        dirs::data_dir().unwrap_or_else(|| PathBuf::from("/tmp")).join("linvclipboard")
    }
    #[cfg(windows)]
    {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from(r"C:\Users\Default\AppData\Roaming"))
            .join("LinVClipBoard")
    }
}

pub fn socket_path() -> PathBuf {
    #[cfg(unix)]
    {
        PathBuf::from(format!("/run/user/{}/linvclip.sock", unsafe { libc::getuid() }))
    }
    #[cfg(windows)]
    {
        PathBuf::from(r"\\.\pipe\LinVClipBoard")
    }
}
```

### 2.6 Database Layer — No Changes Needed

The `rusqlite`-based database (`crates/shared/src/db.rs`) is already cross-platform. Verify:
- [ ] `cargo build --target x86_64-pc-windows-msvc -p shared` succeeds
- [ ] SQLite FTS5 works on Windows (bundled SQLite includes FTS5)

### 2.7 Windows-Specific Error Handling

- [ ] Handle `OpenClipboard` failures (other app has it open)
- [ ] Add retry logic with exponential backoff for clipboard contention
- [ ] Handle `ERROR_CLIPBOARD_NOT_OPEN` gracefully

### 2.8 Manual Testing on Windows

- [ ] `clipd.exe` starts and stays running
- [ ] Clipboard changes are detected (copy text → appear in DB)
- [ ] Images are detected (copy image → appear in DB)
- [ ] Multiple rapid copies are all captured (deduplication works)
- [ ] Shutdown via Ctrl-C works cleanly
- [ ] SQLite database is created in `%APPDATA%\LinVClipBoard\`
- [ ] Named pipe is accessible at `\\.\pipe\LinVClipBoard`

---

## Deliverables

1. Fully functional `clipd.exe` that:
   - Monitors clipboard via `AddClipboardFormatListener`
   - Stores text history in SQLite
   - Serves IPC requests via named pipe
   - Handles graceful shutdown
2. Windows clipboard provider implementation (text + HTML)
3. Image clipboard support (basic PNG read)

---

## Acceptance Criteria

- [ ] `clipd.exe` detects clipboard changes (text, images)
- [ ] History is persisted in SQLite on Windows
- [ ] Named pipe is created and accessible
- [ ] Graceful shutdown works (Ctrl-C)
- [ ] `cargo build --target x86_64-pc-windows-msvc --release -p clipd` succeeds
- [ ] Linux build still works without regressions
- [ ] Phase 2 branch committed
