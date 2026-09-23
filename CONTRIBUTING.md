# Contributing

## Quick start

```bash
git clone https://github.com/DreamerX00/LinVClipBoard.git
cd LinVClipBoard
make deb   # builds everything, then packaging/build-deb.sh
```

System dependencies are listed in `README.md` and installed by the composite
action `.github/actions/linux-deps` — if you add one, update both places.

## Rules that keep releases working

1. **Versions are synced, never hand-drifted.** Bump together:
   `Cargo.toml` (`[workspace.package]`), `crates/linvclip-ui/package.json`,
   `crates/linvclip-ui/src-tauri/tauri.conf.json`, `packaging/PKGBUILD`,
   `packaging/linvclipboard.spec`, and `windows/publish/*`. (`build-deb.sh`
   reads the version from the workspace `Cargo.toml`.) Then run
   `cargo update --workspace` so `Cargo.lock` picks up the new crate versions
   — CI builds with `--locked` and fails on a stale lockfile.
2. **Lockfiles are committed and authoritative.** After editing
   `package.json` run `npm install` and commit `package-lock.json`
   (`npm ci` in CI refuses an out-of-sync lockfile). After editing a
   `Cargo.toml` dependency, commit the resulting `Cargo.lock` change.
3. **Tauri npm packages and Rust crates stay on the same minor.** The Tauri
   CLI aborts `tauri build` when e.g. `@tauri-apps/api` 2.11 meets `tauri`
   2.10. `package.json` pins `@tauri-apps/*` with `~` ranges for that reason;
   when you bump `tauri`/`tauri-plugin-*` in `Cargo.toml`, bump the matching
   `~` range and re-run `npm install`.
4. **The Rust toolchain is pinned** in `rust-toolchain.toml` so CI and local
   builds share one compiler. Bump it deliberately and run `make ci` first —
   new stable releases add clippy lints.
5. **Never commit binaries or generated files.** `dist/`, `*.exe`,
   `src-tauri/resources/*.exe` and `src-tauri/gen/schemas/` are gitignored.
   Release artifacts are produced by the CI `release` job when a `v*` tag is
   pushed (see "Cutting a release" below).
6. **Run the gates before pushing:** `make ci` — builds the frontend, then
   runs the same `cargo fmt`, `cargo clippy --all-targets -D warnings` and
   `cargo test` invocations as the Lint/Test jobs in
   `.github/workflows/ci.yml`. Windows-only code is compiled and tested by the
   `windows-2025` jobs on the PR.
7. Update `CHANGELOG.md` for user-visible changes.

## Cutting a release

```bash
make release        # = scripts/release.sh
```

The script is interactive: it asks for the version (suggesting the next
patch/minor/major), a headline, and the release notes (opens `$EDITOR` with a
template), then shows a plan and, on confirmation:

1. bumps every file from rule 1 above plus `README.md`, `docs/INSTALL.md` and
   `SECURITY.md`, re-syncs `Cargo.lock`/`package-lock.json`, and adds a
   `## [X.Y.Z] - date` section with your notes to `CHANGELOG.md`;
2. commits `Release vX.Y.Z`, creates an annotated tag and pushes both;
3. builds the artifacts — on GitHub Actions (default; the Windows `.exe` is
   built on a real Windows runner and you watch the jobs live), locally
   (`.deb` + `.tar.gz`, plus the `.exe` when `cargo-xwin` and `makensis` are
   installed), or both;
4. publishes the GitHub release with your notes, GitHub's generated
   "What's Changed" list, the `.deb`, `.tar.gz`, `.exe` and `SHA256SUMS`.

`scripts/release.sh --dry-run` walks through everything without changing
anything; `--help` lists the flags for non-interactive use. Everything up to
the push can be rolled back from the script if a step fails. The release
body on a tag push always comes from the matching `CHANGELOG.md` section
(`packaging/release-notes.sh`), so keep that section accurate.

## Windows-only Tauri config

`tauri.conf.json` holds only cross-platform settings. Everything that needs
`clipd.exe`/`clipctl.exe` (bundle resources, NSIS target, updater artifacts)
lives in `tauri.windows.conf.json`, which Tauri merges in only when the build
target is Windows. Keep it that way — `tauri-build` validates
`bundle.resources` at compile time, so listing `.exe` files in the base config
breaks every Linux job with "resource path doesn't exist".
