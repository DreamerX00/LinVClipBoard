%global debug_package %{nil}

Name:           linvclipboard
Version:        3.2.0
Release:        1%{?dist}
Summary:        Clipboard history manager for Linux (Win+V equivalent)
License:        MIT
URL:            https://github.com/DreamerX00/LinVClipBoard
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  cargo >= 1.75
BuildRequires:  gcc
BuildRequires:  pkg-config
BuildRequires:  sqlite-devel
BuildRequires:  libxcb-devel
BuildRequires:  wayland-devel
BuildRequires:  webkit2gtk4.1-devel
BuildRequires:  libappindicator-gtk3-devel
BuildRequires:  librsvg2-devel
BuildRequires:  nodejs >= 18

Requires:       sqlite-libs
Requires:       libxcb
Requires:       wayland
Recommends:     wtype
Suggests:       xdotool
Suggests:       ydotool

%description
LinVClipBoard is a Win+V style clipboard manager for Linux.
Captures text and images, supports full-text search, pinning,
and a beautiful overlay UI. Works on X11 and Wayland.

Includes:
  clipd       — background clipboard capture daemon
  clipctl     — CLI client
  linvclip-ui — Tauri overlay window (Ctrl+/)

%prep
%setup -q

%build
cargo build --release -p clipd -p clipctl
cd crates/linvclip-ui && npm ci && npx tauri build

%install
install -Dm755 target/release/clipd        %{buildroot}%{_bindir}/clipd
install -Dm755 target/release/clipctl      %{buildroot}%{_bindir}/clipctl
install -Dm755 target/release/linvclip-ui  %{buildroot}%{_bindir}/linvclip-ui
install -Dm644 install/clipd.service       %{buildroot}%{_userunitdir}/clipd.service
install -Dm644 install/linvclipboard.desktop %{buildroot}%{_datadir}/applications/linvclipboard.desktop
install -Dm644 crates/linvclip-ui/src-tauri/icons/icon.png \
    %{buildroot}%{_datadir}/icons/hicolor/128x128/apps/linvclipboard.png

%post
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database %{_datadir}/applications 2>/dev/null || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f %{_datadir}/icons/hicolor 2>/dev/null || true
fi

%preun
systemctl --user --no-ask-password stop clipd.service 2>/dev/null || true
systemctl --user --no-ask-password disable clipd.service 2>/dev/null || true

%files
%license LICENSE
%{_bindir}/clipd
%{_bindir}/clipctl
%{_bindir}/linvclip-ui
%{_userunitdir}/clipd.service
%{_datadir}/applications/linvclipboard.desktop
%{_datadir}/icons/hicolor/128x128/apps/linvclipboard.png

%changelog
* Wed Sep 23 2026 LinVClipBoard Contributors <noreply@linvclipboard.dev> - 3.2.0-1
- The Complete Win+V For Linux, Now On Windows As well
* Mon Sep 22 2026 LinVClipBoard Contributors <noreply@linvclipboard.dev> - 3.1.0-1
- Linux release artifacts: .deb (with Depends), .rpm, portable tarball, SHA256SUMS
- Version sync: Cargo workspace, npm package, Tauri config, and all manifests
* Sun Mar 02 2026 LinVClipBoard Contributors <noreply@linvclipboard.dev> - 1.1.0-1
- Full audit pass: 45 fixes, improvements, and new features
- Schema migration system, D-Bus interface, CI/CD pipeline
- UI: settings panel, image previews, theme toggle, accessibility
- Security: proper CSP, socket permissions, FTS5 hardening
- Packaging: RPM spec, AppImage, ARM64 cross-compilation
