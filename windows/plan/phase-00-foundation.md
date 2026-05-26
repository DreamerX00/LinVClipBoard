# Phase 0: Foundation — Windows Toolchain, Workspace & Research

**Version**: v3.0.0-base  
**Effort**: 2-3 days  
**Dependencies**: None (starting point)

---

## Objective

Set up the Windows development environment, prepare the Rust workspace for cross-platform builds, install required tooling, and establish the foundational project structure for the Windows port.

---

## Tasks

### 0.1 Install Windows Development Tooling

**On Windows build machine / CI runner:**
- [ ] Install Rust via `rustup.rs` (stable channel)
- [ ] Add Windows MSVC target:
  ```bash
  rustup target add x86_64-pc-windows-msvc
  rustup target add aarch64-pc-windows-msvc
  rustup target add i686-pc-windows-msvc
  ```
- [ ] Install Visual Studio Build Tools 2022 (or Visual Studio 2022 Community) with:
  - "Desktop development with C++" workload
  - Windows 10/11 SDK
  - MSVC v143 toolchain
- [ ] Install WebView2 Runtime (if not already present on Windows 10/11)
- [ ] Install Git for Windows
- [ ] Install Node.js 20+ for frontend builds

**On Linux (for cross-compilation):**
- [ ] Install NSIS:
  ```bash
  sudo apt install nsis lld llvm mingw-w64
  ```
- [ ] Install cross-compilation tools:
  ```bash
  rustup target add x86_64-pc-windows-msvc
  cargo install --locked cargo-xwin
  ```
- [ ] Install Tauri CLI:
  ```bash
  npm install -g @tauri-apps/cli@latest
  ```

### 0.2 Update Workspace Configuration

**`Cargo.toml` (workspace root):**
- [ ] Add Windows-specific workspace metadata
- [ ] Verify resolver v2 works for all platforms
- [ ] Add Windows targets to `.cargo/config.toml`:
  ```toml
  [target.x86_64-pc-windows-msvc]
  linker = "rust-lld"
  ```

**`rust-toolchain.toml`:**
- [ ] Add targets list:
  ```toml
  [toolchain]
  channel = "stable"
  targets = [
    "x86_64-unknown-linux-gnu",
    "x86_64-pc-windows-msvc",
    "aarch64-pc-windows-msvc",
  ]
  ```

### 0.3 Create Windows Directory Structure

```
windows/
├── plan/                    # Phase documents
├── hooks/                   # NSIS installer hooks
│   ├── postinstall.nsh
│   └── preuninstall.nsh
├── assets/                  # Windows-specific assets
│   ├── icon.ico
│   ├── icon.ico (all sizes)
│   └── installer.bmp        # Optional installer banner
├── scripts/                 # Windows build/install scripts
│   ├── build.ps1
│   └── install.ps1
└── resources/               # Windows resource files
    ├── app.manifest         # uiAccess=true manifest
    └── app.rc               # Version info resource
```

### 0.4 Verify Linux Builds Still Work

- [ ] `cargo build --release -p clipd -p clipctl` — succeeds
- [ ] `cargo test --workspace` — all tests pass
- [ ] `cargo clippy --workspace -- -D warnings` — clean
- [ ] `cargo fmt --all -- --check` — clean

### 0.5 Set Up Windows VM for Testing

- [ ] Create Windows 11 VM (or use CI)
- [ ] Install Rust + VS Build Tools + Git
- [ ] Clone repository
- [ ] Verify `cargo build --target x86_64-pc-windows-msvc --release -p clipd` succeeds (may fail — this is the baseline)

### 0.6 Verify All Current Dependencies Support Windows

Check each crate in `Cargo.lock` for Windows support:

| Dependency | Windows Support | Notes |
|------------|----------------|-------|
| `tokio` | ✅ Tier 1 | IOCP backend |
| `serde` / `serde_json` | ✅ | Pure Rust |
| `rusqlite` (bundled) | ✅ | Bundled SQLite |
| `uuid` | ✅ | Pure Rust |
| `chrono` | ✅ | Pure Rust |
| `sha2` / `hex` | ✅ | Pure Rust |
| `dirs` | ✅ | Maps to AppData/Roaming |
| `toml` | ✅ | Pure Rust |
| `thiserror` | ✅ | Pure Rust |
| `tracing` | ✅ | Pure Rust |
| `clap` | ✅ | Pure Rust |
| `arboard` | ✅ | Uses clipboard-win on Windows |
| `image` | ✅ | Pure Rust (with png) |
| `tauri` | ✅ | Tier 1 supports Windows |
| `tauri-plugin-*` | ✅ | Desktop plugins |
| `reqwest` | ✅ | schannel or native-tls |
| `syntect` | ✅ | Pure Rust |
| `scraper` | ✅ | Pure Rust |
| `qrcode` | ✅ | Pure Rust |
| `base64` | ✅ | Pure Rust |
| `notify` | ✅ | Windows backend via ReadDirectoryChangesW |
| `html2text` | ✅ | Pure Rust |
| `r2d2` / `r2d2_sqlite` | ✅ | Pure Rust |
| `regex` | ✅ | Pure Rust |
| **`libc`** | ❌ Unix only | Must be gated |
| **`wl-clipboard-rs`** | ❌ Linux/Wayland only | Must be gated |
| **`zbus`** | ❌ Linux/D-Bus only | Must be gated |
| **`x11rb`** (if any) | ❌ Linux/X11 only | Must be gated |

### 0.7 `.gitignore` Updates

- [ ] Add Windows-specific ignore patterns:
  ```gitignore
  # Windows
  *.exe
  *.msi
  *.dll
  *.pdb
  target/windows/
  ```

### 0.8 Research & Documentation

- [ ] Read latest `clipboard_win` 5.x docs (crates.io, docs.rs)
- [ ] Read latest `windows` crate 0.62 docs (if needed)
- [ ] Read Tauri v2 Windows bundle docs (v2.tauri.app)
- [ ] Read Tokio named pipe docs (`tokio::net::windows::named_pipe`)
- [ ] Document any Windows-specific crate version requirements

---

## Deliverables

1. Working `cargo build` for Windows target (even if with stubs)
2. `windows/` directory structure created
3. All Linux builds still passing
4. Inventory of dependencies that need platform gating
5. Windows VM ready for development

---

## Acceptance Criteria

- [ ] `cargo build --target x86_64-pc-windows-msvc --release -p shared` succeeds
- [ ] All existing Linux tests pass
- [ ] Windows directory structure exists
- [ ] Phase 0 branch committed
