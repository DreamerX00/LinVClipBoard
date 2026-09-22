; NSIS post-install hook — runs after file extraction but before finalization
; Called by Tauri bundler's NSIS template

; Create data directory
CreateDirectory "$APPDATA\LinVClipBoard"

; Optionally install clipd as a run-at-startup for all users
; (Currently handled per-user via HKCU Run in Phase 6)
