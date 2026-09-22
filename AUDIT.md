# LinVClipBoard — Full Audit & Overhaul Report

| | |
|---|---|
| **Audited revision** | `8ef21f0` (tag `v3.0.3`, branch `main`) |
| **Audit date** | 2026-09-21 |
| **Scope** | Every tracked file (142). All Rust crates, Tauri backend, React frontend, CSS, i18n, packaging, install scripts, CI, Windows port, publish manifests, repo governance. |
| **Method** | Full read of all source; `cargo fmt/clippy/test/build` run locally; Windows-target type-check; `npm ci`/`npm audit`/`vite build`; GitHub Actions and Releases queried via API; each headline finding re-verified against source before inclusion. |
| **Toolchain used** | rustc 1.95.0, cargo 1.95.0, node 24.13.1, npm 11.12.1 |

---

## 0. Executive summary

LinVClipBoard is a feature-rich clipboard manager (Rust daemon + Tauri v2 GUI) with a genuinely good architectural skeleton: a single daemon owning SQLite/FTS5 storage, a length-prefixed JSON IPC protocol shared by CLI and GUI, a hardened systemd user unit, and a CSP-locked webview. The product idea and the breadth of features are well above hobby level.

The engineering around it is not. The project currently **cannot be built by its own CI**, **cannot be built for Windows at all**, ships a **`.deb` with no dependency declarations**, has **no LICENSE file**, has a **non-functional auto-updater on Windows** and a **root-privilege self-updater on Linux that installs unverified packages**, and distributes only through a manually uploaded single-architecture `.deb`. There is no one-line installer, no package repository, and every third-party manifest in the tree is a stale placeholder.

### Verified facts (not opinions)

| Check | Result |
|---|---|
| GitHub Actions on `main` | **Failed on every run** from 2026-03-12 through 2026-08-20 (8 of 8 inspected). Latest run: Lint ✗ (`cargo fmt`), Test ✗, Build UI ✗, Build Windows ✗ (`npm ci` lockfile drift). |
| `v3.0.3` release | Tag exists; **no GitHub Release** was ever published. Latest release is `v3.0.2` with a single `linvclipboard_3.0.2-1_amd64.deb`. |
| `cargo fmt --all --check` | **Fails** (`crates/linvclip-ui/src-tauri/src/lib.rs:980`). |
| `cargo clippy --all-targets -D warnings` (daemon crates) | **Fails** (`crates/shared/src/db.rs:796`, `len_zero`). |
| `cargo check --target x86_64-pc-windows-msvc -p platform` | **8 type errors**. The Windows platform layer has never compiled. |
| `cargo test` (shared/clipd/clipctl/platform) | 24 tests pass. `clipd`, `clipctl`, `platform` have **zero tests**. |
| `npm ci` | **Fails**: `package-lock.json` out of sync with `package.json` (`@tauri-apps/plugin-updater` missing, `@tauri-apps/api` version mismatch). |
| `npm audit` | 7 vulnerabilities (4 high, 2 moderate, 1 low) in build toolchain. |
| Frontend bundle | Main chunk 522 KB minified (154 KB gzip); emoji + symbol JSON (320 KB raw) inlined into the startup chunk. |
| `LICENSE` file | **Missing**. README badge, PKGBUILD, RPM spec, winget and Chocolatey manifests all reference it. GitHub reports `licenseInfo: null`. |
| Release binary size | `clipd` 11.2 MB, `clipctl` 2.6 MB — no `[profile.release]` (no LTO, no strip). |
| Repo stars / downloads | 2 stars; 13 downloads of latest release. |

### Verdict in one line

**Not production-grade and not currently shippable.** The Linux `.deb` path works for the author's own machine; nothing else does. See §12 for the scorecard and §11 for the phased plan that would change that.

---

## 1. Project map

```
LinVClipBoard/
├── crates/
│   ├── shared/            Library: SQLite+FTS5 DB, IPC framing, config, models, migrations   (838-line db.rs)
│   ├── clipd/             Daemon: clipboard monitor (poll), IPC server, D-Bus, config watcher (+ Windows forks)
│   ├── clipctl/           CLI client (clap) — list/search/paste/pin/delete/clear/status/completions/manpage
│   ├── platform/          "Platform Abstraction Layer" — traits + unix/ + windows/ impls  (added May 2026)
│   └── linvclip-ui/       Tauri v2 GUI
│       ├── src-tauri/     Rust backend: 1,869-line lib.rs, 50+ #[tauri::command]s, tray, updater, GIF, OCR, QR…
│       └── src/           React 19 + Vite 5 frontend: App.jsx (719 lines), 22 components, 4,502-line styles.css
├── install/               systemd units, .desktop files, dev install.sh, weekly update-check timer
├── packaging/             build-deb.sh (418 lines, used by CI), build-appimage.sh, PKGBUILD, RPM spec, postinst/prerm
├── windows/               15 planning docs, publish manifests (winget/scoop/choco), dev build/install .ps1, unused .rc/.manifest
├── upgrade-plan/          Historical planning doc (tracked but gitignored)
└── .github/workflows/ci.yml   Single 262-line workflow: lint, test, build-daemon, build-ui, build-windows, release
```

**Runtime topology:** `clipd` polls the clipboard every 250 ms via `arboard`, dedups by SHA-256, stores text in SQLite and images as PNG blobs, and serves a 4-byte-length-prefixed JSON protocol over `$XDG_RUNTIME_DIR/linvclip.sock` (Unix) or `\\.\pipe\LinVClipBoard` (Windows). `clipctl` and the Tauri GUI are both clients. The GUI additionally polls the daemon every 2 s (status) and 5 s (items), rebuilds the tray menu every 5 s, and talks directly to GitHub (updates) and KLIPY (GIFs).

---

## 2. Consolidated critical & high findings (all areas)

Ranked by impact. Each is verified against source; file:line references are to this revision.

### CRITICAL

| # | Finding | Evidence | Fix |
|---|---|---|---|
| **C1** | **Windows port has never compiled.** 8 type errors in `crates/platform/src/windows/` plus a use-after-move in `crates/clipd/src/monitor_windows.rs:35,42`. Every Windows binary depends on `platform`. | `cargo check --target x86_64-pc-windows-msvc -p platform` → E0308 ×3 (`clipboard.rs:23,40,61`, `u32` vs `usize`), E0599 (`clipboard.rs:65` `map_err` on `Option`), E0308 (`clipboard.rs:68` `Vec<u8>` vs `String`), E0599 (`input.rs:26` trait not imported), E0599 (`input.rs:54` `Key::Layout` removed in enigo 0.3), E0277 (`monitor.rs:31` `HWND` not `Send`). `monitor_windows.rs:35` moves `checksum`, `:42` borrows it. | Fix the 9 errors; add `cargo clippy --target x86_64-pc-windows-msvc` to the **Linux** lint job so Windows code is type-checked on every PR. |
| **C2** | **CI has been red for 6 months; releases are hand-uploaded.** `cargo fmt` diff at `lib.rs:980`; `package-lock.json` drift breaks `npm ci` in `build-ui` and `build-windows`; `v3.0.3` produced no release. | GitHub Actions API: 8/8 runs `failure`. Local `npm ci`: "Missing: @tauri-apps/plugin-updater@2.12.0 from lock file". | `cargo fmt --all`; `npm install` and commit lockfile; add branch protection requiring green CI; never tag from a red `main`. |
| **C3** | **Linux self-updater runs `pkexec bash` on a generated script with unsanitised interpolation, installing an unverified `.deb` as root.** | `crates/linvclip-ui/src-tauri/src/lib.rs:697` `format!("linvclipboard_{}_x86_64.{}", version, ext)` (path traversal via `version`); `:869,886,912` `pkexec bash /tmp/linvclip-update-<pid>.sh` containing `dpkg -i --force-overwrite "{path}"`; `:781-903` scripts and logs in world-writable `/tmp` with predictable names; `:654-666` picks the **first** `.deb` asset regardless of architecture; no checksum or signature check anywhere. | Verify a signed `SHA256SUMS` (minisign/cosign or GitHub attestation) before install; restrict hosts to `github.com`/`objects.githubusercontent.com`; validate `version` against `^\d+\.\d+\.\d+$`; replace script generation with a fixed, packaged helper + polkit policy; long-term, replace with an apt/dnf repo. |
| **C4** | **Windows auto-updater (the v3.0.3 headline feature) cannot work: placeholder public key.** | `crates/linvclip-ui/src-tauri/tauri.conf.json:75` `"pubkey": "PLEASE_REPLACE_WITH_GENERATED_PUBKEY"`. `lib.rs:968` swallows the plugin error and silently falls back to a GitHub API check. | `npx tauri signer generate`; commit the public key; store private key as a secret; make the release job **fail** (not skip) if the `.sig` is absent on a tag build. |
| **C5** | **`.deb` declares no `Depends:`.** Installs on a minimal system, then the GUI dies with missing `libwebkit2gtk-4.1`. `Maintainer:` has no e-mail. | `packaging/build-deb.sh:96-117` has only `Recommends:`/`Suggests:`. | Use `dpkg-shlibdeps` or `cargo-deb`; run `lintian` in CI. |
| **C6** | **No `LICENSE` file.** Legal blocker for AUR, Flathub, winget, Chocolatey; breaks PKGBUILD and RPM spec (`install -Dm644 LICENSE`). | `git ls-files` has no LICENSE; `README.md:14`, `packaging/PKGBUILD:54`, `packaging/linvclipboard.spec:68` reference it. | Add MIT `LICENSE` with copyright holder. |
| **C7** | **Windows bundle references a gitignored, never-committed NSIS hook and lacks an `.ico`.** Clean clone cannot bundle. | `tauri.conf.json:45` → `windows/hooks/installer.nsh` (directory does not exist); `.gitignore:44` `*.nsh`. `icons/` has only `32x32.png` and `icon.png`. | Un-ignore and commit the hook; `npx tauri icon <1024px.png>`. |
| **C8** | **Wrong GitHub owner and placeholder hashes in every Windows publish manifest.** | `windows/publish/chocolatey/*`, `winget/*` use `github.com/akash-singh/…` (canonical is `DreamerX00`); `SHA256_HASH`, `<REPLACE_WITH_ACTUAL_SHA256>`, `ProductCode: "{...GUID...}"`; all pinned to 3.0.0. | Generate manifests in the release job from the real artifact (`komac`, `wingetcreate`, `choco pack`). |

### HIGH

| # | Area | Finding | Evidence |
|---|---|---|---|
| **H1** | clipd | **Blacklisted-app protection is bypassed on focus change.** Capture is skipped only *while* the password manager is focused; the checksum is not updated, so the password is captured on the first poll after Alt-Tab. | `crates/clipd/src/monitor.rs:184-199` `continue` without touching `last_text_checksum`. |
| **H2** | clipd | **Blacklist is inert on GNOME/KDE Wayland and forks 3 processes per poll.** Only `xdotool`, `swaymsg`, `hyprctl` are tried, synchronously, 4×/s. | `monitor.rs:20-63`. |
| **H3** | clipd | **Remote-triggerable panic kills the monitor task permanently.** `urlish_decode` slices a `&str` by byte index; `%` followed by ASCII then a multibyte char panics. Task is `tokio::spawn`ed and its handle discarded. | `monitor.rs:418-419` `&s[i + 1..i + 3]`; `main.rs:127` `_monitor_handle`. |
| **H4** | clipd | **IPC server `expect()`s on `Clipboard::new()` at boot** — the exact display-not-ready failure that v3.0.1 fixed for the monitor. Daemon stays alive with no socket; systemd never restarts it. | `crates/clipd/src/server.rs:36-38`. |
| **H5** | clipd/shared | **One large copy breaks the UI permanently.** Text/HTML capture never checks `max_item_size_bytes`; `List` returns full `content` per row; receiver caps frames at 64 MB. | `monitor.rs:433-543` (no size check), `db.rs:159-164` (`SELECT … content …`), `shared/src/ipc.rs:10,37-42`. |
| **H6** | clipctl | **`clipctl list` panics on non-ASCII previews and short IDs.** Byte slicing of `preview_text[..57]` and `id[..8]`. | `crates/clipctl/src/main.rs:185,188-189,225-226`. |
| **H7** | shared | **Config parse failure fails open** (security settings reset to defaults) and the warning is logged before the subscriber exists. | `config.rs:237-246`; `main.rs:21` load before `:23-30` init. |
| **H8** | clipd | **Image polling transfers and SHA-256-hashes the full RGBA buffer 4×/s** while any image is on the clipboard (~130 MB/s for a 4K screenshot). | `monitor.rs:551-561,588`. |
| **H9** | platform | **Windows IPC framing uses `read()` not `read_exact()`** — any response over the pipe buffer (~64 KB) deserialises garbage. Unix impl silently does `read_exact`, so the trait has undefined semantics. | `crates/platform/src/ipc.rs:27-37`; `windows/ipc.rs:59-64,87-92` vs `unix/ipc.rs:50-52`. |
| **H10** | platform | **Named pipe: no `first_pipe_instance`, no explicit DACL, global name** → pipe squatting, duplicate daemons, cross-session collisions. | `windows/ipc.rs:9,38-46,128-130`. |
| **H11** | platform | **Accept loop can hot-spin at 100% CPU** if re-creating the next pipe instance fails; also a listen gap between `connect()` and `create()`. | `windows/ipc.rs:116-137`; `clipd/src/server_windows.rs:64-66` (no backoff, no break). |
| **H12** | tauri | **Windows build of the GUI crate cannot compile:** `tracing::` macros used with no `tracing` dependency. | `lib.rs:1708,1711,1716`; `src-tauri/Cargo.toml` has no `tracing`. |
| **H13** | frontend | **Keyboard actions act on the unfiltered list while the UI highlights the filtered list.** With a filter pill active, Enter pastes and **Delete deletes a different item than the one shown selected.** | `App.jsx:440-441,474-481,505` use `itemsRef.current`; `:603` renders `filteredItems`. |
| **H14** | frontend | **Vim `dd`/`gg` fire while typing in any non-search input** (snippet editor, tag input), deleting the selected clipboard item mid-word. | `KeybindingContext.jsx:190-204` runs before the `isInput` guard at `:214`. |
| **H15** | frontend | **GIF tab refetch loop** hitting `api.klipy.com` every ~300 ms + RTT for as long as a search is open. | `GifPicker.jsx:52-80,95-99` — `loading` in `useCallback` deps re-triggers the effect. |
| **H16** | frontend | **Debounced search silently dropped** when a background poll is in flight; no request sequencing, so stale results can overwrite newer ones. | `App.jsx:121-122,208-222,231-236`. |
| **H17** | packaging | **Weekly update-check launches a second GUI instance and hangs the timer.** `linvclip-ui --version` is not implemented; `main.rs` has no argv handling; `grep` waits for EOF forever; `Type=oneshot` has no timeout. | `install/linvclip-update-check.sh:9`; `install/linvclip-update-check.service:6-8` also hardcodes `DISPLAY=:0`. |
| **H18** | packaging | **preinst rewrites dpkg's own database and runs nested `dpkg`; maintainer scripts `rm -f` files inside every user's `$HOME` as root; postinst `pkill`s all users' UIs and `su`-launches a GUI with no `DISPLAY`.** All Debian Policy violations. | `build-deb.sh:142-160`, `:176-191,294-304`, `:320-322`. |
| **H19** | packaging | **Release job rebuilds from scratch instead of publishing tested artifacts; aarch64, `.rpm`, AppImage, checksums, SBOM, attestations all missing.** `build-daemon` builds aarch64 and uploads nothing. | `ci.yml:62-77,194-262`. |
| **H20** | repo | **`Cargo.lock` is gitignored for a binary project** → non-reproducible releases, degraded cache, nothing for `cargo-audit` to scan. | `.gitignore:4`. |
| **H21** | repo | **No supply-chain checks:** no `cargo-audit`/`cargo-deny`, no `npm audit`, no Dependabot, actions pinned by mutable tag. `npm audit` currently reports 4 high. | `ci.yml` (entire); no `.github/dependabot.yml`. |
| **H22** | packaging | **RPM spec and PKGBUILD are unbuildable and stale** (`1.3.0`; `sha256sums=('SKIP')`; missing `LICENSE`; wrong `%setup` dir; network in `%build`). | `packaging/linvclipboard.spec:4,40,68,77`; `packaging/PKGBUILD:3,30,54`. |
| **H23** | systemd | **Sandbox paths don't match the socket the daemon binds.** `ReadWritePaths=%t/linvclip` but socket is `%t/linvclip.sock`; `PrivateTmp=true` hides the `/tmp` fallback socket from clients. `MemoryMax=50M` vs `max_item_size_bytes=50MB` → one big image OOM-kills the daemon. | `install/clipd.service:29-35`; `crates/shared/src/config.rs:221-223`. |

---

## 3. Component deep-dives

### 3.1 `shared` crate (DB, IPC, config, models)

**Done well:** WAL mode; r2d2 pool; `thiserror` error enum; parametrised SQL everywhere (no injection found); race-safe dedup via `UPDATE`-then-`INSERT OR IGNORE` on a `UNIQUE(checksum)` index; versioned idempotent migrations with a test; length-prefixed framing with a size cap and `InvalidData` instead of panics.

**Medium findings**

| Ref | Location | Issue | Fix |
|---|---|---|---|
| S-M1 | `db.rs:183-190` | FTS5 "escaping" strips `-`, `+`, `:`, `(`, `)`, `*` etc. inside a phrase query, corrupting it: `foo-bar` → `foobar` (no match), `C++` → `C`. Only `"` is special inside an FTS5 phrase. FTS indexes `preview_text` (first 200 chars) only, so deep-text search is impossible. | `query.replace('"', "\"\"")` and nothing else; index `content` (capped) for text types; consider `unicode61 remove_diacritics 2`. |
| S-M2 | `db.rs:229-250` | `search_regex` loads **every row including full content** into RAM, runs a `COUNT(*)` it discards, and returns `total = page size`, so pagination stops after page 1. | Stream with `query_map`, filter lazily, count before `skip/take`. |
| S-M3 | `models.rs:49-62`, `db.rs:573-594` | Tags are write-only: `ClipboardItem` has no `tags` field; `row_to_item` never reads the column. `AddTag` responses are identical before and after. | Add `tags: Vec<String>`; select and parse the column. |
| S-M4 | `db.rs:416-433`, `:284-291`, `:269` | N+1 queries in `enforce_limits` (2N statements, N WAL commits), `bulk_delete`, `delete`. | Single `DELETE … RETURNING` inside one transaction. |
| S-M5 | `db.rs:38-51` | `synchronous`, `cache_size`, `temp_store` PRAGMAs applied to **one** pooled connection; the other 7 run at defaults. | `SqliteConnectionManager::with_init(...)`. |
| S-M6 | `db.rs:55-56,72-76` | FTS external-content table keyed on implicit `rowid`; `VACUUM` may renumber rows of a table without `INTEGER PRIMARY KEY` → corrupted FTS index. Dedup bump fires the FTS update trigger for unchanged text. | Add `rowid INTEGER PRIMARY KEY`; `UPDATE OF preview_text` trigger; expose `rebuild`. |
| S-M7 | `config.rs:16-53` | No validation (`poll_interval_ms = 0` → busy loop forking 3 processes per iteration); `SaveConfig` writes non-atomically; `GetConfig` returns stale in-memory config after save; `sensitive_expiry_minutes`, `clear_after_paste`, `redact_sensitive`, `auto_ocr`, `smart_paste` are **accepted but never read by the daemon**. | Validate on load; temp+rename; `ArcSwap` hot reload; implement or remove the dead knobs. |
| S-M8 | `config.rs:215-225`, `server.rs:23-31` | Socket lifecycle: second `clipd` unlinks the live socket; `/tmp/linvclip-<uid>.sock` fallback in world-writable dir allows another local user to squat; chmod-after-bind window. | `flock` lockfile; bind inside a fresh 0700 dir; `symlink_metadata` for stale check. |
| S-M9 | `shared/src/ipc.rs:33-38`, `server.rs:53-59` | No per-request timeout; semaphore permit acquired **after** accept; `vec![0u8; len]` allocates up to 64 MB on a 4-byte header. 10 stalled clients wedge the daemon. | `tokio::time::timeout(5s)`; acquire before accept; chunked read. |

**Low:** every DB error mapped to `NotFound` (`db.rs:262-263`); `unwrap_or_default()` on every column hides corruption; timestamps stored as RFC3339 text and compared lexically with variable fractional digits; `&item.checksum[..8]` (`db.rs:113,139`) panics on short checksums; `list()` has no upper bound on `limit`; `search_snippets` doesn't escape `LIKE` wildcards; protocol has no version field; `PathBuf::from("~/.config")` never tilde-expanded (`config.rs:173,191`); `libc` pulled in solely for `getuid()`.

**Dependency hygiene:** `serde_json` in both `[dependencies]` and `[dev-dependencies]`; `tokio` `sync` and `rusqlite` `uuid` features unused; no `[workspace.dependencies]` — `serde`, `tokio`, `sha2`, `hex`, `chrono`, `dirs`, `toml`, `uuid`, `arboard`, `image`, `thiserror`, `tracing` are re-declared in 2–4 crates with independent specs.

### 3.2 `clipd` daemon

**Done well:** boot-time `Clipboard::new()` retry with capped backoff and cancellation (`monitor.rs:135-167`); `CancellationToken` threaded through tasks; socket `0700`; connection semaphore; orphan-blob GC; adaptive idle slowdown; rich-content capture (HTML, `text/uri-list`, GNOME copied-files) on Wayland.

**Beyond the High items above (H1–H5, H8):**

| Ref | Location | Issue |
|---|---|---|
| D-M1 | `monitor.rs`, `server.rs` (throughout) | Blocking work on tokio workers: `Command::output()` ×3, `clipboard.get_*()`, `wl_get_contents` + unbounded `read_to_string`, synchronous PNG encode (`:592`), all SQLite calls, `image::open` while holding the clipboard mutex (`server.rs:235-243`), r2d2 `pool.get()` blocking up to 30 s. |
| D-M2 | `main.rs:68-73,127,137` | Shutdown "grace" is `timeout(3s, sleep(100ms))` — always 100 ms; task handles discarded; in-flight DB write or PNG save can be cut mid-way. `monitor.rs:270` sleeps up to 60 s without `select!` on cancel. |
| D-M3 | `monitor.rs:579-599`, `db.rs:363-390` | Blob bookkeeping by **absolute path string**; changing `XDG_DATA_HOME` or a `$HOME` symlink makes every blob "orphan" and deletes it within ~100 polls. |
| D-M4 | `server.rs:202-207` | Files paste emits `file://` URIs without percent-encoding; space and `#` break Nautilus/Dolphin. Capture path *decodes*, so the round-trip is asymmetric. |
| D-M5 | `server.rs:182-196` | HTML paste fallback pastes raw markup as text; `plain` is computed but unused; `paste_html_wayland` spawns its own Wayland connection per call while `arboard::set_html(html, Some(alt))` already does both MIME types. |
| D-M6 | `monitor_windows.rs:11,22,76` | Comment and log say "event-driven"; code polls at hardcoded 300 ms ignoring `poll_interval_ms`; no blacklist, no images; backoff gets stuck after >10 failures then `Ok(None)`; checksum committed **before** `db.insert` so a DB error drops the item permanently. |
| D-M7 | `server.rs:95-417` vs `server_windows.rs:79-313` | **~300 lines duplicated** — 22 identical match arms with identical error strings. Only `Paste`/`UseSnippet` differ. |
| D-L | `is_blacklisted("")` matches everything (`monitor.rs:108-111`); raw RGBA written as `.png` when `from_raw` fails (`:591-596`); "Daemon ready" logged before the socket is bound (`main.rs:156-159`); bind failure exits 0; `dbus_service.rs` advertises `Paste(id)` it doesn't implement; `arboard` declared twice, `serde`/`chrono`/`dirs` unused in `clipd/Cargo.toml`; `u64::is_multiple_of` requires Rust ≥1.87 but no `rust-version` is declared. |

### 3.3 `clipctl` CLI

Beyond H6 (panics): prints 8-char IDs (`main.rs:185`) but every mutating verb requires the full UUID (`db.rs:257-263 WHERE id = ?1`) → `clipctl paste <shown-id>` always fails. Any I/O error, including protocol errors from a running daemon, prints "Start it with: clipd". Missing `get`, `bulk-*`, snippet, regex, config subcommands the protocol supports. `rt-multi-thread` for a one-shot CLI. Zero tests.

### 3.4 `platform` crate + Windows port

**Verdict: scaffolding, not a port.** Beyond C1, H9–H11:

- **Unix half is 100% dead code** (339 lines, zero consumers). Linux still uses `shared::ipc`, `clipd/monitor.rs`, `server.rs`, and the Tauri lib's own `wtype`/`xdotool` tables. The PAL added a third copy beside the two that already existed.
- **Windows "event-driven" monitor is dead code** (`windows/monitor.rs`); it's also thread-affine (`GetMessage` only delivers to the creating thread) and un-shutdownable through the trait.
- `get_image`/`get_files` return `Ok(None)` (indistinguishable from "empty") instead of `Unsupported` (`windows/clipboard.rs:47-51,76-78`).
- Nested clipboard open: `get_clipboard` internally opens/closes, so the outer guard provides no exclusivity and its own `CloseClipboard` fails (`clipboard.rs:22-28,39-44`).
- CF_HTML fragment slicing can panic on non-char-boundary offsets (`clipboard.rs:89-106`).
- Autostart `Run` value unquoted (`windows/service.rs:29`, `install.ps1:53`) — classic unquoted-path hazard; GUI and dev script write different paths to the same key.
- `build.ps1:5,18,22`: `[switch]$Release = $true` passes a literal `""` to cargo on PowerShell 7.3+; cwd leaks on `throw`; announces the wrong bundle output path.
- `windows/resources/app.manifest` and `app.rc` are unused by any build script and stale at `3.0.0.0` / `LinVClipBoard.exe` (no such binary).
- `windows/DEP-COMPAT.md` claims are wrong: arboard has no `clipboard-win` feature; `SetConsoleCtrlHandler` is promised but only `ctrl_c()` is used; `dbus_service.rs` is *not* gated.
- Plan vs reality (`windows/plan/main.md`): "zero-polling event-driven" → 300 ms poll; "HTML/files/images" → text only; "enigo + SendInput + UIA" → enigo only; "Authenticode signing" → none; "x86_64, ARM64" → x86_64 only; "auto-update" → placeholder key.

### 3.5 Tauri backend (`src-tauri/src/lib.rs`, config)

**Done well:** all daemon calls via `shared::ipc::send_request` (no duplicated client); argv arrays (never a shell) for `xdotool`/`wtype`/`tesseract`; CSP `script-src 'self'` with no `unsafe-inline`/`unsafe-eval`; minimal capability set; HTML-escaping in `highlight_code`; UpdateModal deliberately avoids raw HTML.

Beyond C3, C4, H12:

| Ref | Location | Issue | Fix |
|---|---|---|---|
| T-M1 | `lib.rs:1449-1527` | `fetch_link_preview` is auto-invoked for every URL item shown (`LinkCard.jsx:17`): SSRF/tracking vector to any host incl. localhost/RFC1918; "1 MB cap" comment is false (`resp.text()` reads whole body first); `body[..1_048_576]` **panics** if not a char boundary. | Opt-in; allow only `http(s)`; reject private ranges; stream with byte budget; `floor_char_boundary`. |
| T-M2 | `lib.rs:491-499,504-546,762-767` | `get_image_base64(path)`, `extract_text_from_image(path, lang)`, `install_update(path)` accept arbitrary paths from the webview. | Canonicalise and require prefix under the blob dir; validate `lang`. |
| T-M3 | `src-tauri/Cargo.toml:14` | `custom-protocol` hardcoded in default features → `tauri dev` loads `frontendDist`, not `devUrl`; no HMR. | Move behind `[features]` as the template does. |
| T-M4 | `tauri.conf.json:31` | CSP blocks link-preview images (`img-src` lacks external hosts); `asset:` entries are dead (protocol not enabled); missing `object-src 'none'`, `base-uri 'self'`, `frame-src 'none'`; `connect-src` lists hosts JS never contacts. | Tighten and correct. |
| T-M5 | `lib.rs:1390-1436` | `highlight_code`/`detect_language` are **sync** (main thread) and call `SyntaxSet::load_defaults_newlines()` + `ThemeSet::load_defaults()` **on every call** → UI jank on every arrow-key press over a code item. `type_text` blocks on `std::process::Command::status()` inside async. | `static LazyLock`; make async; `tokio::process`/`spawn_blocking`. |
| T-M6 | (no `tauri-plugin-single-instance`) | No single-instance guard; autostart + system shortcut + restart script all launch the binary → duplicate tray icons (the v3.0.2 fix only removed one autostart file). | Add the plugin; focus existing window in callback. |
| T-M7 | `lib.rs:1820-1844` | Global shortcut registered once at startup; changing `ui.shortcut` in settings does nothing until UI restart (daemon says "restart clipd", which is the wrong process); failure only `eprintln!`s; plugin has no Wayland backend so it silently fails on GNOME/KDE Wayland. | `set_shortcut` command; emit error event; document Wayland path. |
| T-M8 | `build.rs:5-14`, `ci.yml` | KLIPY key read from gitignored `klipy.key`; CI never provides it → **every CI-built artifact ships without GIF support**. XOR "obfuscation" emits the pad into the same file. No `rerun-if-changed`. | `KLIPY_API_KEY` env + `rerun-if-env-changed`; GitHub secret. |
| T-M9 | `tauri.conf.json:24-27` | Window is `visible: true` + `alwaysOnTop` + `skipTaskbar` at launch → autostart pops an always-on-top frameless window at every login. | `visible: false`. |
| T-L | GIF cache functions have no writer (dead feature, `lib.rs:1233-1308`); 25 near-identical `match send_request` blocks; `get_config` silently falls back to disk on daemon error; tray menu rebuilt every 5 s even when unchanged; `std::process::exit(0)` inside a command; version compare drops pre-release tags; no `reqwest` timeouts anywhere; three `get_webview_window("main").unwrap()` in setup; `crate-type = ["lib","cdylib","staticlib"]` mobile leftovers; `gen/schemas/*` committed; `index.html` links nonexistent `/clipboard.svg`. |

**Proposed module split** (from 1 file → ~15): `error.rs`, `daemon.rs` (typed `daemon_call<T>` wrapper), `dto.rs`, `commands/{items,snippets,config,clipboard,media,gif,transform,highlight,link_preview}.rs`, `updater/{mod,linux,windows}.rs`, `tray.rs`, `window.rs`, `shortcut.rs`, `background.rs`. Move the two bash scripts to `src-tauri/scripts/*.sh` and `include_str!` them so they can be shellchecked.

### 3.6 React frontend

**Done well:** push-based `clipboard-updated` events with polling fallback; consistent `cancelled` flag in async effects; stable `key`s by id; API key never in the bundle; `react-markdown` default URL sanitisation; `ConfirmDialog` manages focus and Escape; `PreviewPane` correctly code-split; `StrictMode` on; `prefers-reduced-motion` and `:focus-visible` handled.

Beyond H13–H16:

| Ref | Location | Issue |
|---|---|---|
| F-M1 | `App.jsx:205-227`, `UpdateModal.jsx:25-33` | `listen()` unlisten race — cleanup before the promise resolves leaks a second listener (guaranteed under StrictMode). |
| F-M2 | `App.jsx:429`; `SnippetVarDialog`, `SnippetEditor`, `QrModal` | Escape in snippet dialogs hides the **whole window**; Escape does nothing in QR modal. |
| F-M3 | `ClipboardList.jsx:270-284`; `smartDetect.js` | `prettify`, `open_file`, `date` chips render but do nothing; `window.open()` and `<a target=_blank>` are no-ops in a Tauri webview (should use plugin `open()`). |
| F-M4 | `App.jsx:41,202,211,221,238-245` | `get_status` polled every 5 s **and never rendered** (StatusBar was removed; CSS and i18n remain). |
| F-M5 | `App.jsx:130-133` | Background refresh re-fetches **all loaded items** every 5 s (500 rows + previews after scrolling). |
| F-M6 | theme/accent/language/zoom/window-size | Dual/triple sources of truth: `localStorage` + `config.ui.*` + component state; 3 copies of accent-application code, 4 copies of setSize logic. |
| F-S1 | `ClipboardList.jsx:319,391`; `PreviewPane.jsx` | **Redaction bypass**: with "Redact sensitive" on, `aria-label`, chip `title`, and the entire PreviewPane show unredacted text. |
| F-S2 | `PreviewPane.jsx:209` | Markdown `<a>` has no handler → clicking navigates the Tauri window off-site. |
| F-P1 | bundle | 522 KB main chunk; `emojis.json` (153 KB) + `symbols.json` (167 KB) inlined; `EmojiPicker`, `SymbolPicker`, `GifPicker`, `SnippetPicker`, `ColorPicker`, `SettingsPanel`, `UpdateModal` all eagerly imported. Only `PreviewPane` is `lazy()`. For a tray overlay that must appear instantly, this is the single biggest startup win. |
| F-P2 | `EmojiPicker.jsx:121-140`, `SymbolPicker.jsx:124-146` | ~4,000 buttons rendered at once on tab switch; no virtualisation. |
| F-P3 | `ClipboardList.jsx:125-149` | `ClipItem` not memoised; 19 props incl. 4 helper fns recreated per render; `JSON.parse(item.tags)` per render; no virtualisation for a list whose cap is 10,000. |
| F-P4 | `ClipboardList.jsx:7-20`, `lib.rs:491-499` | Thumbnails fetch the **full PNG base64** per visible row per mount (a 5 MB screenshot = 6.7 MB string over IPC). |
| F-A11y | `App.jsx:573` `role="application"` on root (disables screen-reader browse mode); context-menu submenus have no `aria-haspopup`/keyboard nav; snippet rows are `<div onClick>` (mouse-only); `aria-controls` point at nonexistent IDs; several modals lack `role="dialog"`/focus trap. |
| F-Tooling | **No ESLint, no Prettier, no TypeScript, no tests, no test runner** (`@testing-library/*` installed with nothing to run them). `@tauri-apps/plugin-global-shortcut` and `plugin-updater` never imported in `src/` (dead deps). `react-hooks/exhaustive-deps` alone would have caught H15 and F-M1. |

**CSS (`styles.css`, 4,502 lines):** semantic CSS-variable theming with 8 themes is good. But **10 undefined custom properties** are referenced (`--border-color`, `--text-muted`, `--surface`, `--font-mono`, …, at ~23 sites) → borders degrade to `currentColor`, monospace blocks render in Inter. `.color-swatch` is defined twice with different sizes (`:3637` vs `:4220`) — the ColorPicker's main swatch renders 10×10. ~300 lines of dead rules (removed StatusBar, placeholder panels, old advanced collapsible). 7 duplicated selectors from append-only growth. 164 hardcoded hex colours outside theme blocks. `--zoom-factor` is set but referenced 0 times; 76 `px` font-sizes → zoom is partial.

**i18n:** key parity is near-perfect (1 missing key per locale, 0 extras) but **translation coverage is not**: in all three non-English locales the entire Features tab, Keyboard tab, all `ocr.*`, `keybind.*`, `security.*` strings are untranslated English (57–65 identical values). ~41 `en.json` keys are dead (removed StatusBar, `smart.*` labels hardcoded in `smartDetect.js`, GIF API-key UI that no longer exists). ~60 hardcoded English strings in JSX. Homegrown provider has no interpolation, no plurals, no dev-mode missing-key warning. Files named `hin.json`/`japanese.json` mapped to codes `hi`/`ja`.

### 3.7 Packaging, install, CI

Beyond C2, C5–C8, H17–H23:

- **Three divergent `.deb` pipelines**: `build-deb.sh` (used), `cargo-deb` metadata in `clipd`/`clipctl` Cargo.toml (never invoked, contradicts build-deb on autostart), and `tauri.conf.json` `bundle.linux.deb` (never bundled; references `install/deb-desktop.desktop` which is missing **and** gitignored, and `install/deb-postinst.sh` which is tracked **but** gitignored).
- `/etc/xdg/autostart/linvclipboard.desktop` not in `conffiles`; `Replaces:`/`Breaks:` reference the package itself; `gsettings set … ibus … emoji hotkey '[]'` silently rewrites a user's IBus setting from root via `su`.
- **Four sources, three different shortcuts**: README `Super+.`, `install.sh` `Super+V`, postinst `Ctrl+/`, code default `Ctrl+/`.
- AppImage script installs a systemd unit into AppDir and launches only the UI — the daemon never starts, so the AppImage has no backend.
- `install/install.sh` is a **build-from-source dev helper** (needs repo, Rust, Node, Tauri CLI), writes an inline unit that differs from `install/clipd.service`, and aborts under `set -e` on `systemctl status` exit 3.
- CI: identical 8-line apt block duplicated 4×; `cargo install cross --git` unpinned every run; no workflow-level `permissions:`; no `concurrency:`; clippy Linux-only (Windows code never linted); Node 20 (EOL 2026-04-30); `*.nsis.zip` glob is a Tauri v1 leftover; `build-windows-cross` duplicates `build-windows`.
- Repo hygiene: no `CHANGELOG`, `CONTRIBUTING`, `SECURITY.md`, `CODE_OF_CONDUCT`, issue/PR templates, `CODEOWNERS`, `.editorconfig`, `rustfmt.toml`, `deny.toml`; stray empty root `package-lock.json`; `.claude/settings.json` committed; 350 KB of generated `gen/schemas/*.json` committed; 15 planning docs + `DEP-COMPAT.md` + `upgrade-plan/` in the tree instead of `docs/` or a wiki; README says `rust-2024` badge (edition is 2021), "~300 emojis" (1,870), install example `1.5.0`, no Windows section despite claiming Windows support, no screenshots.
- Commit history: 50 commits / 27 tags in 6 months, one squashed commit per version (poor bisectability); mixed message conventions; `8348460` "update version to 4.0.2" typo; several tags not on the commit that announces the version.

---

## 4. Version drift table

Canonical: **3.0.3** (`Cargo.toml:12`, `package.json:4`, `tauri.conf.json:3` agree).

| File | Line | Value | Status |
|---|---|---|---|
| `crates/platform/Cargo.toml` | 3 | `0.1.0` | not workspace-inherited |
| `README.md` | 67 | `linvclipboard_1.5.0-1_amd64.deb` | stale |
| `packaging/PKGBUILD` | 3 | `1.3.0` | stale |
| `packaging/linvclipboard.spec` | 4, 77 | `1.3.0`, changelog `1.1.0-1` | stale, incoherent |
| `windows/publish/chocolatey/*` | 4–15 | `3.0.0`, `SHA256_HASH`, owner `akash-singh` | placeholder |
| `windows/publish/scoop.json` | 2, 8, 9 | `3.0.0`, `SHA256_HASH` | placeholder |
| `windows/publish/winget/*` | 3, 14, 15, 24 | `3.0.0`, wrong owner, `<REPLACE_WITH_ACTUAL_SHA256>`, `{...GUID...}` | placeholder |
| `windows/resources/app.manifest`, `app.rc` | 5; 7, 8, 21, 26 | `3.0.0.0` | dead files |
| `install/install.sh` | 53 | `Documentation=https://github.com/LinVClipBoard` | wrong URL |

**Fix:** adopt `release-plz` or `cargo-release`; sync `package.json`/`tauri.conf.json` from Cargo in a pre-commit step; generate every packaging manifest in the release job from `${GITHUB_REF_NAME#v}`; never hand-edit them.

---

## 5. Duplication map

| Copy A | Copy B | Lines |
|---|---|---|
| `shared/src/ipc.rs:9-49` framing | `platform/src/ipc.rs:6-40` framing (and they already disagree — H9) | ~40 |
| `clipd/src/server.rs:95-417` `handle_request` | `clipd/src/server_windows.rs:79-313` | ~300 |
| `clipd/src/server.rs:176-262` paste transforms | `server_windows.rs:320-356` `paste_impl` | ~40 |
| SHA-256-hex: `monitor_windows.rs:83-88` | `platform/unix/monitor.rs:26-31`, `monitor.rs:447,559`, `db.rs:752` | 4 copies |
| `platform/unix/input.rs:37-75` wtype/xdotool/ydotool | `lib.rs:190-215` (live copy) | ~40 |
| `platform/unix/ipc.rs:33-37` socket setup | `clipd/src/server.rs:23-31` | ~10 |
| Pipe name literal `platform/windows/ipc.rs:9` | `shared/src/config.rs:228` | — |
| Frontend accent-apply ×3, setSize ×4, theme-card map ×3, `listen` setup ×2 | `App.jsx`, `SettingsPanel.jsx` | ~150 |
| `.deb` pipelines ×3; Debian postinst ×2 | `build-deb.sh`, `cargo-deb` metadata, `tauri.conf.json`; `install/deb-postinst.sh` | ~400 |
| apt dependency block ×4 | `ci.yml:24-31,50-58,91-98,208-215` | 32 |
| 25 near-identical `match send_request` blocks | `lib.rs:69-560,1312-1336` | ~250 |

---

## 6. Test coverage

| Crate / area | Tests | Assessment |
|---|---|---|
| `shared` | 7 unit + 17 integration | IPC tests are **serde round-trips only** — no actual socket I/O, no size-cap, no truncation, no malformed JSON. Missing: FTS punctuation, regex pagination, `enforce_limits` boundaries, orphan-blob path mismatch, tag round-trip, config parse failure, `poll_interval_ms = 0`. |
| `clipd` | **0** | `urlish_decode` (H3), `is_blacklisted("")`, `swaymsg` tree parsing, URI preview, server handler against temp DB + fake clipboard, double-start, graceful shutdown. |
| `clipctl` | **0** | multibyte preview truncation (H6), short IDs, time/bytes formatting. |
| `platform` | **0** | framing round-trip with >64 KB payload and short-read mock (H9), pipe bind/connect + second-bind-fails (H10), CF_HTML fragment extraction with malformed offsets. |
| Tauri lib | **0** | version compare, `fetch_link_preview` host policy, path scoping. |
| Frontend | **0** (no runner) | `smartDetect` regexes, `KeybindingContext.resolveAction` incl. vim-in-input (H14), `filteredItems`/selection index (H13), i18n key parity script in CI. |
| Shell | none | `shellcheck` on `install/*.sh`, `packaging/*.sh`; `lintian` on `.deb`; `rpmlint` on spec; Docker smoke matrix for install. |

---

## 7. Optimisation opportunities (ranked by user-visible impact)

1. **Startup**: `lazy()` every picker/panel; move `emojis.json`/`symbols.json` behind `import()`; set `build.target` for WebKitGTK/WebView2. Expected: main chunk 522 KB → ~150 KB.
2. **Idle CPU**: stop 3× `Command::output()` per poll (H2); fingerprint images before full hash (H8); replace fixed polling with X11 `XFixesSelectionNotify` / Wayland data-control offer events where available; stop the GUI's 2 s status poll (result is never rendered) and 5 s tray rebuild when unchanged.
3. **Syntax highlighting**: `static LazyLock<SyntaxSet>` — turns every arrow-key press over code from ~100 ms of deserialisation into microseconds.
4. **IPC payload**: `List`/`Search` return summaries (no `content`), `Get` returns full content; thumbnails via backend-generated small PNG or `asset:` protocol instead of full base64.
5. **DB**: PRAGMAs on all pooled connections; `DELETE … RETURNING` in `enforce_limits`; index `content` for FTS; integer epoch timestamps.
6. **Binary size**: `[profile.release] lto = "thin", codegen-units = 1, strip = true, panic = "abort"` → expect `clipd` 11 MB → ~5 MB.
7. **List rendering**: `React.memo(ClipItem)`, `useMemo` helpers, virtualise (`@tanstack/react-virtual`) for the 10k cap; virtualise emoji/symbol grids.
8. **CI time**: artifact reuse instead of rebuild-in-release; apt caching; pinned `cross`/`cargo-xwin` via `taiki-e/install-action`; composite action for deps.

---

## 8. Better workarounds & feature setup

| Current approach | Problem | Recommended |
|---|---|---|
| Self-updater downloads `.deb` and runs `pkexec bash /tmp/*.sh` with `dpkg -i --force-overwrite` | Root code path with injection surface, no verification, arch-blind | apt/dnf repo (GitHub Pages + `reprepro`, or Cloudsmith OSS) so `apt upgrade` handles it; keep notify-only timer. Windows: real Tauri updater key. |
| Blacklist via `xdotool`/`swaymsg`/`hyprctl` shell-outs each poll | Inert on GNOME/KDE Wayland; 12 fork/exec per second | Detect compositor once; x11rb `_NET_ACTIVE_WINDOW`; `ext-foreign-toplevel` where offered; GNOME shell extension or D-Bus; log once when unavailable. Also poison checksums seen while blacklisted (H1). |
| Global shortcut via `tauri-plugin-global-shortcut` | No Wayland backend; silent failure | Ship `clipctl toggle` + D-Bus activation (`org.linvclipboard.App.Toggle`) and document binding it in GNOME/KDE settings; keep plugin for X11/Windows. |
| Autostart via `/etc/xdg/autostart` + user autostart + postinst `su` launch | Duplicate instances, root touching `$HOME` | `tauri-plugin-single-instance`; one autostart mechanism; postinst only `systemctl --user daemon-reload` via `systemd-run`/`loginctl` for active sessions. |
| Config hot-reload logs "restart required" | Security toggles (incognito, blacklist) need restart | `Arc<ArcSwap<AppConfig>>`; watch parent dir; apply `security.*` and `poll_interval_ms` live. |
| Tags stored as JSON string in a column, never read back | Feature is invisible | Proper `tags` + `item_tags` tables, or at minimum surface in `ClipboardItem`. |
| `redact_sensitive` / `clear_after_paste` / `sensitive_expiry_minutes` in config but unimplemented | Users believe secrets are purged | Implement in daemon (`Paste` handler, `enforce_limits` sweep, `capture_text`) or remove. |
| Homegrown i18n | No interpolation, plurals, missing-key warnings; 60 hardcoded strings | `i18next` + `react-i18next` (or keep homegrown but add `t(key, vars)`, dev warnings, and a CI parity check). |
| 15 planning docs + `DEP-COMPAT.md` + `upgrade-plan/` in tree | Noise; some tracked-but-ignored | `docs/` with `ARCHITECTURE.md`, `PROTOCOL.md`, `CONTRIBUTING.md`; move plans to GitHub Discussions/Projects or `docs/adr/`. |
| Emoji/symbol names English-only; ~4,000 buttons | Non-English search useless; render cost | Ship per-locale CLDR annotations lazily; virtualise. |

**Features worth adding once the base is sound** (in rough order of value): clipboard sync encryption at rest (SQLCipher or per-blob AES-GCM — the plan promised it, nothing implements it); `clipctl` parity with the protocol (`get`, snippets, regex, config); D-Bus `Paste(id)` (advertised, missing); export/import (JSON/CSV); per-item TTL; Flatpak with portal-based background; an `ext-data-control` Wayland monitor to drop polling on wlroots/KDE.

---

## 9. Industry-standard gap checklist

| Area | Expected | Present |
|---|---|---|
| License file | ✅ required | ❌ |
| Lockfiles committed & in sync | `Cargo.lock` + `package-lock.json` | ❌ Cargo.lock ignored; npm lock drifted |
| Green CI on `main`, required for merge | ✅ | ❌ red for 6 months, no branch protection |
| Reproducible release from CI | artifacts from tested jobs, checksums, attestation | ❌ manual upload, rebuild-in-release, no checksums |
| Supply chain | `cargo-deny`, `cargo-audit`, `npm audit`, Dependabot, SHA-pinned actions | ❌ none |
| Formatting/lint gates | rustfmt, clippy (all targets), eslint, prettier, stylelint, shellcheck | partial (rustfmt/clippy Linux-only; nothing for JS/CSS/shell) |
| Tests | unit + integration + e2e; coverage reporting | 24 tests in one crate; nothing else |
| Release profile | LTO, strip, codegen-units | ❌ |
| Versioning | single source, automated bump, CHANGELOG, SemVer | ❌ hand-edited in 15+ places, no CHANGELOG |
| Docs | README (accurate), CONTRIBUTING, SECURITY, ARCHITECTURE, man pages shipped | README only, partly stale; completions/manpages generated but never packaged |
| Governance | CoC, issue/PR templates, CODEOWNERS | ❌ |
| Packaging | proper `Depends`, lintian-clean, multi-arch, rpm/AppImage/Flatpak | ❌ amd64 `.deb` only, policy violations |
| Installation | one-liner + package repos | ❌ "download a URL you construct yourself, then `dpkg -i`" |
| Code signing | Authenticode (Windows), GPG-signed apt repo | ❌ |
| Secrets | API key via CI secret | ❌ CI builds ship without GIF key |

---

## 10. Single-link installation design (the user's explicit ask)

### Target UX

```bash
# Linux — one line, any distro
curl -fsSL https://get.linvclipboard.dev | bash          # or raw.githubusercontent.com/DreamerX00/LinVClipBoard/main/install.sh
```
```powershell
# Windows — one line
irm https://get.linvclipboard.dev/win | iex              # or raw.githubusercontent.com/.../install.ps1
```
Plus native channels: `sudo apt install linvclipboard` (own repo), `yay -S linvclipboard-bin`, `dnf copr enable … && dnf install`, `flatpak install flathub com.linvclipboard.app`, `winget install LinVClipBoard.LinVClipBoard`, `scoop install linvclipboard`, `choco install linvclipboard`.

### `install.sh` contract

1. `set -euo pipefail`; refuse to run as root; announce `sudo` use up front; flags `--version`, `--user` (no-sudo tarball into `~/.local`), `--uninstall`, `--dry-run`, `--yes`.
2. Detect `uname -m` → `amd64|arm64`; `/etc/os-release` → `apt|dnf|zypper|pacman|other`; check `systemctl --user` reachable.
3. Resolve tag via GitHub API (honour `GITHUB_TOKEN`); download `SHA256SUMS` + `SHA256SUMS.minisig` (or `gh attestation verify` if `gh` present) + matching asset into `mktemp -d`.
4. `sha256sum --ignore-missing -c SHA256SUMS`; abort on mismatch.
5. apt → `sudo apt-get install -y ./linvclipboard_${V}_${ARCH}.deb` (resolves `Depends`); dnf/zypper → `.rpm`; pacman → print AUR instruction or fall to `--user`; other → `--user` tarball (binaries → `~/.local/bin`, unit → `~/.config/systemd/user`, desktop+icon → `~/.local/share`).
6. `systemctl --user daemon-reload && systemctl --user enable --now clipd.service`; print the shortcut read from the real config default.
7. Idempotent (exit 0 if already at target version). `shellcheck`-clean. CI smoke matrix in Docker: `ubuntu:22.04`, `ubuntu:24.04`, `debian:12`, `fedora:40`, `archlinux`.

### `install.ps1` contract

1. `Set-StrictMode`; TLS 1.2; params `-Version`, `-Uninstall`, `-NoAutostart`.
2. If `winget` present and package published → `winget install --id LinVClipBoard.LinVClipBoard -e --silent`; exit.
3. Else resolve latest, download `LinVClipBoard_<v>_x64-setup.exe` + `SHA256SUMS`, `Get-FileHash` compare, `Get-AuthenticodeSignature` once signed, run `& $exe /S` (NSIS silent; `installMode: currentUser` → no UAC).
4. `-Uninstall` → NSIS `uninstall.exe /S`.

### CI redesign to make this possible

- Split `ci.yml` (PR gates) from `release.yml` (on `v*` tag).
- **ci.yml**: fmt; clippy Linux **and** `--target x86_64-pc-windows-msvc`; `cargo test --locked`; `cargo deny check`; `npm ci && npm run lint && npm test && npm audit --audit-level=high`; `shellcheck`; `lintian` on a built `.deb`; `rpmlint` on spec; i18n parity script. Workflow-level `permissions: contents: read`; `concurrency` with cancel-in-progress; SHA-pinned actions; composite action for apt deps.
- **release.yml** matrix: `linux-amd64` (deb, rpm, AppImage, tar.gz), `linux-arm64` (`ubuntu-24.04-arm` runner: deb, tar.gz), `windows-x64` (nsis + `.sig`). Each uploads artifacts. `publish` job downloads all, writes `SHA256SUMS`, signs with minisign, runs `actions/attest-build-provenance`, uploads `install.sh`/`install.ps1`, creates the release, then fans out: `KSXGitHub/github-actions-deploy-aur`, `komac`/`winget-releaser`, `choco push`, scoop bucket commit, apt repo publish (`reprepro` → GitHub Pages), COPR webhook.

### Distribution roadmap

| Phase | Channel | Blockers to clear first |
|---|---|---|
| 0 | Repo prerequisites | LICENSE, Cargo.lock, release profile, real `Depends`, version single-sourcing, CHANGELOG, `SHA256SUMS` + attestation, green CI |
| 1 | `install.sh` / `install.ps1` | Phase 0 |
| 2a | apt repo (GitHub Pages + reprepro, GPG-signed) | multi-arch `.deb`; replaces pkexec updater |
| 2b | AUR `linvclipboard-bin` + `linvclipboard` | real `sha256sums`, `.SRCINFO`, LICENSE |
| 2c | Fedora COPR / openSUSE OBS | offline-buildable spec (`cargo vendor`, npm offline cache), `%systemd_user_*` macros |
| 2d | Flathub | metainfo.xml, `flatpak-cargo-generator`, portal-based background; document Wayland data-control limitation on GNOME |
| 2e | Homebrew/Linuxbrew tap | cheap, CLI + daemon |
| 3a | winget | fix owner, `ProductCode = com.linvclipboard.app`, real SHA; automate |
| 3b | Scoop (own bucket, portable zip) | publish `_x64-portable.zip` |
| 3c | Chocolatey | fix owner/checksum; `choco pack/push` in release |
| 3d | Code signing | SignPath Foundation (free OSS) or Azure Trusted Signing for `.exe`s; required for SmartScreen |
| 4 | Auto-update | Linux via repos; Windows via Tauri updater with real key; retire `lib.rs` pkexec path |

---

## 11. Remediation plan (phased, prioritised)

### Phase 0 — Stop the bleeding (≈1–2 days)
1. `cargo fmt --all`; fix `db.rs:796` clippy; `npm install` and commit lockfile; commit `Cargo.lock`; add `LICENSE`; add `[profile.release]`.
2. Fix the 9 Windows compile errors (C1) and `monitor_windows.rs` use-after-move; add `tracing` dep or replace with `eprintln!` in `lib.rs`; add Windows-target clippy to Linux lint job.
3. Generate updater keypair; commit pubkey; add secrets; fail release on missing `.sig`.
4. Un-ignore and commit `windows/hooks/installer.nsh`; `npx tauri icon`.
5. Add `Depends:` + `Maintainer:` e-mail to the `.deb`; run `lintian` in CI.
6. Implement `linvclip-ui --version`; add `TimeoutStartSec=60` to update-check service; drop `DISPLAY=:0`.
7. Fix `urlish_decode` (H3), `clipctl` byte slicing (H6), server `expect` (H4), blacklist checksum poisoning (H1), config fail-closed + early logging (H7).
8. Frontend: `filteredItems` via `useMemo` shared by list/keyboard/preview (H13); `isInput` guard before vim block (H14); remove `loading` from `fetchGifs` deps (H15); request sequencing (H16); `listen()` cancel flag (F-M1).
9. Delete dead packaging paths (`packaging/postinst`, `prerm`, cargo-deb metadata, `bundle.linux.deb`, `install/deb-postinst.sh`, `windows/resources/*`, `upgrade-plan/`); untrack `gen/schemas`, root `package-lock.json`, `.claude/settings.json`; fix `.gitignore` (`Cargo.lock`, `*.nsh`, tracked-but-ignored files).
10. Fix README: correct shortcut, version, edition badge, emoji count, add Windows section.

### Phase 1 — Make releases trustworthy (≈1 week)
- Split CI into PR gates + release workflow with artifact reuse, `SHA256SUMS`, attestations, multi-arch `.deb`, `.rpm`, AppImage (with daemon in AppRun), Windows NSIS + `.sig`.
- Add `cargo-deny`, `cargo-audit`, `npm audit`, Dependabot, SHA-pinned actions, `permissions`, `concurrency`.
- Strip Debian Policy violations from maintainer scripts (H18); add `conffiles`; remove self-`Replaces`/`Breaks`; remove IBus `gsettings` hack.
- Replace Linux pkexec updater with verified download + fixed helper (or notify-only until apt repo exists) (C3).
- `install.sh` / `install.ps1` per §10; Docker smoke matrix.
- `release-plz`/`cargo-release` for version bumps + CHANGELOG; generate all manifests in CI.

### Phase 2 — Correctness & robustness (≈2 weeks)
- Daemon: size caps for all content types; `List`/`Search` summaries + `Get`; request timeouts; permit-before-accept; `flock` single instance; bind in 0700 dir; proper shutdown join; `spawn_blocking` for DB/clipboard/PNG; blob paths relative; percent-encode `file://`; `set_html` with alt text; validated config + atomic write + hot reload; implement or remove dead security knobs; tags in model; FTS escaping fix; regex search streaming + correct total; PRAGMAs via `with_init`; `rowid INTEGER PRIMARY KEY`.
- Windows: `read_exact` framing + loopback test; `first_pipe_instance` + DACL + per-user pipe name; pre-create next instance + backoff; share `handle_request` between platforms via `ClipboardProvider`; real event-driven monitor on a dedicated thread; delete or wire the Unix PAL.
- Tauri: split `lib.rs` into modules; `LazyLock` syntect; async highlight commands; path scoping for `get_image_base64`/OCR; opt-in link preview with host policy + streaming cap; `tauri-plugin-single-instance`; `set_shortcut` command; `visible: false`; CSP corrections; KLIPY key via env secret; `reqwest` timeouts.
- Frontend: ESLint (`react-hooks`), Prettier, Vitest with tests for `smartDetect`, `resolveAction`, selection logic; `lazy()` all pickers; virtualise lists; `React.memo(ClipItem)`; single source of truth for settings (`useConfig()`); split `App.jsx`/`SettingsPanel.jsx`; consistent redaction incl. preview/aria; route all URLs via plugin `open()`; Escape handling in every modal; fix undefined CSS vars, swatch collision, dead CSS; i18n interpolation + dev warnings + parity check in CI; translate the untranslated sections.
- Tests: the list in §6.

### Phase 3 — Distribution & polish (ongoing)
- apt repo, AUR, COPR, Flathub, winget, Scoop, Chocolatey, Homebrew tap, code signing (§10 roadmap).
- Wayland-native monitoring (`ext-data-control`) and focus detection; D-Bus toggle for Wayland shortcut; encryption at rest; `clipctl` protocol parity; `docs/` with ARCHITECTURE/PROTOCOL/CONTRIBUTING/SECURITY; screenshots; TypeScript migration.

---

## 12. Final verdict

### Scorecard (1 = absent/broken, 5 = industry standard)

| Dimension | Score | Rationale |
|---|---|---|
| Architecture & design | **3.5** | Daemon/client split, shared protocol, SQLite+FTS5, systemd hardening are the right shapes. PAL is a third copy rather than a consolidation; 1,869-line lib.rs; God components. |
| Correctness | **2** | 3 verified panics reachable from clipboard content or CLI use; wrong-item delete in the UI; blacklist bypass; server boot panic; use-after-move on Windows. |
| Security | **2** | Root-privilege updater with injection surface and no verification; blacklist inert on mainstream Wayland; SSRF via link preview; unimplemented "security" knobs; pipe squatting on Windows. Good: CSP, no shell interpolation for tools, 0700 socket. |
| Performance | **2.5** | 12 fork/exec per second on unsupported desktops; full-image hashing 4×/s; syntect reload per keystroke; 522 KB startup chunk; 4,000 unvirtualised buttons. |
| Build & CI | **1** | Red for six months; cannot `npm ci`; cannot compile Windows; release job rebuilds and publishes one artifact. |
| Packaging & distribution | **1.5** | `.deb` without `Depends`; policy violations; stale/placeholder RPM, PKGBUILD, winget, scoop, choco; no one-liner; no repo; no checksums; no LICENSE. |
| Tests | **1.5** | 24 tests in one crate; zero elsewhere; no frontend runner; IPC tests don't touch a socket. |
| Documentation & governance | **1.5** | README partially stale and wrong on the shortcut/version; no CHANGELOG/CONTRIBUTING/SECURITY; planning docs in tree. |
| Windows port | **1** | Never compiled. Scaffolding. |
| **Overall** | **1.9 / 5** | **Pre-alpha engineering wrapped around a beta-quality feature set.** |

### Bottom line

The user's self-assessment is correct: the project does not meet industry standards, and the repo setup is far from single-link installation. But the gap is closable. Almost every Critical item is a one-day fix (fmt, lockfile, LICENSE, pubkey, `Depends`, nine type errors, one `.nsh` file). The structural work (CI redesign, updater replacement, module split, tests, package repos) is a few focused weeks. Nothing found requires a rewrite; the daemon design, protocol, and DB schema are worth keeping.

**Recommended immediate sequence:** Phase 0 in full → cut `v3.0.4` from a green CI as the first release ever produced by the pipeline → Phase 1 → publish `install.sh`/`install.ps1` and the apt repo → then Phase 2. Do not add features until Phase 1 is done; every feature added on top of a red pipeline and an unverified updater increases the blast radius.
