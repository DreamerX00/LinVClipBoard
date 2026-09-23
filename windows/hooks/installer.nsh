; NSIS installer hooks for LinVClipBoard
; Used by Tauri v2 NSIS bundler (bundle.windows.nsis.installerHooks).
;
; Tauri only inserts these four macros: NSIS_HOOK_PREINSTALL,
; NSIS_HOOK_POSTINSTALL, NSIS_HOOK_PREUNINSTALL, NSIS_HOOK_POSTUNINSTALL.
; (The electron-builder names — customInstall etc. — are silently ignored.)
;
; $UpdateMode / $PassiveMode are set by the template from the /UPDATE and /P
; flags that tauri-plugin-updater passes when it runs this installer, so an
; in-app update never shows a prompt and never touches user data.

!define CLIPD_AUTOSTART_KEY "Software\Microsoft\Windows\CurrentVersion\Run"
!define CLIPD_AUTOSTART_NAME "LinVClipBoard"

; Stop the clipboard daemon. Tauri's own running-app check only covers
; linvclip-ui.exe; clipd.exe is a bundled resource that keeps running after
; the UI exits, and a locked clipd.exe makes the File copy (or Delete) fail.
!macro StopClipd
  DetailPrint "Stopping clipd.exe"
  nsExec::ExecToLog 'taskkill /F /IM clipd.exe /T'
  Pop $0
  nsExec::ExecToLog 'taskkill /F /IM clipctl.exe /T'
  Pop $0
  Sleep 500
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro StopClipd
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; clipd data directory
  CreateDirectory "$APPDATA\LinVClipBoard"

  IfFileExists "$INSTDIR\clipd.exe" +2
    DetailPrint "Warning: clipd.exe not found in resources"

  ; Autostart clipd at login.
  ;  - An existing entry is rewritten so it follows $INSTDIR across updates.
  ;  - In-app updates (/UPDATE) never add one the user did not have.
  ;  - Fresh installs ask; silent/passive installs default to yes.
  ClearErrors
  ReadRegStr $0 HKCU "${CLIPD_AUTOSTART_KEY}" "${CLIPD_AUTOSTART_NAME}"
  ${If} ${Errors}
    ${If} $UpdateMode <> 1
      MessageBox MB_YESNO|MB_ICONQUESTION "Launch the LinVClipBoard clipboard daemon at startup?" /SD IDYES IDNO +2
        WriteRegStr HKCU "${CLIPD_AUTOSTART_KEY}" "${CLIPD_AUTOSTART_NAME}" '"$INSTDIR\clipd.exe"'
    ${EndIf}
  ${Else}
    WriteRegStr HKCU "${CLIPD_AUTOSTART_KEY}" "${CLIPD_AUTOSTART_NAME}" '"$INSTDIR\clipd.exe"'
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro StopClipd
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Never on an update (/UPDATE): autostart and user data survive it.
  ${If} $UpdateMode <> 1
    DeleteRegValue HKCU "${CLIPD_AUTOSTART_KEY}" "${CLIPD_AUTOSTART_NAME}"
    ; The template's "Delete the application data" checkbox only covers
    ; $APPDATA\<bundle id>; clipd keeps its history and settings here.
    ${If} $DeleteAppDataCheckboxState = 1
      RMDir /r "$APPDATA\LinVClipBoard"
    ${EndIf}
  ${EndIf}
!macroend
