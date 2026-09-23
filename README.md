<p align="center">
  <img src="crates/linvclip-ui/src-tauri/icons/icon.png" width="120" alt="LinVClipBoard" />
</p>

<h1 align="center">LinVClipBoard</h1>

<p align="center">
  <strong>The clipboard manager Linux deserves.</strong><br/>
  Blazing fast &bull; Keyboard-first &bull; X11 + Wayland &bull; Under 50 MB RAM
</p>

<p align="center">
  <a href="https://github.com/DreamerX00/LinVClipBoard/releases/latest"><img src="https://img.shields.io/github/v/release/DreamerX00/LinVClipBoard?style=flat-square&color=6366f1&label=release" alt="Latest Release" /></a>
  <a href="https://github.com/DreamerX00/LinVClipBoard/blob/main/LICENSE"><img src="https://img.shields.io/github/license/DreamerX00/LinVClipBoard?style=flat-square&color=34d399" alt="MIT License" /></a>
  <a href="https://github.com/DreamerX00/LinVClipBoard/releases/latest"><img src="https://img.shields.io/github/downloads/DreamerX00/LinVClipBoard/total?style=flat-square&color=f59e0b&label=downloads" alt="Downloads" /></a>
  <img src="https://img.shields.io/badge/rust-2021-orange?style=flat-square&logo=rust" alt="Rust" />
  <img src="https://img.shields.io/badge/tauri-v2-24C8D8?style=flat-square&logo=tauri" alt="Tauri v2" />
</p>

<p align="center">
  <a href="#-quick-install">Install</a> &bull;
  <a href="#-features">Features</a> &bull;
  <a href="#%EF%B8%8F-usage">Usage</a> &bull;
  <a href="#-configuration">Config</a> &bull;
  <a href="https://github.com/DreamerX00/LinVClipBoard/releases">Releases</a>
</p>

---

## Why LinVClipBoard?

Windows has `Win+V`. Mac has clipboard history. Linux had…nothing great. Until now.

LinVClipBoard is a **native**, **lightweight** clipboard platform that gives you everything the other OSes have — and more. GIF search, emoji picker, symbol tables, full-text search across your entire clipboard history, all wrapped in a gorgeous glassmorphism overlay activated with a single keystroke.

Built in **Rust + Tauri v2**. Runs as a systemd user service. No Electron. No bloat.

---

## ✨ Features

| Feature | Description |
|:--------|:------------|
| 📋 **Clipboard History** | Every text & image you copy, searchable with SQLite FTS5 |
| 🎞️ **GIF Search** | Browse trending GIFs, search the KLIPY library, copy URL with one click |
| 😀 **Emoji Picker** | ~1,800 emojis across 9 categories with recent-used tracking |
| ∑ **Symbol Table** | Math, arrows, currency, Greek, superscripts, box drawing |
| 🔍 **Full-text Search** | Instant FTS5 search across your entire history |
| 📌 **Pin & Organize** | Pin important items so they never expire |
| 🎨 **Themes** | Dark, Light, or Auto (follows OS). Glassmorphism everywhere |
| 🌐 **4 Languages** | English, Português, 日本語, हिन्दी — easily extensible |
| 🔄 **Auto Updates** | Weekly update check with desktop notification. Manual check in Settings |
| 🖱️ **Draggable Window** | Grab the title bar and move the overlay anywhere |
| 🔍 **Zoom** | Scale the entire UI from 50% to 200% |
| ⌨️ **Keyboard-first** | Arrow keys, Enter to paste, Escape to dismiss, Ctrl+/−/0 for zoom |
| 🔒 **Secure** | Incognito mode, app blacklist, auto-expiry, memory limits |
| 🖥️ **X11 + Wayland** | Native clipboard access via arboard — no hacks |

---

## 🚀 Quick Install

### One line (Debian/Ubuntu)

```bash
curl -fsSL https://raw.githubusercontent.com/DreamerX00/LinVClipBoard/main/install.sh | bash
```

Alternate CDN (if the above 404s from a stale cache):

```bash
curl -fsSL https://cdn.jsdelivr.net/gh/DreamerX00/LinVClipBoard@main/install.sh | bash
```

That downloads the latest `.deb` from
[Releases](https://github.com/DreamerX00/LinVClipBoard/releases/latest),
verifies its checksum, and installs it (dependencies resolve automatically).
The daemon starts on its own — press **`Ctrl+/`** to open the overlay.

> The package includes everything: `clipd` (daemon), `clipctl` (CLI), `linvclip-ui` (overlay), systemd service + update timer, desktop entry, man pages, and icon.

### Manual install

Grab the files for your distro from
[Releases](https://github.com/DreamerX00/LinVClipBoard/releases/latest)
(the installer above does this for you on Debian/Ubuntu).

### Debian/Ubuntu (.deb)

```bash
# Download linvclipboard_3.3.2-1_amd64.deb and SHA256SUMS, then verify:
sha256sum --ignore-missing -c SHA256SUMS

# Install (apt resolves dependencies automatically):
sudo apt install ./linvclipboard_3.3.2-1_amd64.deb
```

That's it. The daemon starts automatically. Press **`Ctrl+/`** to open the overlay.

> The package includes everything: `clipd` (daemon), `clipctl` (CLI), `linvclip-ui` (overlay), systemd service + update timer, desktop entry, man pages, and icon.

### Fedora/RHEL (.rpm)

```bash
sha256sum --ignore-missing -c SHA256SUMS
sudo dnf install ./linvclipboard-3.3.2-1.x86_64.rpm
```

### Other distros (tarball)

```bash
sha256sum --ignore-missing -c SHA256SUMS
tar xzf linvclipboard-3.3.2-linux-x86_64.tar.gz
cd linvclipboard-3.3.2-linux-x86_64
./install-user.sh   # installs to ~/.local, no root needed
```

### Windows

Download `LinVClipBoard_3.3.2_x64-setup.exe` (and `SHA256SUMS` to verify it)
from [Releases](https://github.com/DreamerX00/LinVClipBoard/releases/latest)
and run it. It installs for the current user (no admin rights needed), bundles
the `clipd` daemon and offers to start it at login.

```powershell
# Optional: verify the download before running it
(Get-FileHash .\LinVClipBoard_3.3.2_x64-setup.exe -Algorithm SHA256).Hash.ToLower()
Select-String LinVClipBoard_3.3.2_x64-setup.exe .\SHA256SUMS
```

Later versions install themselves: **Settings → Check for Updates** downloads
the signed installer, verifies it and relaunches the app on the new version.
The `winget`/`scoop`/`chocolatey` manifests in `windows/publish/` are not
submitted to those repositories yet.

### Build from source

```bash
# Prerequisites (Ubuntu/Debian)
sudo apt install -y build-essential pkg-config libsqlite3-dev \
    libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
    libwayland-dev wl-clipboard \
    libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev

# Clone & build
git clone https://github.com/DreamerX00/LinVClipBoard.git
cd LinVClipBoard
make deb
sudo dpkg -i target/debian/linvclipboard_*_amd64.deb
```

---

## 🏗️ Architecture

```
  ┌─────────────────┐     ┌─────────────┐
  │  linvclip-ui    │     │   clipctl   │
  │  (Tauri v2)     │     │   (CLI)     │
  └────────┬────────┘     └──────┬──────┘
           │                     │
           └──────┬──────────────┘
                  │
        Unix Domain Socket (IPC)
                  │
           ┌──────┴──────┐
           │    clipd     │
           │   (daemon)   │
           └──────┬──────┘
                  │
     ┌────────────┼────────────┐
     │            │            │
  SQLite       FTS5        arboard
  + blobs    full-text    X11/Wayland
              search      clipboard
```

| Crate | Role |
|:------|:-----|
| **`clipd`** | Background daemon — captures clipboard changes, enforces limits, serves IPC |
| **`clipctl`** | CLI tool — list, search, paste, pin, delete, status |
| **`linvclip-ui`** | Tauri v2 overlay — the full GUI experience |
| **`shared`** | Library — database, IPC protocol, config, models |

---

## ⌨️ Usage

### Overlay UI

Press **`Ctrl+/`** (or your custom shortcut) to summon the overlay.

| Key | Action |
|:----|:-------|
| `↑` `↓` | Navigate items |
| `Enter` | Copy selected item to clipboard |
| `Delete` | Remove selected item |
| `Escape` | Dismiss overlay |
| `Ctrl` + `+` / `-` / `0` | Zoom in / out / reset |

**Tabs:** Clipboard • Emojis • Symbols • GIFs

Just start typing to search — the search bar auto-focuses.

### CLI

```bash
clipctl list                  # Recent items
clipctl list --limit 50       # Last 50
clipctl search "hello"        # Full-text search
clipctl paste <id>            # Copy item back to clipboard
clipctl pin <id>              # Pin / unpin
clipctl delete <id>           # Delete
clipctl clear                 # Clear all non-pinned
clipctl status                # Daemon info
```

### Daemon

```bash
systemctl --user status clipd           # Status
systemctl --user restart clipd          # Restart
journalctl --user -u clipd -f           # Live logs
systemctl --user status linvclip-update-check.timer  # Update timer
```

---

## ⚙️ Configuration

Auto-created at `~/.config/linvclip/config.toml` on first run.

```toml
[daemon]
poll_interval_ms = 250
log_level = "info"                # trace | debug | info | warn | error

[storage]
max_items = 10000
max_item_size_bytes = 52428800    # 50 MB
expiry_days = 30

[security]
blacklisted_apps = ["keepassxc", "1password", "bitwarden"]
incognito = false                 # true = pause all capture

[ui]
theme = "auto"                    # auto | dark | light
language = "en"                   # en | pt | ja | hi
zoom = 100                        # 50–200
window_position = "mouse"         # mouse | fixed

[gif]                             # optional — defaults come from gif-provider.json
# api_key = "your-own-klipy-key"  # use your own KLIPY app key
# base_url = "https://api.klipy.com/api/v1"
```

GIF search uses the API key from [`gif-provider.json`](gif-provider.json) in
this repository, downloaded at runtime and cached for 6 hours. Rotating the
key is a one-line edit to that file; installed apps pick it up on their next
GIF request, no update needed.

---

## 🌐 Adding a Language

1. Copy `crates/linvclip-ui/src/i18n/en.json` → `fr.json`
2. Translate all values (keep keys unchanged)
3. Import in `crates/linvclip-ui/src/i18n/index.jsx` and add to the `TRANSLATIONS` map
4. It appears in Settings automatically

---

## 🗑️ Uninstall

```bash
sudo apt remove linvclipboard
systemctl --user disable --now clipd.service linvclip-update-check.timer

# Optional: remove user data
rm -rf ~/.config/linvclip ~/.local/share/linvclip
```

---

## 📄 License

[MIT](LICENSE) — Built with ❤️ by **DreamerX**
