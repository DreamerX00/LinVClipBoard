#!/usr/bin/env bash
set -euo pipefail

# All-in-one build + package script for LinVClipBoard.
# Compiles Rust binaries, builds the frontend, and produces a single .deb.
#
# Usage:  ./packaging/build-deb.sh            # build everything + package
#         ./packaging/build-deb.sh --skip-build  # package only (binaries must exist)

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION=$(grep '^version' "${PROJECT_DIR}/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
ARCH="amd64"
PKG_NAME="linvclipboard"
PKG_DIR="$(mktemp -d)"
RELEASE_DIR="${PROJECT_DIR}/target/release"
OUT="${PROJECT_DIR}/target/debian/${PKG_NAME}_${VERSION}-1_${ARCH}.deb"

trap 'rm -rf "$PKG_DIR"' EXIT

SKIP_BUILD=false
if [ "${1:-}" = "--skip-build" ]; then
    SKIP_BUILD=true
fi

if [ "$SKIP_BUILD" = false ]; then
    echo "==> [1/4] Building Rust release binaries (clipd, clipctl, linvclip-ui)..."
    cd "${PROJECT_DIR}"
    cargo build --release

    echo "==> [2/4] Building frontend..."
    cd "${PROJECT_DIR}/crates/linvclip-ui"
    npm run build

    echo "==> [3/4] Preparing Tauri frontend assets..."
    cd "${PROJECT_DIR}/crates/linvclip-ui"
    # Frontend was already built above; the Rust binary was built in step 1.
    # Skip tauri bundling since we do our own .deb packaging.

    cd "${PROJECT_DIR}"
    echo "==> [4/4] Packaging ${PKG_NAME} ${VERSION}..."
else
    echo "==> Packaging ${PKG_NAME} ${VERSION} (skip-build mode)"
fi

# Verify binaries exist
for bin in clipd clipctl linvclip-ui; do
    if [ ! -f "${RELEASE_DIR}/${bin}" ]; then
        echo "ERROR: ${RELEASE_DIR}/${bin} not found. Run without --skip-build." >&2
        exit 1
    fi
done

# Create directory structure
mkdir -p "${PKG_DIR}/DEBIAN"
mkdir -p "${PKG_DIR}/usr/bin"
mkdir -p "${PKG_DIR}/usr/lib/systemd/user"
mkdir -p "${PKG_DIR}/usr/lib/linvclipboard"
mkdir -p "${PKG_DIR}/usr/share/applications"
mkdir -p "${PKG_DIR}/usr/share/icons/hicolor/128x128/apps"
mkdir -p "${PKG_DIR}/usr/share/doc/${PKG_NAME}"
mkdir -p "${PKG_DIR}/etc/xdg/autostart"

# Copy binaries
install -Dm755 "${RELEASE_DIR}/clipd"        "${PKG_DIR}/usr/bin/clipd"
install -Dm755 "${RELEASE_DIR}/clipctl"      "${PKG_DIR}/usr/bin/clipctl"
install -Dm755 "${RELEASE_DIR}/linvclip-ui"  "${PKG_DIR}/usr/bin/linvclip-ui"

# Copy assets
install -Dm644 "${PROJECT_DIR}/install/clipd.service" \
    "${PKG_DIR}/usr/lib/systemd/user/clipd.service"
install -Dm644 "${PROJECT_DIR}/install/linvclip-update-check.service" \
    "${PKG_DIR}/usr/lib/systemd/user/linvclip-update-check.service"
install -Dm644 "${PROJECT_DIR}/install/linvclip-update-check.timer" \
    "${PKG_DIR}/usr/lib/systemd/user/linvclip-update-check.timer"
install -Dm755 "${PROJECT_DIR}/install/linvclip-update-check.sh" \
    "${PKG_DIR}/usr/lib/linvclipboard/update-check.sh"
install -Dm644 "${PROJECT_DIR}/install/linvclipboard.desktop" \
    "${PKG_DIR}/usr/share/applications/linvclipboard.desktop"
install -Dm644 "${PROJECT_DIR}/install/linvclipboard-autostart.desktop" \
    "${PKG_DIR}/etc/xdg/autostart/linvclipboard.desktop"
install -Dm644 "${PROJECT_DIR}/crates/linvclip-ui/src-tauri/icons/icon.png" \
    "${PKG_DIR}/usr/share/icons/hicolor/128x128/apps/linvclipboard.png"

# Copyright
cat > "${PKG_DIR}/usr/share/doc/${PKG_NAME}/copyright" <<EOF
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: LinVClipBoard
License: MIT
Copyright: 2025-2026 LinVClipBoard Contributors
EOF

# Calculate installed size (in KB)
INSTALLED_SIZE=$(du -sk "${PKG_DIR}" | cut -f1)

# DEBIAN/control
cat > "${PKG_DIR}/DEBIAN/control" <<EOF
Package: ${PKG_NAME}
Version: ${VERSION}-1
Section: utils
Priority: optional
Architecture: ${ARCH}
Installed-Size: ${INSTALLED_SIZE}
Maintainer: LinVClipBoard Contributors
Replaces: linvclipboard, lin-v-clip-board
Breaks: linvclipboard (<< ${VERSION}-1)
Conflicts: lin-v-clip-board
Recommends: wtype | xdotool
Suggests: ydotool
Description: Clipboard history manager for Linux
 A Win+V style clipboard manager that captures text and images,
 with full-text search, pinning, and a beautiful overlay UI.
 Includes clipd (daemon), clipctl (CLI), and linvclip-ui (GUI).
 .
 For emoji/symbol insertion into text fields (like the Windows
 emoji panel), install wtype (Wayland) or xdotool (X11).
Homepage: https://github.com/DreamerX00/LinVClipBoard
EOF

# DEBIAN/preinst — clean up old manual installs before unpacking
cat > "${PKG_DIR}/DEBIAN/preinst" <<'PREINST'
#!/bin/sh
set -e

# ── Neuter old prerm scripts to prevent SIGPIPE disaster ──────────────
#
# During an upgrade, dpkg's order is:
#   1. NEW preinst (this script)
#   2. OLD prerm            ← this is the problem
#   3. Unpack new files
#   4. NEW postinst
#
# Old versions' prerm scripts unconditionally run `pkill -x linvclip-ui`,
# which kills the running UI.  When the in-app updater is active, the UI
# owns the stdout/stderr pipes that pkexec (and therefore dpkg) writes to.
# Killing the UI breaks those pipes, and the next write delivers SIGPIPE
# which silently kills dpkg mid-install — leaving a half-installed package,
# no update, and a stale binary.
#
# By replacing the old prerm with a no-op HERE (before dpkg invokes it),
# dpkg completes safely.  Our NEW prerm (installed alongside) handles
# cleanup correctly (only kills on full "remove", not "upgrade").
for pkg in linvclipboard lin-v-clip-board; do
    OLD_PRERM="/var/lib/dpkg/info/${pkg}.prerm"
    if [ -f "$OLD_PRERM" ]; then
        echo "Neutering old ${pkg}.prerm to prevent SIGPIPE during upgrade..."
        cat > "$OLD_PRERM" <<'NEUTERED'
#!/bin/sh
# Neutered by linvclipboard preinst to prevent SIGPIPE during upgrade.
exit 0
NEUTERED
        chmod 755 "$OLD_PRERM"
    fi
done

# Remove the old Tauri-generated "lin-v-clip-board" package if installed.
# It declares Conflicts: linvclipboard which blocks our install.
if dpkg -s lin-v-clip-board >/dev/null 2>&1; then
    echo "Removing old lin-v-clip-board package..."
    dpkg --remove --force-depends lin-v-clip-board 2>/dev/null || true
fi

# Remove old manual-install binaries that shadow /usr/bin/ paths
# (from install.sh / make install which put them in ~/.local/bin/)
cleanup_old_install() {
    USER="$1"
    HOME_DIR="$2"
    uid=$(id -u "$USER" 2>/dev/null) || return 0
    XDG_RUNTIME_DIR="/run/user/${uid}"
    [ -d "$XDG_RUNTIME_DIR" ] || return 0
    DBUS_SESSION_BUS_ADDRESS="unix:path=${XDG_RUNTIME_DIR}/bus"

    # Stop clipd via the old user service if it's running
    su "$USER" -c "XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR} DBUS_SESSION_BUS_ADDRESS=${DBUS_SESSION_BUS_ADDRESS} systemctl --user stop clipd.service" 2>/dev/null || true

    # Remove old manual-install binaries
    for bin in clipd clipctl linvclip-ui; do
        if [ -f "${HOME_DIR}/.local/bin/${bin}" ]; then
            rm -f "${HOME_DIR}/.local/bin/${bin}"
            echo "  Removed old ${HOME_DIR}/.local/bin/${bin}"
        fi
    done

    # Remove old user-level service files that shadow the system ones
    for svc in clipd.service linvclip-update-check.service linvclip-update-check.timer; do
        if [ -f "${HOME_DIR}/.config/systemd/user/${svc}" ]; then
            # Disable first so symlinks are cleaned up
            su "$USER" -c "XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR} DBUS_SESSION_BUS_ADDRESS=${DBUS_SESSION_BUS_ADDRESS} systemctl --user disable ${svc}" 2>/dev/null || true
            rm -f "${HOME_DIR}/.config/systemd/user/${svc}"
            echo "  Removed old ${HOME_DIR}/.config/systemd/user/${svc}"
        fi
    done

    # Daemon-reload to pick up the removal
    su "$USER" -c "XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR} DBUS_SESSION_BUS_ADDRESS=${DBUS_SESSION_BUS_ADDRESS} systemctl --user daemon-reload" 2>/dev/null || true
}

echo "Cleaning up old LinVClipBoard installations..."

# Remove orphaned /usr/bin/ binaries from old versions that weren't
# properly cleaned up (e.g. v1.4 uninstall leaving files behind).
# dpkg refuses to overwrite files not owned by any package.
for bin in clipd clipctl linvclip-ui; do
    if [ -f "/usr/bin/${bin}" ]; then
        owner=$(dpkg -S "/usr/bin/${bin}" 2>/dev/null || true)
        if [ -z "$owner" ]; then
            echo "  Removing orphaned /usr/bin/${bin} (not owned by any package)"
            rm -f "/usr/bin/${bin}"
        fi
    fi
done

# Also clean up orphaned systemd unit files in /usr/lib/systemd/user/
for svc in clipd.service linvclip-update-check.service linvclip-update-check.timer; do
    if [ -f "/usr/lib/systemd/user/${svc}" ]; then
        owner=$(dpkg -S "/usr/lib/systemd/user/${svc}" 2>/dev/null || true)
        if [ -z "$owner" ]; then
            echo "  Removing orphaned /usr/lib/systemd/user/${svc}"
            rm -f "/usr/lib/systemd/user/${svc}"
        fi
    fi
done

if [ -n "${SUDO_USER:-}" ] && [ "$SUDO_USER" != "root" ]; then
    SUDO_HOME=$(getent passwd "$SUDO_USER" | cut -d: -f6)
    cleanup_old_install "$SUDO_USER" "$SUDO_HOME"
else
    for user_run in /run/user/[0-9]*; do
        uid=$(basename "$user_run")
        [ "$uid" -ge 1000 ] 2>/dev/null || continue
        username=$(id -nu "$uid" 2>/dev/null) || continue
        user_home=$(getent passwd "$username" | cut -d: -f6)
        cleanup_old_install "$username" "$user_home"
    done
fi
PREINST
chmod 755 "${PKG_DIR}/DEBIAN/preinst"

# DEBIAN/postinst
cat > "${PKG_DIR}/DEBIAN/postinst" <<'POSTINST'
#!/bin/sh
set -e

# Update icon cache and desktop database
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database /usr/share/applications 2>/dev/null || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f /usr/share/icons/hicolor 2>/dev/null || true
fi

# Free up Super+. from IBus emoji picker so LinVClipBoard can use it.
# Save the original value so we can restore on uninstall.
free_shortcut() {
    USER="$1"
    HOME_DIR="$2"
    [ -n "$USER" ] && [ -n "$HOME_DIR" ] || return 0
    # Run gsettings as the actual user, not root
    ORIG=$(su - "$USER" -c "gsettings get org.freedesktop.ibus.panel.emoji hotkey" 2>/dev/null) || return 0
    # Save original if not already saved
    SAVE_FILE="${HOME_DIR}/.config/linvclip/.ibus-emoji-hotkey-backup"
    if [ ! -f "$SAVE_FILE" ]; then
        mkdir -p "${HOME_DIR}/.config/linvclip" 2>/dev/null || true
        chown "$USER":"$USER" "${HOME_DIR}/.config/linvclip" 2>/dev/null || true
        echo "$ORIG" > "$SAVE_FILE" 2>/dev/null || true
        chown "$USER":"$USER" "$SAVE_FILE" 2>/dev/null || true
    fi
    su - "$USER" -c "gsettings set org.freedesktop.ibus.panel.emoji hotkey '[]'" 2>/dev/null || true
}

# Apply to all logged-in users with a graphical session, or SUDO_USER
if [ -n "$SUDO_USER" ] && [ "$SUDO_USER" != "root" ]; then
    SUDO_HOME=$(getent passwd "$SUDO_USER" | cut -d: -f6)
    free_shortcut "$SUDO_USER" "$SUDO_HOME"
fi

# Enable and start clipd for all logged-in desktop users.
# dpkg runs as root, so we must target each real user's session.
enable_clipd() {
    USER="$1"
    uid=$(id -u "$USER" 2>/dev/null) || return 0
    XDG_RUNTIME_DIR="/run/user/${uid}"
    [ -d "$XDG_RUNTIME_DIR" ] || return 0
    DBUS_SESSION_BUS_ADDRESS="unix:path=${XDG_RUNTIME_DIR}/bus"

    # Helper: run a command as the target user with the correct
    # XDG_RUNTIME_DIR and DBUS_SESSION_BUS_ADDRESS so that
    # "systemctl --user" finds the right user manager instance.
    run_as() {
        su "$USER" -c "XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR} DBUS_SESSION_BUS_ADDRESS=${DBUS_SESSION_BUS_ADDRESS} $*" 2>/dev/null || true
    }

    HOME_DIR=$(getent passwd "$USER" | cut -d: -f6)

    # Remove any lingering user-level service files that shadow /usr/lib/systemd/user/
    for svc in clipd.service linvclip-update-check.service linvclip-update-check.timer; do
        if [ -f "${HOME_DIR}/.config/systemd/user/${svc}" ]; then
            run_as systemctl --user disable "${svc}" 2>/dev/null || true
            rm -f "${HOME_DIR}/.config/systemd/user/${svc}"
        fi
    done

    # Remove duplicate user-level autostart entry to prevent double tray icons
    # (system-level /etc/xdg/autostart/ from the deb is the canonical one)
    rm -f "${HOME_DIR}/.config/autostart/linvclipboard.desktop" 2>/dev/null || true

    run_as systemctl --user daemon-reload 2>/dev/null || true
    run_as systemctl --user enable clipd.service 2>/dev/null || true
    run_as systemctl --user enable linvclip-update-check.timer 2>/dev/null || true
    run_as systemctl --user start linvclip-update-check.timer 2>/dev/null || true

    # Restart clipd — use restart to handle both fresh install and upgrade
    run_as systemctl --user restart clipd.service 2>/dev/null || true

    # Kill any existing UI instances, then start exactly one.
    # UNLESS a self-update restart script exists — that means the UI triggered
    # this install and will handle its own restart after dpkg finishes.
    if ls /tmp/linvclip-restart-*.sh >/dev/null 2>&1; then
        echo "  Self-update detected — skipping UI launch (restart script will handle it)"
    else
        pkill -x linvclip-ui 2>/dev/null || true
        sleep 0.5
        run_as "setsid nohup /usr/bin/linvclip-ui >/dev/null 2>&1 &" 2>/dev/null || true
    fi
}

if [ -n "$SUDO_USER" ] && [ "$SUDO_USER" != "root" ]; then
    enable_clipd "$SUDO_USER"
else
    # Fallback: iterate all logged-in desktop users
    for user_run in /run/user/[0-9]*; do
        uid=$(basename "$user_run")
        [ "$uid" -ge 1000 ] 2>/dev/null || continue
        username=$(id -nu "$uid" 2>/dev/null) || continue
        enable_clipd "$username"
    done
fi

echo ""
echo "LinVClipBoard installed successfully!"
echo "The clipboard daemon (clipd) has been enabled and started."
echo ""
echo "Press Ctrl+/ to open the overlay,"
echo "or use 'clipctl' from the terminal."
echo ""
POSTINST
chmod 755 "${PKG_DIR}/DEBIAN/postinst"

# DEBIAN/prerm — runs as root, must iterate real user sessions (#6)
cat > "${PKG_DIR}/DEBIAN/prerm" <<'PRERM'
#!/bin/sh
set -e

echo "Stopping clipd service for active users..."

# When dpkg runs as root, systemctl --user targets root's session,
# not the actual desktop user's. Iterate over /run/user/* to find
# real user sessions and stop clipd in each.
for user_run in /run/user/[0-9]*; do
    uid=$(basename "$user_run")
    # Skip root (uid 0) and system users (<1000)
    [ "$uid" -ge 1000 ] 2>/dev/null || continue
    username=$(id -nu "$uid" 2>/dev/null) || continue
    su "$username" -c "XDG_RUNTIME_DIR=$user_run DBUS_SESSION_BUS_ADDRESS=unix:path=$user_run/bus systemctl --user stop clipd.service" 2>/dev/null || true
    su "$username" -c "XDG_RUNTIME_DIR=$user_run DBUS_SESSION_BUS_ADDRESS=unix:path=$user_run/bus systemctl --user stop linvclip-update-check.timer" 2>/dev/null || true
    # Only disable services on full remove, not on upgrade
    if [ "$1" = "remove" ]; then
        su "$username" -c "XDG_RUNTIME_DIR=$user_run DBUS_SESSION_BUS_ADDRESS=unix:path=$user_run/bus systemctl --user disable clipd.service" 2>/dev/null || true
        su "$username" -c "XDG_RUNTIME_DIR=$user_run DBUS_SESSION_BUS_ADDRESS=unix:path=$user_run/bus systemctl --user disable linvclip-update-check.timer" 2>/dev/null || true
    fi
done

# Only kill processes on full remove, not on upgrade
if [ "$1" = "remove" ]; then
    pkill -x clipd 2>/dev/null || true
    pkill -x linvclip-ui 2>/dev/null || true
fi
PRERM
chmod 755 "${PKG_DIR}/DEBIAN/prerm"

# DEBIAN/postrm
cat > "${PKG_DIR}/DEBIAN/postrm" <<'POSTRM'
#!/bin/sh
set -e

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database /usr/share/applications 2>/dev/null || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f /usr/share/icons/hicolor 2>/dev/null || true
fi

# Restore the original IBus emoji hotkey if we saved a backup
restore_shortcut() {
    USER="$1"
    HOME_DIR="$2"
    SAVE_FILE="${HOME_DIR}/.config/linvclip/.ibus-emoji-hotkey-backup"
    if [ -f "$SAVE_FILE" ]; then
        ORIG=$(cat "$SAVE_FILE")
        su - "$USER" -c "gsettings set org.freedesktop.ibus.panel.emoji hotkey \"$ORIG\"" 2>/dev/null || true
        rm -f "$SAVE_FILE"
    fi
}

if [ -n "$SUDO_USER" ] && [ "$SUDO_USER" != "root" ]; then
    SUDO_HOME=$(getent passwd "$SUDO_USER" | cut -d: -f6)
    restore_shortcut "$SUDO_USER" "$SUDO_HOME"
fi
POSTRM
chmod 755 "${PKG_DIR}/DEBIAN/postrm"

# Build the .deb
mkdir -p "$(dirname "$OUT")"
dpkg-deb --build --root-owner-group "${PKG_DIR}" "${OUT}"

echo "==> Created: ${OUT}"
echo "    Size: $(du -h "${OUT}" | cut -f1)"
echo ""
echo "Install with: sudo dpkg -i ${OUT}"
