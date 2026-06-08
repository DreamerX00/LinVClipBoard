#!/bin/bash
# Post-install script for LinVClipBoard deb package
# Runs as root after package files are unpacked

set -e

# Enable and start clipd for all logged-in users with graphical sessions
for USER_HOME in /home/*; do
    USER_NAME=$(basename "$USER_HOME")
    # Skip if not a real user
    id "$USER_NAME" >/dev/null 2>&1 || continue

    SYSTEMD_DIR="$USER_HOME/.config/systemd/user"
    mkdir -p "$SYSTEMD_DIR" 2>/dev/null || true

    # Copy service file to user's systemd directory
    cp /usr/share/linvclipboard/clipd.service "$SYSTEMD_DIR/clipd.service" 2>/dev/null || true
    chown -R "$USER_NAME":"$USER_NAME" "$SYSTEMD_DIR" 2>/dev/null || true

    # Remove stale local bin copies (install.sh leftovers) to avoid duplication
    rm -f "$USER_HOME/.local/bin/linvclip-ui" 2>/dev/null || true

    # Remove duplicate local desktop entry if deb provides the system one
    rm -f "$USER_HOME/.local/share/applications/linvclipboard.desktop" 2>/dev/null || true

    # Remove duplicate user-level autostart entry to prevent double icons
    # (system-level /etc/xdg/autostart/ from the deb is the canonical one)
    rm -f "$USER_HOME/.config/autostart/linvclipboard.desktop" 2>/dev/null || true

    # Try to enable and start the daemon for this user
    # Need the user's DBUS and XDG_RUNTIME_DIR
    UID_NUM=$(id -u "$USER_NAME" 2>/dev/null) || continue
    XDG_RUNTIME="/run/user/$UID_NUM"
    DBUS_ADDR="unix:path=$XDG_RUNTIME/bus"

    if [ -S "$XDG_RUNTIME/bus" ]; then
        su "$USER_NAME" -c "DBUS_SESSION_BUS_ADDRESS=$DBUS_ADDR XDG_RUNTIME_DIR=$XDG_RUNTIME HOME=$USER_HOME systemctl --user daemon-reload" 2>/dev/null || true
        su "$USER_NAME" -c "DBUS_SESSION_BUS_ADDRESS=$DBUS_ADDR XDG_RUNTIME_DIR=$XDG_RUNTIME HOME=$USER_HOME systemctl --user enable clipd.service" 2>/dev/null || true
        su "$USER_NAME" -c "DBUS_SESSION_BUS_ADDRESS=$DBUS_ADDR XDG_RUNTIME_DIR=$XDG_RUNTIME HOME=$USER_HOME systemctl --user restart clipd.service" 2>/dev/null || true
    fi
done

# Update desktop database
update-desktop-database /usr/share/applications 2>/dev/null || true

exit 0
