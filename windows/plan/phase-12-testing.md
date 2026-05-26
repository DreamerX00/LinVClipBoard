# Phase 12: Windows Testing & Quality Assurance

**Version**: v3.0.0-test  
**Effort**: 3-4 days  
**Dependencies**: All phases 0-11 complete

---

## Objective

Systematic testing of LinVClipBoard on Windows 10 and Windows 11. This phase covers functionality testing, edge cases, performance, and bug fixes before release.

---

## Tasks

### 12.1 Test Environment Setup

- [ ] Windows 10 VM (version 22H2, build 19045)
- [ ] Windows 11 VM (version 24H2, build 26100)
- [ ] Clean install with no prior clipboard managers
- [ ] Install with competing clipboard manager (Ditto, CopyQ, PowerToys)
- [ ] Remote Desktop (RDP) session
- [ ] Elevated (Admin) session
- [ ] Non-English locale (Japanese, Portuguese, Hindi for i18n)

### 12.2 Core Functionality Tests

**Clipboard Monitoring:**
| Test | Expected | Status |
|------|----------|--------|
| Copy plain text | Appears in history | |
| Copy Unicode (emoji, CJK, Arabic) | Appears correctly | |
| Copy HTML from browser | Appears as rich content | |
| Copy image (screenshot, browser) | Appears as image thumbnail | |
| Copy files (Explorer, Ctrl+C) | File paths captured | |
| Copy multiple formats (text + image) | Both stored | |
| Rapid copies (10x in 1 second) | All captured (no dedup misses) | |
| Copy from elevated app (Run as Admin) | Still captured | |
| Copy from UWP app (Calculator, Settings) | Still captured | |

**Clipboard History:**
| Test | Expected | Status |
|------|----------|--------|
| List history | All items shown, newest first | |
| Search history | FTS5 search works | |
| Pin item | Item stays at top, not removed on overflow | |
| Delete item | Removed from list and DB | |
| Bulk delete | Multiple items removed | |
| Clear all history | All items removed | |

**Pasting:**
| Test | Expected | Status |
|------|----------|--------|
| Paste into Notepad | Text appears | |
| Paste into Chrome/Firefox/Edge | Text appears | |
| Paste into Windows Terminal | Text appears | |
| Paste into password field | Works (audit: not stored if blacklisted) | |
| Paste into elevated app (Admin) | Works via UI Automation | |
| Paste Unicode/emoji | Characters correct | |
| Paste image | Image pastes | |

### 12.3 GUI Tests

| Test | Expected | Status |
|------|----------|--------|
| Overlay opens via Ctrl+Shift+V | Overlay appears | |
| Overlay closes on Escape | Overlay disappears | |
| Overlay transparent areas | Click-through works | |
| System tray icon | Icon visible, right-click menu works | |
| Tray icon left-click | Toggles overlay | |
| App auto-hides on focus loss | Overlay closes | |
| Always-on-top behavior | Overlay stays above other windows | |
| Settings panel | All settings load and save | |
| Theme switching | Dark, Light, Catppuccin, Nord, Dracula | |
| Language switching | EN, PT, JA, HI | |
| Emoji picker | Emojis copy to clipboard | |

### 12.4 Autostart Tests

| Test | Expected | Status |
|------|----------|--------|
| Enable "Launch at startup" | HKCU registry key written | |
| Reboot | clipd.exe starts automatically | |
| GUI launches after clipd | Connects via named pipe | |
| Disable "Launch at startup" | Registry key removed | |
| Uninstall | Registry key removed | |

### 12.5 Input Simulation Tests

| Test | Expected | Status |
|------|----------|--------|
| Paste via Ctrl+V | Text pasted into foreground app | |
| Type text | Characters typed one by one | |
| Paste into browser address bar | Works | |
| Paste into terminal | Works (may need "paste as text" mode) | |
| Rapid successive pastes | No race conditions | |
| Keyboard state restoration | No stuck modifiers after paste | |
| Paste into RDP session | Works (may have limitations) | |

### 12.6 Edge Cases & Stress Tests

| Test | Expected | Status |
|------|----------|--------|
| 10,000 clipboard entries | DB handles large history | |
| 1GB+ total database | Performance remains acceptable | |
| Image clipboard (large screenshots) | Stored and displayed | |
| Unicode edge cases (RTL, combining chars) | Displayed correctly | |
| App blacklist (KeePassXC, 1Password, Bitwarden) | Not capturing sensitive data | |
| Incognito mode | Nothing stored during session | |
| Two instances of app | Single instance enforced | |
| Rapid open/close overlay (50x) | No memory leaks | |
| System sleep/resume | Daemon resumes monitoring | |
| WebView2 not installed | Installer downloads it | |

### 12.7 Performance Tests

| Metric | Target | Measured |
|--------|--------|----------|
| Memory (idle, no overlay) | < 30 MB | |
| Memory (overlay open) | < 80 MB | |
| Overlay open time | < 200 ms | |
| Overlay close time | < 100 ms | |
| Clipboard change → history update | < 100 ms | |
| Search response (10k items) | < 50 ms | |
| CPU (idle, no clipboard changes) | 0% | |
| CPU (during clipboard change) | < 5% spike | |
| NSIS installer size | < 15 MB | |

### 12.8 Security Tests

| Test | Expected | Status |
|------|----------|--------|
| Named pipe accessible only by current user | Other user cannot connect | |
| Named pipe accessible over network | Remote connection rejected | |
| Clipboard history DB is user-only | Only current user can read | |
| Password manager blacklist | Credentials not stored | |
| Update signature verification | Tampered update is rejected | |
| uiAccess manifest | Only works from Program Files | |

### 12.9 Upgrade Tests

| Test | Expected | Status |
|------|----------|--------|
| Install v1.0.0 (hypothetical) | Works | |
| Upgrade to v3.0.0 | History preserved | |
| Upgrade with DB schema change | Migration runs | |
| Upgrade while clipd is running | New clipd starts | |
| Rollback (downgrade) | Not supported (graceful message) | |

### 12.10 Cross-Platform Compatibility

- [ ] Verify no Linux regressions:
  - `cargo test --workspace` on Ubuntu — all pass
  - `cargo build --release` on Ubuntu — no warnings
  - `npx tauri build` on Ubuntu — produces .deb

### 12.11 Bug Tracking

Create a bug tracker checklist with discovered issues. Each bug gets:
- Severity: Critical / High / Medium / Low
- Status: Open / In Progress / Fixed / Verified
- Notes: Workaround, reproduction steps

**Template:**
```
## Bug #[N]: [Title]
- **Severity:** [Critical/High/Medium/Low]
- **Environment:** [Windows 10/11, build number]
- **Steps to reproduce:**
  1. ...
  2. ...
- **Expected:** ...
- **Actual:** ...
- **Workaround:** ...
- **Fixed in:** [commit/phase]
```

---

## Deliverables

1. Comprehensive test report for all test categories
2. Bug tracker with all discovered issues
3. Performance benchmark results
4. Verified working Windows build ready for release

---

## Acceptance Criteria

- [ ] All core functionality tests pass
- [ ] All GUI tests pass
- [ ] Performance metrics meet targets
- [ ] No critical or high-severity bugs remain
- [ ] Upgrade path tested (if applicable)
- [ ] Linux build has zero regressions
- [ ] Phase 12 branch committed
