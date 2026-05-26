# Phase 1: Platform Abstraction Layer (PAL) — Core Traits

**Version**: v3.0.0-pal  
**Effort**: 3-4 days  
**Dependencies**: Phase 0 complete

---

## Objective

Define platform-agnostic traits for all OS-specific functionality, create the `crates/platform/` crate, and gate existing Linux code behind `#[cfg(unix)]` where appropriate. This is the architectural foundation that enables Windows and (future) macOS builds.

---

## Tasks

### 1.1 Create `crates/platform/` Crate

**`crates/platform/Cargo.toml`:**
```toml
[package]
name = "platform"
version = "0.1.0"
edition = "2021"

[features]
default = []

[dependencies]
# Cross-platform
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }

# Unix (Linux + macOS)
[target.'cfg(unix)'.dependencies]
tokio = { workspace = true, features = ["net", "io-util"] }

# Windows
[target.'cfg(windows)'.dependencies]
tokio = { workspace = true, features = ["net", "io-util"] }
windows = { version = ">=0.59, <=0.62", features = [
    "Win32_Foundation",
    "Win32_System_Pipes",
    "Win32_System_DataExchange",
    "Win32_UI_WindowsAndMessaging",
    "Win32_UI_Input_KeyboardAndMouse",
], optional = true }
clipboard_win = { version = "5", features = ["monitor"], optional = true }

[target.'cfg(windows)'.dependencies.enigo]
version = "0.3"
optional = true
```

### 1.2 Define Core Traits

All traits in `crates/platform/src/lib.rs`:

```rust
/// Error type for platform operations
#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("IPC error: {0}")]
    Ipc(String),
    #[error("Clipboard error: {0}")]
    Clipboard(String),
    #[error("Input simulation error: {0}")]
    Input(String),
    #[error("Service error: {0}")]
    Service(String),
    #[error("Not supported on this platform: {0}")]
    Unsupported(String),
}
```

#### IpcTransport Trait

```rust
/// Platform-specific IPC transport (Unix sockets vs Named Pipes)
#[async_trait::async_trait]
pub trait IpcTransport: Send + Sync {
    async fn connect(&self) -> Result<Box<dyn IpcStream>, PlatformError>;
    async fn bind(&self) -> Result<Box<dyn IpcListener>, PlatformError>;
    fn path(&self) -> String;
}

#[async_trait::async_trait]
pub trait IpcStream: Send {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, PlatformError>;
    async fn write_all(&mut self, buf: &[u8]) -> Result<(), PlatformError>;
}

#[async_trait::async_trait]
pub trait IpcListener: Send {
    async fn accept(&mut self) -> Result<Box<dyn IpcStream>, PlatformError>;
}
```

#### ClipboardProvider Trait

```rust
pub trait ClipboardProvider: Send + Sync {
    fn get_text(&self) -> Result<Option<String>, PlatformError>;
    fn set_text(&self, text: &str) -> Result<(), PlatformError>;
    fn get_image(&self) -> Result<Option<Vec<u8>>, PlatformError>;
    fn set_image(&self, data: &[u8], width: usize, height: usize) -> Result<(), PlatformError>;
    fn get_html(&self) -> Result<Option<String>, PlatformError>;
    fn get_files(&self) -> Result<Option<Vec<String>>, PlatformError>;
}
```

#### ClipboardMonitor Trait

```rust
pub trait ClipboardMonitor: Send {
    /// Blocking wait for next clipboard change
    fn wait_for_change(&mut self) -> Result<(), PlatformError>;
    /// Request shutdown of monitor
    fn shutdown(&mut self);
}
```

#### InputSimulator Trait

```rust
pub trait InputSimulator: Send + Sync {
    fn paste_text(&self, text: &str) -> Result<(), PlatformError>;
    fn type_text(&self, text: &str) -> Result<(), PlatformError>;
    fn simulate_paste_shortcut(&self) -> Result<(), PlatformError>;
}
```

#### ServiceManager Trait

```rust
pub trait ServiceManager: Send + Sync {
    fn register_autostart(&self, app_path: &str) -> Result<(), PlatformError>;
    fn unregister_autostart(&self) -> Result<(), PlatformError>;
    fn is_autostart_enabled(&self) -> Result<bool, PlatformError>;
}
```

### 1.3 Implement Unix (Linux) Backends

Move existing Linux implementations into the PAL:

- [ ] `IpcTransport` for Unix Domain Sockets (`crates/platform/src/unix/ipc.rs`)
- [ ] `ClipboardProvider` using `arboard` + `wl-clipboard-rs` (`crates/platform/src/unix/clipboard.rs`)
- [ ] `ClipboardMonitor` using polling or inotify (`crates/platform/src/unix/monitor.rs`)
- [ ] `InputSimulator` using `xdotool`/`wtype`/`ydotool` (`crates/platform/src/unix/input.rs`)
- [ ] `ServiceManager` using systemd (`crates/platform/src/unix/service.rs`)

### 1.4 Implement Windows Backends (Stubs First, Real in Future Phases)

- [ ] `IpcTransport` for Named Pipes (`crates/platform/src/windows/ipc.rs`) — stub
- [ ] `ClipboardProvider` using `clipboard_win` (`crates/platform/src/windows/clipboard.rs`) — stub
- [ ] `ClipboardMonitor` using `clipboard_win::Monitor` (`crates/platform/src/windows/monitor.rs`) — stub
- [ ] `InputSimulator` using `enigo` + `SendInput` (`crates/platform/src/windows/input.rs`) — stub
- [ ] `ServiceManager` using HKCU Run key (`crates/platform/src/windows/service.rs`) — stub

### 1.5 Create Platform Selection Module

```rust
// crates/platform/src/lib.rs

// Platform-specific type aliases
#[cfg(unix)]
pub type RecommendedIpcTransport = unix::IpcTransport;
#[cfg(windows)]
pub type RecommendedIpcTransport = windows::IpcTransport;

// Factory function
pub fn create_clipboard_provider() -> Result<Box<dyn ClipboardProvider>, PlatformError>;
pub fn create_clipboard_monitor() -> Result<Box<dyn ClipboardMonitor>, PlatformError>;
pub fn create_input_simulator() -> Result<Box<dyn InputSimulator>, PlatformError>;
pub fn create_service_manager() -> Result<Box<dyn ServiceManager>, PlatformError>;
```

### 1.6 Gate Existing Linux-Specific Code

- [ ] `crates/shared/Cargo.toml`: Gate `libc` with `[target.'cfg(unix)'.dependencies]`
- [ ] `crates/shared/src/config.rs`: Platform-specific socket path
  ```rust
  pub fn socket_path() -> PathBuf {
      #[cfg(unix)]
      { /* existing Linux logic */ }
      #[cfg(windows)]
      { /* Windows named pipe path */ }
  }
  ```
- [ ] `crates/shared/src/ipc.rs`: Abstract IPC connection
- [ ] `crates/clipd/src/dbus_service.rs`: Gate behind `#[cfg(feature = "dbus")]`
- [ ] `crates/clipd/src/monitor.rs`: Move clipboard logic to PAL
- [ ] `crates/clipd/src/server.rs`: Use `IpcTransport` trait

### 1.7 Update Cargo Workspace

- [ ] Add `platform` to workspace members in root `Cargo.toml`
- [ ] Update `clipd` to depend on `platform` crate
- [ ] Update `linvclip-ui` to depend on `platform` crate (where needed)
- [ ] Update `clipctl` to depend on `platform` crate (where needed)

### 1.8 Verify Linux Build (Critical)

- [ ] `cargo build --release -p clipd -p clipctl` — succeeds (no regressions)
- [ ] `cargo test --workspace` — all tests pass
- [ ] `cargo clippy --workspace -- -D warnings` — clean

---

## Deliverables

1. `crates/platform/` crate with all trait definitions
2. Platform-agnostic type aliases
3. Existing Linux code still works (no functional changes)
4. Windows backend stubs compile (even if they return `Unsupported` errors)
5. `socket_path()` returns valid paths per platform

---

## Acceptance Criteria

- [ ] All `libc` references are gated behind `cfg(unix)`
- [ ] `cargo build --release` (Linux) succeeds with no warnings
- [ ] `cargo build --target x86_64-pc-windows-msvc --release` succeeds (all crates)
- [ ] Phase 1 branch committed
