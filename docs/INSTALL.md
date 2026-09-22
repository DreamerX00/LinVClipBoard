# Installing LinVClipBoard

Download release files from the
[latest release](https://github.com/DreamerX00/LinVClipBoard/releases/latest).

## 1. Verify checksums

Release assets ship with a `SHA256SUMS` file holding bare filenames, so
verify inside your download directory:

```bash
sha256sum --ignore-missing -c SHA256SUMS
# every file you downloaded must print OK — otherwise stop and re-download
```

## 2. Install

**Debian/Ubuntu** (dependencies resolve automatically):

```bash
sudo apt install ./linvclipboard_3.1.0-1_amd64.deb
```

**Fedora/RHEL:**

```bash
sudo dnf install ./linvclipboard-3.1.0-1.x86_64.rpm
```

**Other distros** (no root needed):

```bash
tar xzf linvclipboard-3.1.0-linux-x86_64.tar.gz
cd linvclipboard-3.1.0-linux-x86_64
./install-user.sh
```

## 3. Confirm the daemon

```bash
systemctl --user enable --now clipd.service
systemctl --user status clipd
journalctl --user -u clipd -f   # live logs if anything looks off
```

Press `Ctrl+/` to open the overlay.

## Uninstall

```bash
sudo apt remove linvclipboard   # or: sudo dnf remove linvclipboard
systemctl --user disable --now clipd.service linvclip-update-check.timer
rm -rf ~/.config/linvclip ~/.local/share/linvclip   # optional: user data
```
