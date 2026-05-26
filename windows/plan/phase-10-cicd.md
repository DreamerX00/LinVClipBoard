# Phase 10: Windows CI/CD Pipeline

**Version**: v3.0.0-ci  
**Effort**: 2-3 days  
**Dependencies**: Phase 8 (Packaging), Phase 9 (Updater)

---

## Objective

Extend the GitHub Actions CI/CD pipeline to build, test, and release Windows binaries. The existing pipeline (`ci.yml`) builds for Linux only. We will add Windows jobs to the matrix, enabling automatic Windows builds on every push and Windows release artifacts on tag pushes.

---

## Tasks

### 10.1 Update CI Matrix

**`.github/workflows/ci.yml`:**

```yaml
name: CI

on:
  push:
    branches: [main]
    tags: ['v*']
  pull_request:
    branches: [main]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace -- -D warnings

  test:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: Swatinem/rust-cache@v2
      - name: Install Linux deps
        if: matrix.os == 'ubuntu-latest'
        run: |
          sudo apt-get update
          sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev \
            libappindicator3-dev librsvg2-dev libxcb1-dev libxcb-render0-dev \
            libxcb-shape0-dev libxcb-xfixes0-dev
      - run: cargo test --workspace
      - run: cargo test -p shared
```

### 10.2 Add Windows Build Job

```yaml
  build-windows:
    needs: [lint, test]
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: stable
          targets: x86_64-pc-windows-msvc

      - name: Install Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 20

      - uses: Swatinem/rust-cache@v2

      - name: Install npm dependencies
        run: |
          cd crates/linvclip-ui
          npm ci

      - name: Build Rust binaries
        run: |
          cargo build --release -p clipd -p clipctl

      - name: Copy binaries to Tauri resources
        shell: pwsh
        run: |
          Copy-Item "target/release/clipd.exe" "crates/linvclip-ui/src-tauri/resources/" -Force
          Copy-Item "target/release/clipctl.exe" "crates/linvclip-ui/src-tauri/resources/" -Force

      - name: Build Tauri Windows app
        shell: pwsh
        env:
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY || '' }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD || '' }}
        run: |
          cd crates/linvclip-ui
          npx tauri build --bundles nsis

      - name: Upload Windows artifacts
        uses: actions/upload-artifact@v4
        with:
          name: windows-installer
          path: |
            crates/linvclip-ui/src-tauri/target/release/bundle/nsis/*.exe
            crates/linvclip-ui/src-tauri/target/release/bundle/nsis/*.exe.sig
            crates/linvclip-ui/src-tauri/target/release/bundle/nsis/*.nsis.zip

      - name: Upload Windows binaries
        uses: actions/upload-artifact@v4
        with:
          name: windows-binaries
          path: |
            target/release/clipd.exe
            target/release/clipctl.exe
```

### 10.3 Add Cross-Compilation Job (Linux → Windows)

For faster builds and lower CI cost, optionally cross-compile from Linux:

```yaml
  build-windows-cross:
    needs: [lint, test]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: stable
          targets: x86_64-pc-windows-msvc

      - name: Install cross-compilation tools
        run: |
          sudo apt-get update
          sudo apt-get install -y nsis lld llvm mingw-w64
          cargo install cargo-xwin

      - name: Build Rust binaries (cross)
        run: |
          cargo xwin build --release --target x86_64-pc-windows-msvc -p clipd -p clipctl

      - name: Build Tauri (cross)
        run: |
          cd crates/linvclip-ui
          cp ../target/x86_64-pc-windows-msvc/release/clipd.exe src-tauri/resources/
          cp ../target/x86_64-pc-windows-msvc/release/clipctl.exe src-tauri/resources/
          npm ci
          npx tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis

      - name: Upload cross-compiled artifacts
        uses: actions/upload-artifact@v4
        with:
          name: windows-installer-cross
          path: |
            crates/linvclip-ui/src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/*.exe
```

### 10.4 Update Release Job

```yaml
  release:
    needs: [build-daemon, build-ui, build-windows]
    if: startsWith(github.ref, 'refs/tags/v')
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Download Linux daemon artifacts
        uses: actions/download-artifact@v4
        with:
          name: daemon-binaries

      - name: Download Linux UI artifacts
        uses: actions/download-artifact@v4
        with:
          name: linux-installer

      - name: Download Windows artifacts
        uses: actions/download-artifact@v4
        with:
          name: windows-installer

      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          files: |
            target/debian/*.deb
            windows-installer/*.exe
            windows-installer/*.exe.sig
          generate_release_notes: true
```

### 10.5 Windows-Specific CI Considerations

**WebView2 Runtime:** On `windows-latest` GitHub runners, WebView2 is pre-installed. If not, the bootstrapper will download it.

**NSIS Build Prerequisites:**
GitHub's `windows-latest` runner has NSIS pre-installed at `C:\Program Files (x86)\NSIS\`.

**Rust Cache:**
Use `Swatinem/rust-cache@v2` which supports Windows. The cache key includes the target triple, so Windows and Linux caches are separate.

**Long Paths:**
Windows has a 260-char MAX_PATH limit by default. Enable long paths:
```yaml
- name: Enable long paths
  run: git config --system core.longpaths true
```

### 10.6 Test All CI Jobs

- [ ] `lint` — passes (Rust formatting + clippy)
- [ ] `test` — passes on windows-latest (all tests pass)
- [ ] `build-windows` — produces NSIS installer
- [ ] Test that Windows build is triggered on push to main
- [ ] Test that Windows build is triggered on PR to main
- [ ] Test that Windows release artifacts are attached to GitHub Release

### 10.7 CI Secrets

Add to GitHub repository secrets:
- `TAURI_SIGNING_PRIVATE_KEY` — private key for update signing
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — password for private key
- `CODE_SIGNING_THUMBPRINT` — Authenticode signing cert thumbprint (optional)
- `MICROSOFT_STORE_SECRET` — for Store submission (Phase 13)

---

## Deliverables

1. Updated `ci.yml` with Windows build/test/release jobs
2. Windows builds on every push and PR
3. Windows artifacts in GitHub Releases
4. Cross-compilation job (optional)

---

## Acceptance Criteria

- [ ] `cargo test --workspace` passes on `windows-latest`
- [ ] Windows NSIS installer is built in CI
- [ ] Windows artifacts are uploaded to GitHub Releases
- [ ] Linux builds still run (no Windows-only CI)
- [ ] Cross-compilation from Linux produces working binaries
- [ ] Release contains both `.deb` and `.exe` artifacts
- [ ] Phase 10 branch committed
