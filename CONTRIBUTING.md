# Contributing

## Quick start

```bash
git clone https://github.com/DreamerX00/LinVClipBoard.git
cd LinVClipBoard
make deb   # builds everything, then packaging/build-deb.sh
```

System dependencies are listed in `README.md` and installed verbatim in
`.github/workflows/ci.yml` — if you add one, update both places.

## Rules that keep releases working

1. **Versions are synced, never hand-drifted.** Bump together:
   `Cargo.toml` (`[workspace.package]`), `crates/linvclip-ui/package.json`,
   `crates/linvclip-ui/src-tauri/tauri.conf.json`, `packaging/PKGBUILD`,
   `packaging/linvclipboard.spec`, and `windows/publish/*`. (`build-deb.sh`
   reads the version from the workspace `Cargo.toml`.)
2. **Never commit binaries.** `dist/`, `*.exe`, and
   `src-tauri/binaries/` are gitignored. Release artifacts are produced by
   the CI `release` job when a `v*` tag is pushed.
3. **Run the gates before pushing:** `cargo fmt --all -- --check`,
   `cargo clippy --workspace -- -D warnings`, `cargo test`, and in
   `crates/linvclip-ui`: `npm run lint && npm test`.
4. Update `CHANGELOG.md` for user-visible changes.
