# Changelog

All notable changes to this project are documented here. Versions follow
[SemVer](https://semver.org/); release artifacts are published on the
[Releases](https://github.com/DreamerX00/LinVClipBoard/releases) page.

## [3.3.2] - 2026-09-23

Fixed Final Bugs of This Version Now Use It Howerver You Like

## [Unreleased]

### 🐛 Fixed

- **GIF tab works again, and stays working when the KLIPY key is rotated.** The key used to be compiled into every build, so a rotated or revoked key broke GIF search in every released version at once (the error users saw was "Couldn't load GIF categories … error sending request"). The app now downloads the key and API endpoint from [`gif-provider.json`](gif-provider.json) on the `main` branch at runtime, caches it for 6 hours, and re-downloads it immediately when KLIPY rejects the current key. Rotating the key is a one-line commit; no rebuild, no release.
- GIF requests use HTTP/1.1: KLIPY's edge was observed stalling HTTP/2 GETs for 15 s or more (the request then hit the 10 s timeout) while HTTP/1.1 answers in under half a second.
- GIF error messages no longer echo the request URL, which contained the API key. Network problems, a missing key and a rejected key each get a localized message with a Retry button.

### ✨ New

- `[gif]` section in `config.toml`: `api_key` (use your own KLIPY key), `base_url` (proxy), `provider_url` (forks). `KLIPY_API_KEY` in the environment works like `api_key`.

### 🔧 Build / CI

- The `KLIPY_API_KEY` repository secret, the `klipy.key` file and the key-embedding `build.rs` are gone; CI no longer needs the secret to release. The Lint job validates `gif-provider.json` instead.
- Unit tests for the GIF provider resolution (download, cache, rotation retry, throttling, offline fallback, error redaction) against a local mock server, plus an opt-in live test against KLIPY (`cargo test -p linvclip-ui -- --ignored`).
- Webview CSP `img-src` allows any `https:` host so a change of KLIPY's CDN does not need a release either.

## [3.3.1] - 2026-09-23

### 🐛 Fixed

- *Check for Updates* no longer depends on the GitHub REST API, whose 60 requests/hour limit is shared by everyone behind the same public IP and produced a "GitHub API error: 403" on shared networks. The app now reads the release's update manifest from GitHub's CDN first (no rate limit) and only falls back to the API if that is unavailable. The manifest carries a `linux-x86_64` entry for the `.deb` for this purpose.

### 🔧 Build / CI

- Unit tests for the version comparison and `SHA256SUMS` lookup used by the updater.

## [3.3.0] - 2026-09-23

### ✨ New

- **In-app updates now work end to end.** Windows builds are signed with the project's updater key and the release carries `update-windows-x86_64.json`, so *Check for Updates* → *Download Now* downloads, verifies, installs (passive installer) and relaunches the app. Linux keeps the `.deb` flow (download → *Install Now* → pkexec) and now verifies the package against the release `SHA256SUMS` first; Windows falls back to the same checksum-verified installer download if the signed manifest is ever unavailable.
- Release notes are shown in the update dialog on both platforms.

### 🐛 Fixed

- GIF tab works in the Windows installer: the Windows build job never received the KLIPY key, so every published `.exe` had GIF search disabled.
- Windows installer/updater: the NSIS hooks used macro names Tauri never runs, so nothing in them applied. They now stop `clipd.exe` before files are replaced (a running daemon blocked updates), keep autostart and clipboard history across updates, and respect the uninstaller's "delete app data" choice for clipd's data.
- Update check has a 15 s timeout instead of hanging on a stalled connection, treats pre-release versions correctly, and only offers the `.deb` on systems that have `dpkg` (others get *Visit GitHub*).

### 🔧 Build / CI

- Tag builds fail early when `KLIPY_API_KEY` or `TAURI_SIGNING_PRIVATE_KEY` is missing, and the release job refuses to publish without a signed installer whose version matches the tag — a release can no longer silently ship without update support.
- `CONTRIBUTING.md` documents the updater signing key and why it must not be rotated casually.

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
