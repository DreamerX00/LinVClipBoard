.PHONY: build build-ui deb rpm appimage install clean check test lint completions manpages \
        build-win build-win-cross build-win-ui

build:
	cargo build --release -p clipd -p clipctl

build-ui:
	cd crates/linvclip-ui && npm install && npx tauri build

build-all: build build-ui

check:
	cargo check --workspace
	cargo test -p shared

test:
	cargo test --workspace

lint:
	cargo fmt --all -- --check
	cargo clippy --workspace -- -D warnings

deb: build-all
	./packaging/build-deb.sh

rpm: build-all
	rpmbuild -ba packaging/linvclipboard.spec

appimage: build-all
	bash packaging/build-appimage.sh

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
