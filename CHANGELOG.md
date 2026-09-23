# Changelog

All notable changes to this project are documented here. Versions follow
[SemVer](https://semver.org/); release artifacts are published on the
[Releases](https://github.com/DreamerX00/LinVClipBoard/releases) page.

## [3.2.2] - 2026-09-23

### 🐛 Fixed

- **Check for Updates works again on every platform.** The check now reads the release manifest (`latest.json`) published with each release instead of the GitHub API, which is rate-limited to 60 requests/hour per IP and was answering `403` — the GitHub API is only a fallback now
- **Windows auto-update actually works.** The updater public key in `tauri.conf.json` was still the `PLEASE_REPLACE…` placeholder, so the Tauri updater could never verify a release and every Windows update attempt ended in an error; installers are now signed in CI and verified against the real key
- **Windows: chained update flow when the plugin can't be used.** If the signed manifest is unreachable, the app downloads the installer itself, verifies its SHA-256 and runs it (passive UI, relaunch when done) instead of showing "Updater plugin install is Windows-only"
- **Windows: the daemon is stopped before an update.** `clipd.exe` kept the old binary locked while the installer tried to replace it; the installer hooks (which used macro names Tauri never invoked and therefore never ran) now stop it first, and the updater does the same before launching the installer
- **GIF tab works in the Windows build.** The Windows CI job never received the KLIPY key, so every Windows installer shipped with GIF search disabled; it does now, and tag builds fail instead of shipping keyless. Image loading also allows every `*.klipy.com` CDN host
- Linux: downloaded `.deb` updates are checked against the SHA-256 from the manifest before `dpkg` runs them as root; the version string can no longer smuggle a path into the download file name

### 🔧 Build / CI

- `release` job writes `latest.json` (Windows signature + URL + SHA-256, Linux `.deb` URL + SHA-256, release notes) and fails when the Windows `.sig` is missing
- `build-windows` fails on `v*` tags when `TAURI_SIGNING_PRIVATE_KEY` is not set
- `KLIPY_REQUIRE_KEY=1` (set on tag builds) turns the missing-key warning in `build.rs` into a build error

## [3.2.1] - 2026-09-23

### 🐛 Fixed

- GIF tab no longer sits on a permanent spinner: a failed categories request now shows an error with a Retry button, and a build without a KLIPY key shows a localized "GIF search is unavailable" message instead of the raw `gif_api_key_missing` code
- GIF search no longer refetches page 1 in a loop after every response (one request per 300 ms keystroke pause; stale responses are discarded; infinite scroll appends one page at a time)
- KLIPY requests time out after 10 s instead of hanging

### 🔧 Build / CI

- The KLIPY app key can be supplied via the `KLIPY_API_KEY` environment variable (CI secret); `klipy.key` remains the local-dev fallback, and `cargo` now warns when a build has no key
- Frontend ESLint + Vitest gates (`npm run lint`, `npm test`) run in the Lint job and `make ci`

## [3.2.0] - 2026-09-23

### ✨ New

- First official Windows build: NSIS installer (`LinVClipBoard_3.2.0_x64-setup.exe`) published to Releases, with `clipd`/`clipctl` bundled
- Windows auto-update feed (`update-windows-x86_64.json`) for the Tauri updater
- winget, Scoop, and Chocolatey manifests point at the real 3.2.0 downloads
- One-line Linux installer (`curl …/install.sh | bash`) with checksum verification, plus a jsDelivr mirror
- Interactive release pilot (`make release`): prompts, live CI watch, asset upload

### 🐛 Fixed

- CI green on every job (lint, Linux + Windows tests, daemon matrix, UI, Windows build) after months red
- Windows platform layer compiles: clipboard/input/monitor type errors fixed
- README install instructions rewritten (correct hotkey `Ctrl+/`, real asset names, verified emoji count)

### 🔧 Changed

- Linux packaging optimized: `.deb` declares real `Depends`, `.rpm` + portable tarball + `SHA256SUMS` shipped together
- Release binaries no longer stored in git — CI publishes them on tag
- `Cargo.lock` committed and `--locked`/`npm ci` enforced for reproducible builds

## [3.1.0] - 2026-09-22

### Distribution (new)

- Linux release set: `linvclipboard_3.1.0-1_amd64.deb` (with real `Depends`),
  `linvclipboard-3.1.0-1.x86_64.rpm`, portable
  `linvclipboard-3.1.0-linux-x86_64.tar.gz`, plus `SHA256SUMS`.
- `.deb` contents: `clipd`, `clipctl`, `linvclip-ui`, systemd user units,
  `apply-update` helper with polkit policy, man pages, shell completions,
  AppStream metainfo, icons.

### Repository hygiene

- Single version source synced to 3.1.0 (`Cargo.toml` workspace,
  `package.json`, `tauri.conf.json`, RPM spec, PKGBUILD, winget/scoop/
  Chocolatey manifests).
- Release binaries are no longer stored in git (`dist/` and
  `src-tauri/binaries/` are now ignored; CI publishes them on tag).
- `Cargo.lock` is committed for reproducible builds.
- Windows NSIS hook scripts (`windows/hooks/*.nsh`) are committed so a clean
  clone can bundle.
- Correct repo owner (`DreamerX00`) in all publish manifests.

### Docs

- README install section rewritten for the 3.1.0 assets with checksum
  verification, correct default shortcut (`Ctrl+/`), and an honest Windows
  note. Added `docs/INSTALL.md`, `CONTRIBUTING.md`, `SECURITY.md`, `LICENSE`.
