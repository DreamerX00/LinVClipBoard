.PHONY: build build-ui frontend deb rpm appimage tarball release install clean check test lint ci completions manpages \
        build-win build-win-cross build-win-ui

build:
	cargo build --release --locked -p clipd -p clipctl

build-ui:
	cd crates/linvclip-ui && npm ci && npx tauri build -- --locked

# tauri::generate_context!() embeds the built frontend at compile time, so
# anything that compiles linvclip-ui (clippy, tests) needs this first.
frontend:
	cd crates/linvclip-ui && npm ci && npm run build

build-all: build build-ui

check:
	cargo check --workspace --locked
	cargo test -p shared --locked

test:
	cargo test --workspace --locked

lint:
	cd crates/linvclip-ui && npm run lint && npm test
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets --locked -- -D warnings

# Same steps as the Lint/Test jobs in .github/workflows/ci.yml. Run before
# pushing; if this passes locally, CI will pass on Linux.
ci: frontend lint test

deb: build-all
	./packaging/build-deb.sh

rpm: build-all
	rpmbuild -ba packaging/linvclipboard.spec

appimage: build-all
	bash packaging/build-appimage.sh

tarball: build-all
	./packaging/build-tarball.sh

# Interactive release: prompts for version + notes, bumps every synced version
# file, updates CHANGELOG.md, commits, tags, pushes, then builds (locally and/or
# on GitHub Actions) and publishes the GitHub release. See scripts/release.sh.
release:
	./scripts/release.sh

completions: build
	mkdir -p target/completions
	target/release/clipctl completions bash > target/completions/clipctl.bash
	target/release/clipctl completions zsh  > target/completions/_clipctl
	target/release/clipctl completions fish > target/completions/clipctl.fish

manpages: build
	mkdir -p target/man
	target/release/clipctl manpage target/man

# ─── Windows cross-compilation targets ──────────────────────────────────

build-win:
	cargo build --target x86_64-pc-windows-msvc --release -p clipd -p clipctl

build-win-cross:
	cargo xwin build --target x86_64-pc-windows-msvc --release -p clipd -p clipctl

build-win-ui: build-win-cross
	cp target/x86_64-pc-windows-msvc/release/clipd.exe   crates/linvclip-ui/src-tauri/resources/
	cp target/x86_64-pc-windows-msvc/release/clipctl.exe crates/linvclip-ui/src-tauri/resources/
	cd crates/linvclip-ui && npx tauri build --target x86_64-pc-windows-msvc

.PHONY: build-windows-installer
build-windows-installer: ## Build Windows NSIS installer (cross-compile)
	cargo xwin build --release --target x86_64-pc-windows-msvc -p clipd -p clipctl
	cp target/x86_64-pc-windows-msvc/release/clipd.exe crates/linvclip-ui/src-tauri/resources/
	cp target/x86_64-pc-windows-msvc/release/clipctl.exe crates/linvclip-ui/src-tauri/resources/
	cd crates/linvclip-ui && npm install && npx tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis
	@echo "Windows NSIS installer built: crates/linvclip-ui/src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/"

install: build
	install -Dm755 target/release/clipd   $(DESTDIR)$(HOME)/.local/bin/clipd
	install -Dm755 target/release/clipctl $(DESTDIR)$(HOME)/.local/bin/clipctl
	install -Dm644 install/clipd.service  $(DESTDIR)$(HOME)/.config/systemd/user/clipd.service
	install -Dm644 install/linvclipboard.desktop $(DESTDIR)$(HOME)/.local/share/applications/linvclipboard.desktop
	systemctl --user daemon-reload
	systemctl --user enable --now clipd.service

clean:
	cargo clean
	rm -rf crates/linvclip-ui/dist crates/linvclip-ui/node_modules target/completions target/man
