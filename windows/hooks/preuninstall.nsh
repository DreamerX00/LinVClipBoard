; NSIS pre-uninstall hook — runs before file deletion
; Clean up per-user data (prompt user first via Tauri uninstaller dialog)

; Remove HKCU Run entry
DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "LinVClipBoard"

; User data is kept by default — uncomment to prompt removal:
; MessageBox MB_YESNO "Remove all clipboard history and settings?" IDNO skip_data
; RmDir /r "$APPDATA\LinVClipBoard"
; skip_data:
