; Tauri CLI 2.12.1 includes this after utils.nsh and RestartManager.nsh.
; Replace its process-closing policy for BOTH install and uninstall.
; A gamma recovery watchdog must never be forcibly stopped by the installer.
!macroundef CheckIfAppIsRunning
!macro CheckIfAppIsRunning executablePath productName
  !insertmacro RestartManager_StartSession $R0
  ${If} $R0 == ""
    SetErrorLevel 1603
    Abort "Cannot check running processes. Close ${productName} and retry."
  ${EndIf}

  !insertmacro RestartManager_RegisterFile $R0 "${executablePath}"
  ${If} $0 <> 0
    !insertmacro RestartManager_EndSession $R0
    SetErrorLevel 1603
    Abort "Cannot check running processes. Close ${productName} and retry."
  ${EndIf}

  System::Call 'RSTRTMGR::RmGetList(p R0, *i .r1, *i .r2, p 0, *i .r3) i .r0'
  StrCpy $R4 $0
  !insertmacro RestartManager_EndSession $R0
  ${If} $R4 = ${ERROR_MORE_DATA}
    SetErrorLevel 1618
    IfSilent +2
      MessageBox MB_OK|MB_ICONEXCLAMATION "${productName} is running. Use Restore and quit from its tray menu, then retry. No processes have been stopped."
    Abort "Use Restore and quit before installing or uninstalling."
  ${ElseIf} $R4 <> 0
    SetErrorLevel 1603
    Abort "Cannot verify that ${productName} is closed. Installation cancelled."
  ${EndIf}
!macroend

; The app intentionally uses a stable Run value name without spaces.
; Preserve upgrades and registrations owned by a different/portable executable.
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "ScreenBrightController"
    ${If} $0 == '$\"$INSTDIR\${MAINBINARYNAME}.exe$\" --autostart'
      DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "ScreenBrightController"
    ${EndIf}
  ${EndIf}
!macroend
