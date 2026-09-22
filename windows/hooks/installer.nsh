; NSIS installer hooks for LinVClipBoard
; Used by Tauri v2 NSIS bundler

!macro preInit
  ; Check for previous install location
  ReadRegStr $INSTDIR HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\LinVClipBoard" \
    "InstallLocation"
  IfErrors +2
    StrCpy $INSTDIR $INSTDIR
!macroend

!macro customHeader
  !insertmacro MUI_HEADER_TEXT "LinVClipBoard Setup" \
    "A clipboard manager for Windows — inspired by Linux."
!macroend

!macro customInstall
  ; Create clipd data directory
  CreateDirectory "$APPDATA\LinVClipBoard"

  ; Ensure resources (clipd.exe, clipctl.exe) are present
  IfFileExists "$INSTDIR\clipd.exe" +2
    DetailPrint "Warning: clipd.exe not found in resources"

  ; Create desktop shortcut
  CreateShortCut "$DESKTOP\LinVClipBoard.lnk" "$INSTDIR\linvclip-ui.exe" \
    "" "$INSTDIR\linvclip-ui.exe" 0

  ; Create Start Menu entries
  CreateDirectory "$SMPROGRAMS\LinVClipBoard"
  CreateShortCut "$SMPROGRAMS\LinVClipBoard\LinVClipBoard.lnk" \
    "$INSTDIR\linvclip-ui.exe" "" "$INSTDIR\linvclip-ui.exe" 0
  CreateShortCut "$SMPROGRAMS\LinVClipBoard\Uninstall LinVClipBoard.lnk" \
    "$INSTDIR\Uninstall.exe" "" "$INSTDIR\Uninstall.exe" 0

  ; Register autostart (user chooses during install)
  ${If} ${Cmd} `MessageBox MB_YESNO "Launch clipd at startup?" /SD IDYES IDYES`
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" \
      "LinVClipBoard" "$INSTDIR\clipd.exe"
  ${EndIf}
!macroend

!macro customUnInstall
  ; Stop clipd if running
  nsExec::Exec '"$INSTDIR\clipd.exe" --shutdown'

  ; Remove autostart registry key
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" \
    "LinVClipBoard"

  ; Remove desktop shortcut
  Delete "$DESKTOP\LinVClipBoard.lnk"

  ; Remove Start Menu entries
  RMDir /r "$SMPROGRAMS\LinVClipBoard"

  ; Remove data directory (user data)
  MessageBox MB_YESNO "Remove clipboard history and settings?" /SD IDNO IDNO +2
    RMDir /r "$APPDATA\LinVClipBoard"
!macroend
