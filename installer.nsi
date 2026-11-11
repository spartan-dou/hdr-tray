; Installeur HDR Tray : installation par utilisateur, sans droits administrateur.
; En mode silencieux (/S, utilisé par la mise à jour auto), l'appli est relancée à la fin.
Unicode true
!include "MUI2.nsh"

!include "x64.nsh"

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!ifndef ARCH
  !define ARCH "x64"
!endif
!ifndef SUFFIX
  !define SUFFIX ""
!endif

Name "HDR Tray"
OutFile "dist/HDRTray-Setup${SUFFIX}.exe"
InstallDir "$LOCALAPPDATA\Programs\HDRTray"
RequestExecutionLevel user
SetCompressor /SOLID lzma

!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\HDRTray"
!define RUN_KEY "Software\Microsoft\Windows\CurrentVersion\Run"

!define MUI_ICON "dist/hdr.ico"
!define MUI_UNICON "dist/hdr.ico"
!define MUI_FINISHPAGE_RUN "$INSTDIR\HDRTray.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Lancer HDR Tray maintenant"

!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "French"

!macro StopRunning
  IfFileExists "$INSTDIR\HDRTray.exe" 0 +2
    ExecWait '"$INSTDIR\HDRTray.exe" --quit'
  Sleep 500
  ; Arrête aussi une ancienne version (.NET) éventuellement lancée
  nsExec::Exec 'taskkill /F /IM HDRTray.exe'
  Pop $0
!macroend

Function .onInit
  ; La version x64 tourne aussi sur ARM64 (émulation) ; l'inverse est impossible
  !if "${ARCH}" == "arm64"
    ${IfNot} ${IsNativeARM64}
      MessageBox MB_ICONSTOP "Cet installeur est pour Windows ARM64.$\nPrenez HDRTray-Setup.exe pour un PC Intel/AMD." /SD IDOK
      Abort
    ${EndIf}
  !endif
FunctionEnd

Section
  !insertmacro StopRunning
  SetOutPath "$INSTDIR"
  File "/oname=HDRTray.exe" "dist/HDRTray${SUFFIX}.exe"
  WriteUninstaller "$INSTDIR\uninstall.exe"

  CreateShortcut "$SMPROGRAMS\HDR Tray.lnk" "$INSTDIR\HDRTray.exe"
  WriteRegStr HKCU "${RUN_KEY}" "HDRTray" '"$INSTDIR\HDRTray.exe"'

  WriteRegStr HKCU "${UNINST_KEY}" "DisplayName" "HDR Tray"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINST_KEY}" "Publisher" "spartan-dou"
  WriteRegStr HKCU "${UNINST_KEY}" "URLInfoAbout" "https://github.com/spartan-dou/hdr-tray"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\HDRTray.exe"
  WriteRegStr HKCU "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoRepair" 1

  IfSilent 0 +2
    Exec '"$INSTDIR\HDRTray.exe"'
SectionEnd

Section "Uninstall"
  !insertmacro StopRunning
  Delete "$INSTDIR\HDRTray.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\HDR Tray.lnk"
  DeleteRegValue HKCU "${RUN_KEY}" "HDRTray"
  DeleteRegKey HKCU "${UNINST_KEY}"
SectionEnd
