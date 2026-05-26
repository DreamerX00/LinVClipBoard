# LinVClipBoard — Windows Port Master Plan

> **Version**: 3.0.0 → 3.*.0-windows  
> **Start Date**: May 2026  
> **Approach**: Phase-by-phase — research, implement, test, iterate, then next phase  
> **Rule**: Zero paid dependencies. Everything free/open-source.  
> **Target**: Windows 10 (1809+) and Windows 11 — x86_64, ARM64

---

## Overview

Port LinVClipBoard from Linux-only to first-class Windows support. This means native `.exe` builds, proper Windows integration (clipboard monitoring, autostart, system tray, input simulation), and a seamless user experience that matches (and in some ways surpasses) the Linux version.

The port involves 14 phases organized into 3 tracks:

| Track | Phases | Description |
|-------|--------|-------------|
| **Foundation** | 0–2 | Research, platform abstraction traits, build system |
| **Core Port** | 3–6 | Daemon, IPC, CLI, Tauri GUI adaptation |
| **Windows Integration** | 7–10 | Input simulation, packaging, updates, CI/CD |
| **Release** | 11–13 | Icons/store, testing, polish, publishing |

---

## Technology Stack (Windows Edition)

| Category | Technology | Purpose |
|----------|-----------|---------|
| **GUI** | Tauri v2 (React 19 frontend) | Overlay window, system tray |
| **Daemon** | Rust + tokio | Background clipboard monitor |
| **IPC** | Tokio Named Pipes | Daemon ↔ GUI / CLI communication |
| **Clipboard** | `clipboard_win` (5.x) + `AddClipboardFormatListener` | Get/set/monitor clipboard |
| **Clipboard (cross-platform alt)** | `arboard` (3.6+) | Text/image get/set (simpler, less control) |
| **Input Simulation** | `enigo` + `SendInput` + UI Automation | Paste into apps |
| **Database** | SQLite via `rusqlite` (bundled) | Clipboard history |
| **Autostart** | `HKCU\...\Run` registry key | User-level autostart |
| **Packaging** | NSIS installer (Tauri v2 bundler) | .exe distribution |
| **Updates** | Tauri v2 updater plugin | Auto-update |
| **Signing** | OV/EV Code Signing Certificate | Authenticode signing |
| **CI** | GitHub Actions (windows-latest) | Build + test + release |

---

## Phase Map

| Phase | File | Features | Version Target | Est. Effort |
|-------|------|----------|----------------|-------------|
| **Phase 0** | [phase-00-foundation.md](phase-00-foundation.md) | Research, toolchain, workspace setup, Windows dev env | v3.0.0-base | 2-3 days |
| **Phase 1** | [phase-01-pal-traits.md](phase-01-pal-traits.md) | Platform Abstraction Layer traits (IPC, Clipboard, Input, Service) | v3.0.0-pal | 3-4 days |
| **Phase 2** | [phase-02-windows-clipd.md](phase-02-windows-clipd.md) | Windows native clipboard daemon (monitor + server) | v3.0.0-daemon | 4-5 days |
| **Phase 3** | [phase-03-ipc-named-pipes.md](phase-03-ipc-named-pipes.md) | Windows Named Pipe IPC transport implementation | v3.0.0-ipc | 2-3 days |
| **Phase 4** | [phase-04-clipctl.md](phase-04-clipctl.md) | Windows CLI client (clipctl port) | v3.0.0-cli | 2 days |
| **Phase 5** | [phase-05-tauri-gui.md](phase-05-tauri-gui.md) | Tauri GUI Windows adaptation (tauri.conf.json, commands, tray) | v3.0.0-gui | 4-5 days |
| **Phase 6** | [phase-06-autostart.md](phase-06-autostart.md) | Windows autostart (HKCU Run), service integration | v3.0.0-autostart | 1-2 days |
| **Phase 7** | [phase-07-input-simulation.md](phase-07-input-simulation.md) | Windows input simulation (SendInput, enigo, UI Automation fallback) | v3.0.0-input | 3-4 days |
| **Phase 8** | [phase-08-packaging.md](phase-08-packaging.md) | NSIS installer, code signing, bundle configuration | v3.0.0-pkg | 3-4 days |
| **Phase 9** | [phase-09-updater.md](phase-09-updater.md) | Windows auto-updater (Tauri plugin) | v3.0.0-update | 2-3 days |
| **Phase 10** | [phase-10-cicd.md](phase-10-cicd.md) | GitHub Actions CI/CD, cross-compilation, release pipeline | v3.0.0-ci | 2-3 days |
| **Phase 11** | [phase-11-icons-assets.md](phase-11-icons-assets.md) | Windows icons (.ico), assets, desktop integration, manifests | v3.0.0-assets | 1-2 days |
| **Phase 12** | [phase-12-testing.md](phase-12-testing.md) | Windows integration testing, QA, edge cases, bug bash | v3.0.0-test | 3-4 days |
| **Phase 13** | [phase-13-store-winget.md](phase-13-store-winget.md) | Microsoft Store submission, winget publishing | v3.0.0-store | 2-3 days |
| **Phase 14** | [phase-14-polish.md](phase-14-polish.md) | Windows-specific features, DPI, HiDPI, accessibility, performance | v3.0.0-windows | 3-4 days |

---

## Dependency Graph

```
Phase 0 (Foundation — toolchain, workspace, research)
  │
  ├── Phase 1 (PAL Traits — IPC, Clipboard, Input, Service traits)
  │     │
  │     ├── Phase 2 (Windows clipd daemon — clipboard monitor + server)
  │     │     │
  │     │     ├── Phase 3 (Named Pipe IPC transport)
  │     │     │     │
  │     │     │     ├── Phase 4 (clipctl CLI port)
  │     │     │     │
  │     │     │     └── Phase 5 (Tauri GUI adaptation)
  │     │     │           │
  │     │     │           └── Phase 6 (Autostart + service mgmt)
  │     │     │                 │
  │     │     │                 └── Phase 7 (Input simulation + pasting)
  │     │     │                       │
  │     │     │                       └── Phase 8 (NSIS packaging + signing)
  │     │     │                             │
  │     │     │                             ├── Phase 9 (Auto-updater)
  │     │     │                             │
  │     │     │                             ├── Phase 10 (CI/CD pipeline)
  │     │     │                             │
  │     │     │                             └── Phase 11 (Icons, assets, manifest)
  │     │     │                                   │
  │     │     │                                   └── Phase 12 (Testing + QA)
  │     │     │                                         │
  │     │     │                                         ├── Phase 13 (Store + winget)
  │     │     │                                         │
  │     │     │                                         └── Phase 14 (Polish + DPI + perf)
  │     │     │
  │     │     └── (Tauri also connects directly to clipd via Named Pipes)
  │     │
  │     └── (All phases require PAL traits from Phase 1)
```

---

## Key Architectural Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **IPC** | Tokio Named Pipes (not TCP, not gRPC) | Native Windows IPC, same-async runtime as existing Linux code, no extra deps |
| **Clipboard Monitoring** | `clipboard_win` Monitor (AddClipboardFormatListener) | Event-driven, zero CPU when idle, mature Rust crate (30M+ downloads) |
| **Clipboard Get/Set** | `clipboard_win` (not arboard) | Need full format enumeration for rich content (HTML, files, images) |
| **Input Simulation** | `enigo` + `SendInput` + UI Automation fallback | enigo for basic, SendInput for reliability, UIA for elevated targets |
| **Autostart** | HKCU\...\Run (not Windows Service) | No admin elevation needed, per-user scope, matches clipboard access scope |
| **Packaging** | NSIS (not MSI) | Cross-compilable from Linux, per-user install, simpler customization |
| **Updates** | Tauri v2 updater plugin | Built-in, works with NSIS, no external service needed |
| **Database** | SQLite via rusqlite (same as Linux) | Cross-platform, no changes needed |
| **Frontend** | React (unchanged) | Zero frontend changes — all platform differences in Rust layer |

---

## File Inventory — Every File That Needs Changes

| File | Phase | Change |
|------|-------|--------|
| `Cargo.toml` | 0 | Add Windows targets, resolver config |
| `Makefile` | 0 | Add Windows build targets |
| `rust-toolchain.toml` | 0 | Add Windows targets |
| `crates/shared/Cargo.toml` | 1 | Gate `libc` to unix, add Windows named pipe dep |
| `crates/shared/src/config.rs` | 1 | Platform-specific paths (AppData vs XDG) |
| `crates/shared/src/ipc.rs` | 3 | Split into platform IPC modules |
| `crates/clipd/Cargo.toml` | 2 | Conditional deps per platform |
| `crates/clipd/src/main.rs` | 2 | Platform-specific startup, signal handling |
| `crates/clipd/src/monitor.rs` | 2 | Windows clipboard monitor |
| `crates/clipd/src/server.rs` | 3 | Named pipe server |
| `crates/clipd/src/dbus_service.rs` | 1 | Gate behind linux only |
| `crates/clipctl/Cargo.toml` | 4 | No changes needed (already cross-platform) |
| `crates/clipctl/src/main.rs` | 4 | Minor IPC path changes |
| `crates/linvclip-ui/src-tauri/Cargo.toml` | 5 | Windows-specific deps |
| `crates/linvclip-ui/src-tauri/tauri.conf.json` | 5 | Add Windows bundle config |
| `crates/linvclip-ui/src-tauri/src/lib.rs` | 5 | Windows-specific commands |
| `crates/linvclip-ui/src-tauri/src/main.rs` | 5 | Minor adjustments |
| `crates/linvclip-ui/src-tauri/capabilities/default.json` | 5 | Windows permissions |
| `.github/workflows/ci.yml` | 10 | Add Windows jobs |
| `install/` | 6 | Windows equivalents of install scripts |
| `packaging/` | 8 | Windows packaging scripts |
| `windows/` | All | New Windows-specific directory |

---

## Execution Protocol

For each phase:
1. Read the phase `.md` file
2. Research any unknowns (documentation, crate APIs)
3. Implement all items in the phase
4. Build: `cargo build --release`
5. If Windows native: `cargo build --target x86_64-pc-windows-msvc --release`
6. Test on Windows VM/CI
7. User provides feedback
8. Address feedback
9. Commit with standard message
10. Move to next phase

---

## Ground Rules

1. **No paid dependencies** — Everything free/open-source
2. **Minimal frontend changes** — All platform differences in Rust layer
3. **Backward compatible** — Existing Linux builds must still work
4. **Feature parity first** — Match Linux features before adding Windows-specific ones
5. **Security** — No storing clipboard in plaintext without encryption
6. **Performance** — Overlay must open in <200ms, memory <50MB
7. **Zero-polling** — Event-driven clipboard monitoring only

---

## Version Milestones

| Version | Codename | Theme |
|---------|----------|-------|
| v3.0.0-base | "Foundation" | Windows toolchain + workspace prep |
| v3.0.0-pal | "Abstraction" | Cross-platform traits in place |
| v3.0.0-daemon | "Chronicler" | Windows clipboard daemon running |
| v3.0.0-ipc | "Pipeline" | Named pipe IPC working |
| v3.0.0-cli | "Commander" | clipctl CLI working on Windows |
| v3.0.0-gui | "Window" | Tauri GUI running natively on Windows |
| v3.0.0-autostart | "Sentinel" | Autostart working |
| v3.0.0-input | "Typist" | Paste into apps working |
| v3.0.0-pkg | "Packager" | NSIS installer ready |
| v3.0.0-update | "Upgrader" | Auto-update pipeline working |
| v3.0.0-ci | "Forge" | CI/CD building Windows releases |
| v3.0.0-assets | "Identity" | Icons, branding, manifests |
| v3.0.0-test | "Assurance" | Tested on Windows 10/11 |
| v3.0.0-store | "Storefront" | Published on winget + Store |
| v3.0.0-windows | "Windows" | Full Windows release |
