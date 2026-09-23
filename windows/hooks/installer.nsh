; NSIS installer hooks for LinVClipBoard (Tauri v2 NSIS bundler).
;
; Tauri only inserts the four NSIS_HOOK_* macros below — any other macro name
; is silently ignored. The main app (linvclip-ui.exe) is closed by Tauri's own
; template; these hooks take care of the clipboard daemon (clipd.exe), which
; Tauri knows nothing about and which would otherwise keep the old binary
; locked while the installer tries to overwrite it (auto-update included).

; Runs before any file is copied — for fresh installs and updates alike.
!macro NSIS_HOOK_PREINSTALL
  ; Remember whether this is a fresh install (no uninstall entry yet) so the
  ; post-install hook only touches the autostart setting the first time.
  Var /GLOBAL LinvFreshInstall
  StrCpy $LinvFreshInstall "1"
  ClearErrors
  ReadRegStr $0 SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCTNAME}" "DisplayVersion"
  ${IfNot} ${Errors}
    StrCpy $LinvFreshInstall "0"
  ${EndIf}

  ; Stop the daemon so clipd.exe can be replaced.
  DetailPrint "Stopping clipd.exe"
  nsExec::ExecToLog 'taskkill /F /IM clipd.exe'
  Pop $0
  Sleep 500
!macroend

; Runs after files, registry keys and shortcuts are in place.
!macro NSIS_HOOK_POSTINSTALL
  ; Fresh install: start the daemon at login so clipboard history is captured
  ; even before the UI is opened. Users can turn this off in Settings, and an
  ; update must not re-enable it — hence the fresh-install check.
  ${If} $LinvFreshInstall == "1"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "LinVClipBoard" '"$INSTDIR\clipd.exe"'
  ${EndIf}
!macroend

; Runs before the uninstaller removes anything.
!macro NSIS_HOOK_PREUNINSTALL
  DetailPrint "Stopping clipd.exe"
  nsExec::ExecToLog 'taskkill /F /IM clipd.exe'
  Pop $0
  Sleep 500
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "LinVClipBoard"
!macroend
