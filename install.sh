#!/usr/bin/env bash
# LinVClipBoard one-line installer (Debian/Ubuntu).
#
#   curl -fsSL https://raw.githubusercontent.com/DreamerX00/LinVClipBoard/main/install.sh | bash
#
# Resolves the latest release, verifies checksums, and installs the .deb
# with apt (dependencies resolve automatically). Flags:
#   --version X.Y.Z   install a specific release instead of latest
#   --dry-run         print what would happen without downloading/installing
#   --uninstall       remove the package and its systemd units
set -euo pipefail

REPO="DreamerX00/LinVClipBoard"
PKG="linvclipboard"
VERSION="" DRY_RUN=0 UNINSTALL=0

while [ $# -gt 0 ]; do
    case "$1" in
        --version)   VERSION="${2:?missing version}"; shift 2 ;;
        --dry-run)   DRY_RUN=1; shift ;;
        --uninstall) UNINSTALL=1; shift ;;
        -h|--help)   sed -n '2,10p' "$0"; exit 0 ;;
        *) echo "Unknown flag: $1 (try --help)" >&2; exit 2 ;;
    esac
done

if [ "$UNINSTALL" = 1 ]; then
    [ "$DRY_RUN" = 1 ] && { echo "would: sudo apt remove -y $PKG + disable units"; exit 0; }
    sudo apt remove -y "$PKG"
    systemctl --user disable --now clipd.service linvclip-update-check.timer 2>/dev/null || true
    echo "Uninstalled. User data left in ~/.config/linvclip ~/.local/share/linvclip"
    exit 0
fi

if [ "$(id -u)" = 0 ]; then
    echo "Do not run as root — sudo is used only for the install step." >&2
    exit 2
fi

command -v curl >/dev/null || { echo "curl is required." >&2; exit 2; }

# Resolve latest tag via the releases/latest redirect (no API, no jq).
if [ -z "$VERSION" ]; then
    TAG_URL=$(curl -fsSIL -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest")
    VERSION="${TAG_URL##*/v}"
fi
DEB="${PKG}_${VERSION}-1_amd64.deb"
BASE="https://github.com/$REPO/releases/download/v$VERSION"

if [ "$DRY_RUN" = 1 ]; then
    echo "would: download $BASE/$DEB + SHA256SUMS, verify, sudo apt install"
    exit 0
fi

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
cd "$TMP"
curl -fsSL -O "$BASE/$DEB" -O "$BASE/SHA256SUMS"
sha256sum --ignore-missing -c SHA256SUMS
sudo apt install -y "./$DEB"
systemctl --user enable --now clipd.service
echo "Installed linvclipboard $VERSION. Press Ctrl+/ to open the overlay."
