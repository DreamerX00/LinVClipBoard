# Changelog

All notable changes to this project are documented here. Versions follow
[SemVer](https://semver.org/); release artifacts are published on the
[Releases](https://github.com/DreamerX00/LinVClipBoard/releases) page.

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
