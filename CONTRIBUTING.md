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
6. **Run the gates before pushing:** `make ci` — builds the frontend, runs
   `npm run lint && npm test` in `crates/linvclip-ui`, then the same
   `cargo fmt`, `cargo clippy --all-targets -D warnings` and `cargo test`
   invocations as the Lint/Test jobs in `.github/workflows/ci.yml`. Windows-only code is compiled and tested by the
   `windows-2025` jobs on the PR.
7. Update `CHANGELOG.md` for user-visible changes.

## GIF search: the KLIPY API key

GIF search talks to [KLIPY](https://klipy.com). The app key is embedded at
compile time by `crates/linvclip-ui/src-tauri/build.rs`, which looks for it in
this order:

1. the `KLIPY_API_KEY` environment variable — **this is what CI uses**;
2. `crates/linvclip-ui/src-tauri/klipy.key` (gitignored) — local-dev fallback.

If neither is set the build still succeeds, but `cargo` prints a
`KLIPY API key not found` warning and every GIF command returns
`gif_api_key_missing`; the GIF tab then shows a localized "GIF search is
unavailable" message instead of results. A release artifact built that way
ships without GIF search, so:

- **CI / releases:** set the repository secret `KLIPY_API_KEY` (Settings →
  Secrets and variables → Actions). `.github/workflows/ci.yml` passes it to
  the `build-ui`, `build-windows` and `release` jobs as an environment
  variable; `cargo:rerun-if-env-changed=KLIPY_API_KEY` makes cargo rebuild
  the UI crate when it changes.
- **Local builds:** `export KLIPY_API_KEY=…` before `make`/`npx tauri build`,
  or drop the key into `crates/linvclip-ui/src-tauri/klipy.key`.
- **Never commit a key.** `klipy.key` is gitignored; do not add the key to
  `tauri.conf.json`, the Makefile, workflow files, or tests. `build.rs` only
  ever prints *where* the key came from, never the value.

## Frontend lint and tests

`crates/linvclip-ui` has ESLint and Vitest gates alongside the Rust ones:

```bash
cd crates/linvclip-ui
npm run lint   # eslint . (flat config in eslint.config.js)
npm test       # vitest run — *.test.{js,jsx} under src/, jsdom environment
```

Tauri's `invoke` is mocked in `src/test/setup.js`; component tests stub it per
test. The Lint job in CI runs both after `npm ci`.

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

## Auto-update: signing key and release manifest

Windows builds update themselves through the Tauri updater plugin, which only
accepts an installer whose `.sig` verifies against `plugins.updater.pubkey` in
`crates/linvclip-ui/src-tauri/tauri.conf.json`. The matching private key lives
**only** in the repository secrets:

- `TAURI_SIGNING_PRIVATE_KEY` — contents of the private key file produced by
  `npx tauri signer generate -w <file>` (the base64 blob, not a path);
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — its password.

Rules:

- **Never commit the private key.** Only the public key goes in
  `tauri.conf.json`. Rotating the key orphans every installed copy (their
  embedded pubkey no longer matches), so treat it as permanent.
- **Tag builds fail without the secret.** `build-windows` refuses to produce an
  unsigned installer on a `v*` tag, and the `release` job requires the `.sig`
  to write the manifest. Branch/PR builds still work unsigned.
- The `release` job publishes `latest.json` (and a `update-windows-x86_64.json`
  copy) next to the assets:
  `{ version, notes, pub_date, platforms: { "windows-x86_64": { signature, url, sha256 }, "linux-x86_64": { url, sha256 } } }`.
  The Tauri updater reads the Windows entry; the app's own update check
  (`check_for_updates` in `lib.rs`) reads it on every platform via
  `releases/latest/download/latest.json`, which — unlike `api.github.com`
  (60 unauthenticated requests/hour per IP) — is never rate-limited. The
  GitHub API remains a fallback for releases without a manifest.
- Linux installs the `.deb` from the manifest URL and refuses it if the
  SHA-256 does not match. Windows without a usable plugin (no manifest, offline
  pubkey mismatch) downloads the `.exe`, verifies it, and runs it with the same
  `/P /UPDATE /R` switches the plugin uses.
- `windows/hooks/installer.nsh` stops `clipd.exe` before files are copied;
  Tauri only closes the main app, and a running daemon would keep the old
  binary locked during an update.

## Windows-only Tauri config

`tauri.conf.json` holds only cross-platform settings. Everything that needs
`clipd.exe`/`clipctl.exe` (bundle resources, NSIS target, updater artifacts)
lives in `tauri.windows.conf.json`, which Tauri merges in only when the build
target is Windows. Keep it that way — `tauri-build` validates
`bundle.resources` at compile time, so listing `.exe` files in the base config
breaks every Linux job with "resource path doesn't exist".
