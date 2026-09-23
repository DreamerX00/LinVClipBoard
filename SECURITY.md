# Security Policy

## Supported versions

Only the latest release (currently 3.3.2) receives security fixes.

## Reporting a vulnerability

**Do not open a public issue.** Use GitHub's
[private vulnerability reporting](https://github.com/DreamerX00/LinVClipBoard/security/advisories/new)
so a fix can land before disclosure.

Include: affected version, steps to reproduce, and impact. Expect an
acknowledgement within 7 days.

## Scope notes for reviewers

- Privilege boundary: `linvclip-ui` never runs package installs itself; it
  delegates to the installed `apply-update` helper under polkit.
- The daemon's IPC socket is created mode `0700`; clipboard contents are
  stored in plaintext SQLite — do not copy secrets into the clipboard while
  incognito mode is off.
- Auto-update metadata is accepted only over HTTPS from the project's own
  release channels.
