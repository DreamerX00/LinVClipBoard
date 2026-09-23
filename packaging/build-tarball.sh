#!/usr/bin/env bash
set -euo pipefail

# Build the portable Linux tarball for LinVClipBoard:
#   target/tarball/linvclipboard-<version>-linux-x86_64.tar.gz
#
# Layout inside the archive (top-level dir linvclipboard-<version>-linux-x86_64/):
#   bin/clipd bin/clipctl bin/linvclip-ui
#   share/systemd/user/*.service|*.timer     (ExecStart rewritten to %h/.local)
#   share/applications/linvclipboard.desktop
#   share/icons/linvclipboard.png
#   lib/update-check.sh
#   install-user.sh / uninstall-user.sh      (no root needed, installs to ~/.local)
#   LICENSE  README.txt
#
# Binaries must already exist in target/release (run `make build-all` first).

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION=$(grep '^version' "${PROJECT_DIR}/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
ARCH="${ARCH:-x86_64}"
NAME="linvclipboard-${VERSION}-linux-${ARCH}"
RELEASE_DIR="${PROJECT_DIR}/target/release"
OUT_DIR="${PROJECT_DIR}/target/tarball"
OUT="${OUT_DIR}/${NAME}.tar.gz"
STAGE="$(mktemp -d)"
ROOT="${STAGE}/${NAME}"

trap 'rm -rf "$STAGE"' EXIT

for bin in clipd clipctl linvclip-ui; do
    if [ ! -f "${RELEASE_DIR}/${bin}" ]; then
        echo "ERROR: ${RELEASE_DIR}/${bin} not found. Run 'make build-all' first." >&2
        exit 1
    fi
done

echo "==> Packaging ${NAME}"

mkdir -p "${ROOT}/bin" "${ROOT}/lib" "${ROOT}/share/systemd/user" \
         "${ROOT}/share/applications" "${ROOT}/share/icons"

install -m755 "${RELEASE_DIR}/clipd" "${RELEASE_DIR}/clipctl" "${RELEASE_DIR}/linvclip-ui" "${ROOT}/bin/"
install -m644 "${PROJECT_DIR}/install/clipd.service" \
              "${PROJECT_DIR}/install/linvclip-update-check.service" \
              "${PROJECT_DIR}/install/linvclip-update-check.timer" \
              "${ROOT}/share/systemd/user/"
install -m755 "${PROJECT_DIR}/install/linvclip-update-check.sh" "${ROOT}/lib/update-check.sh"
install -m644 "${PROJECT_DIR}/install/linvclipboard.desktop" "${ROOT}/share/applications/"
install -m644 "${PROJECT_DIR}/crates/linvclip-ui/src-tauri/icons/icon.png" "${ROOT}/share/icons/linvclipboard.png"
install -m644 "${PROJECT_DIR}/LICENSE" "${ROOT}/LICENSE"

# The units ship with /usr paths; the user install lives under ~/.local.
# systemd expands %h to the user's home in ExecStart.
sed -i 's|^ExecStart=/usr/bin/clipd|ExecStart=%h/.local/bin/clipd|' "${ROOT}/share/systemd/user/clipd.service"
sed -i 's|^ExecStart=/usr/lib/linvclipboard/update-check.sh|ExecStart=%h/.local/lib/linvclipboard/update-check.sh|' \
    "${ROOT}/share/systemd/user/linvclip-update-check.service"

cat > "${ROOT}/install-user.sh" <<'INSTALL'
#!/usr/bin/env bash
# Install LinVClipBoard for the current user (no root). Everything goes under ~/.local.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${HOME}/.local/bin"
LIB="${HOME}/.local/lib/linvclipboard"
UNITS="${HOME}/.config/systemd/user"
APPS="${HOME}/.local/share/applications"
ICONS="${HOME}/.local/share/icons/hicolor/128x128/apps"

mkdir -p "$BIN" "$LIB" "$UNITS" "$APPS" "$ICONS"

echo "==> Installing binaries to ${BIN}"
install -m755 "$HERE"/bin/clipd "$HERE"/bin/clipctl "$HERE"/bin/linvclip-ui "$BIN/"

echo "==> Installing desktop entry, icon, update checker"
sed "s|^Exec=linvclip-ui|Exec=${BIN}/linvclip-ui|" "$HERE/share/applications/linvclipboard.desktop" \
    > "${APPS}/linvclipboard.desktop"
install -m644 "$HERE/share/icons/linvclipboard.png" "${ICONS}/linvclipboard.png"
install -m755 "$HERE/lib/update-check.sh" "${LIB}/update-check.sh"
update-desktop-database "$APPS" 2>/dev/null || true
gtk-update-icon-cache -f "${HOME}/.local/share/icons/hicolor" 2>/dev/null || true

echo "==> Installing systemd user units"
install -m644 "$HERE"/share/systemd/user/* "$UNITS/"
if command -v systemctl >/dev/null 2>&1; then
    systemctl --user daemon-reload
    systemctl --user enable --now clipd.service
    systemctl --user enable --now linvclip-update-check.timer 2>/dev/null || true
    systemctl --user is-active --quiet clipd.service \
        && echo "==> clipd is running" \
        || echo "!!  clipd did not start — check: journalctl --user -u clipd -e"
else
    echo "!!  systemd not found — start ${BIN}/clipd yourself (e.g. from your session autostart)."
fi

case ":$PATH:" in
    *":${BIN}:"*) ;;
    *) echo "!!  ${BIN} is not on your PATH — add it to use 'clipctl' from a terminal." ;;
esac

echo
echo "LinVClipBoard installed. Press Ctrl+/ to open the overlay."
echo "Uninstall with: ${HERE}/uninstall-user.sh"
INSTALL
chmod 755 "${ROOT}/install-user.sh"

cat > "${ROOT}/uninstall-user.sh" <<'UNINSTALL'
#!/usr/bin/env bash
# Remove a LinVClipBoard user install made by install-user.sh. Keeps your data in
# ~/.config/linvclip and ~/.local/share/linvclip unless you delete them yourself.
set -uo pipefail
if command -v systemctl >/dev/null 2>&1; then
    systemctl --user disable --now clipd.service linvclip-update-check.timer 2>/dev/null || true
fi
pkill -x linvclip-ui 2>/dev/null || true
rm -f "${HOME}/.local/bin/clipd" "${HOME}/.local/bin/clipctl" "${HOME}/.local/bin/linvclip-ui"
rm -rf "${HOME}/.local/lib/linvclipboard"
rm -f "${HOME}/.config/systemd/user/clipd.service" \
      "${HOME}/.config/systemd/user/linvclip-update-check.service" \
      "${HOME}/.config/systemd/user/linvclip-update-check.timer"
rm -f "${HOME}/.local/share/applications/linvclipboard.desktop" \
      "${HOME}/.local/share/icons/hicolor/128x128/apps/linvclipboard.png"
command -v systemctl >/dev/null 2>&1 && systemctl --user daemon-reload 2>/dev/null || true
echo "LinVClipBoard removed. Data left in ~/.config/linvclip and ~/.local/share/linvclip."
UNINSTALL
chmod 755 "${ROOT}/uninstall-user.sh"

cat > "${ROOT}/README.txt" <<README
LinVClipBoard ${VERSION} — portable Linux build (${ARCH})
https://github.com/DreamerX00/LinVClipBoard

Install for the current user (no root):   ./install-user.sh
Remove it again:                          ./uninstall-user.sh

Runtime requirements: glibc, GTK 3, WebKitGTK 4.1, libappindicator3, SQLite 3,
and either an X11 or Wayland session. On Debian/Ubuntu and Fedora prefer the
.deb / .rpm from the same release — they pull these in automatically.

Press Ctrl+/ to open the overlay once clipd is running.
README

mkdir -p "$OUT_DIR"
tar -C "$STAGE" --owner=0 --group=0 --numeric-owner -czf "$OUT" "$NAME"

echo "==> Created: ${OUT}"
echo "    Size: $(du -h "${OUT}" | cut -f1)"
