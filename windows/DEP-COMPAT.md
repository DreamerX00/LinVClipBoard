# Dependency Compatibility Matrix — Windows Port

> **Status**: Phase 0 — Foundation Audit  
> **Last Updated**: May 2026

Every dependency in the workspace is checked for Windows support. This matrix drives the `#[cfg(unix)]` / `#[cfg(windows)]` gating in Phase 1.

---

## Legend

| Icon | Meaning |
|------|---------|
| ✅ | Tier 1 — Windows supported unconditionally |
| ⚠️ | Conditional — Windows works but needs feature flags |
| ❌ | Unix-only — must be gated with #[cfg(unix)] |
| 🔄 | Needs replacement — use Windows equivalent |

---

## Workspace Dependencies

### `crates/shared/Cargo.toml`

| Crate | Ver | Windows | Notes |
|-------|-----|---------|-------|
| serde | 1 | ✅ | Pure Rust, zero platform deps |
| serde_json | 1 | ✅ | Pure Rust |
| rusqlite | 0.31 | ✅ | Bundled SQLite, cross-platform |
| uuid | 1 | ✅ | Pure Rust |
| chrono | 0.4 | ✅ | Pure Rust |
| sha2 | 0.10 | ✅ | Pure Rust |
| hex | 0.4 | ✅ | Pure Rust |
| dirs | 5 | ✅ | Maps to `%APPDATA%` on Windows |
| toml | 0.8 | ✅ | Pure Rust |
| thiserror | 1 | ✅ | Pure Rust |
| tracing | 0.1 | ✅ | Pure Rust |
| **libc** | 0.2 | ❌ | Unix-only, used for `getuid()` + socket path |
| tokio | 1 | ✅ | IOCP on Windows, net/io-util/sync all work |
| r2d2 | 0.8 | ✅ | Pure Rust |
| r2d2_sqlite | 0.24 | ✅ | Cross-platform |
| regex | 1.12 | ✅ | Pure Rust |

**Key changes needed in shared**:
- `libc` → gate behind `#[cfg(unix)]` in `config.rs` (`socket_path()` uses `libc::getuid()`)
- `socket_path()` → Windows uses named pipe path `\\.\pipe\LinVClipBoard`

### `crates/clipd/Cargo.toml`

| Crate | Ver | Windows | Notes |
|-------|-----|---------|-------|
| shared | path | ✅ | After port |
| **arboard** | 3 | ⚠️ | Windows needs `clipboard-win` feature (not `wayland-data-control`) |
| tokio | 1 | ✅ | IOCP backend |
| tokio-util | 0.7 | ✅ | Pure Rust |
| serde + json | 1 | ✅ | |
| toml | 0.8 | ✅ | |
| sha2 | 0.10 | ✅ | |
| hex | 0.4 | ✅ | |
| chrono | 0.4 | ✅ | |
| tracing + subscriber | 0.1/0.3 | ✅ | |
| anyhow | 1 | ✅ | |
| dirs | 5 | ✅ | |
| **libc** | 0.2 | ❌ | Unix-only |
| image | 0.25 | ✅ | Pure Rust + png feature |
| uuid | 1 | ✅ | |
| **notify** | 6 | ✅ | Windows: `ReadDirectoryChangesW` |
| html2text | 0.14 | ✅ | Pure Rust |
| **wl-clipboard-rs** | 0.9 | ❌ | Wayland-only, must gate |
| **zbus** | 4 | ❌ | Linux D-Bus only |

**Key changes needed in clipd**:
- `monitor.rs` — entire file is Wayland + X11-specific. New `monitor_windows.rs` needed.
- `server.rs` — `UnixListener` → `NamedPipeServer`, `Clipboard::new()` feature flag fix
- `main.rs` — signal handling: Unix signals → Windows `ctrl_c()` + `SetConsoleCtrlHandler`
- `dbus_service.rs` — already conditionally compiled (behind `dbus` feature)

### `crates/clipctl/Cargo.toml`

| Crate | Ver | Windows | Notes |
|-------|-----|---------|-------|
| shared | path | ✅ | After port |
| clap | 4 | ✅ | Pure Rust |
| clap_complete | 4 | ✅ | Shell completions (PowerShell supported) |
| clap_mangen | 0.2 | ✅ | |
| tokio | 1 | ✅ | |
| serde_json | 1 | ✅ | |
| chrono | 0.4 | ✅ | |
| colored | 2 | ✅ | Pure Rust, Windows console colors via `winconapi` |

**Minimal changes**: IPC connection path (Unix socket → Named Pipe).

### `crates/linvclip-ui/src-tauri/Cargo.toml`

| Crate | Ver | Windows | Notes |
|-------|-----|---------|-------|
| tauri | 2 | ✅ | Tier 1 Windows (NSIS/MSI bundler) |
| tauri-plugin-global-shortcut | 2 | ✅ | Windows via registered hotkeys |
| tauri-plugin-shell | 2 | ✅ | |
| serde + json | 1 | ✅ | |
| tokio | 1 | ✅ | |
| shared | path | ✅ | |
| base64 | 0.22 | ✅ | |
| **arboard** | 3 | ⚠️ | Needs `clipboard-win` feature |
| reqwest | 0.12 | ✅ | Uses `schannel` on Windows by default |
| dirs | 5 | ✅ | |
| tokio-stream | 0.1 | ✅ | |
| qrcode | 0.14 | ✅ | |
| image | 0.25 | ✅ | |
| syntect | 5 | ✅ | |
| scraper | 0.22 | ✅ | |
| html2text | 0.14 | ✅ | |
| yaml_serde | 0.10 | ✅ | |
| toml | 1.1 | ✅ | |

---

## Windows-Specific Target Dependencies (to add)

| Phase | Crate | Purpose | Windows Feature/Branch |
|-------|-------|---------|----------------------|
| 1 | `clipboard_win = "5"` | Clipboard get/set/monitor with full format enumeration | cfg(windows) |
| 1 | `windows = "0.62"` (optional) | Win32 API bindings for advanced features | cfg(windows) |
| 2 | `tokio::net::windows::named_pipe` | IPC server (built into tokio) | cfg(windows) |
| 7 | `enigo = "0.3"` | Cross-platform input simulation | cfg(windows) |
| 7 | `windows::UI::Automation` | UI Automation fallback for elevated targets | cfg(windows) |

---

## Summary: Files Requiring `#[cfg]` Gating

| File | Linux-only Code | Windows Replacement |
|------|----------------|-------------------|
| `shared/src/config.rs` | `libc::getuid()`, `/run/user/` | `\\.\pipe\LinVClipBoard` |
| `shared/src/ipc.rs` | `UnixStream`, `UnixListener` | `NamedPipeClient`, `NamedPipeServer` |
| `clipd/src/monitor.rs` | `wl-clipboard-rs`, `xdotool`, `swaymsg`, `hyprctl`, `arboard::Clipboard` | `clipboard_win::Monitor` + `clipboard_win::Clipboard` |
| `clipd/src/main.rs` | `tokio::signal::unix`, `libc`, socket file cleanup | `ctrl_c()` + `SetConsoleCtrlHandler` |
| `clipd/src/server.rs` | `UnixListener`, `wl-clipboard-rs::copy`, `std::os::unix::PermissionsExt` | `tokio::net::windows::named_pipe`, remove wayland paste |
| `clipd/src/dbus_service.rs` | `zbus` | N/A (not ported, not needed) |
| `linvclip-ui/src-tauri/src/lib.rs` | TBD after reading | Tauri Windows commands |
