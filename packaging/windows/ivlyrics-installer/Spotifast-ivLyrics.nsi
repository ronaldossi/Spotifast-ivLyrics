Unicode true
!include "MUI2.nsh"
!include "x64.nsh"
!include "WinVer.nsh"

!ifndef PAYLOAD
  !error "Pass /DPAYLOAD=<unpacked release directory>"
!endif
!ifndef OUTPUT
  !error "Pass /DOUTPUT=<installer path>"
!endif
!define PRODUCT "Spotifast · ivLyrics"
!define KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\Spotifast-ivLyrics"
Name "${PRODUCT}"
OutFile "${OUTPUT}"
InstallDir "$LOCALAPPDATA\Programs\Spotifast-ivLyrics"
InstallDirRegKey HKCU "${KEY}" "InstallLocation"
RequestExecutionLevel user
SetCompressor /SOLID lzma
SetOverwrite on
ShowInstDetails show
ShowUninstDetails show
VIProductVersion "0.12.0.2"
VIAddVersionKey "ProductName" "${PRODUCT}"
VIAddVersionKey "FileDescription" "Spotifast ivLyrics Windows x64 Setup"
VIAddVersionKey "FileVersion" "0.12.0.2"
VIAddVersionKey "LegalCopyright" "Open-source notices included in the installation"
Icon "${PAYLOAD}\spotifast.ico"
UninstallIcon "${PAYLOAD}\spotifast.ico"
!define MUI_ABORTWARNING
!define MUI_WELCOMEPAGE_TEXT "Spotifast · ivLyrics를 설치합니다.$\r$\n$\r$\n설치 전에 실행 중인 Spotifast를 트레이에서도 완전히 종료해 주세요.$\r$\n$\r$\n사용자 계정에 설치하며 관리자 권한은 필요하지 않습니다."
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH
!insertmacro MUI_LANGUAGE "Korean"
!insertmacro MUI_LANGUAGE "English"

Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_ICONSTOP "Windows 10/11 64비트가 필요합니다."
    Abort
  ${EndIf}
  ${IfNot} ${AtLeastWin10}
    MessageBox MB_ICONSTOP "Windows 10 이상이 필요합니다."
    Abort
  ${EndIf}
  SetShellVarContext current
FunctionEnd

Section "Application"
  SetOutPath "$INSTDIR"
  File /r "${PAYLOAD}\*"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateDirectory "$SMPROGRAMS\Spotifast-ivLyrics"
  CreateShortcut "$SMPROGRAMS\Spotifast-ivLyrics\Spotifast ivLyrics.lnk" "$INSTDIR\spotifast.exe" "" "$INSTDIR\spotifast.ico"
  CreateShortcut "$SMPROGRAMS\Spotifast-ivLyrics\사용 안내.lnk" "$INSTDIR\README.html"
  WriteRegStr HKCU "${KEY}" "DisplayName" "${PRODUCT}"
  WriteRegStr HKCU "${KEY}" "DisplayVersion" "0.12.0-ivlyrics.2"
  WriteRegStr HKCU "${KEY}" "Publisher" "ronaldossi"
  WriteRegStr HKCU "${KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${KEY}" "DisplayIcon" "$INSTDIR\spotifast.ico"
  WriteRegStr HKCU "${KEY}" "UninstallString" '$\"$INSTDIR\Uninstall.exe$\"'
  WriteRegStr HKCU "${KEY}" "QuietUninstallString" '$\"$INSTDIR\Uninstall.exe$\" /S'
  WriteRegStr HKCU "${KEY}" "URLInfoAbout" "https://github.com/ronaldossi/Spotifast-ivLyrics"
  WriteRegDWORD HKCU "${KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${KEY}" "NoRepair" 1
SectionEnd

Section "Uninstall"
  SetShellVarContext current
  ; Delete only packaged files, preserving user-created files and app settings.
  !include "uninstall-files.nsh"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\Spotifast-ivLyrics\Spotifast ivLyrics.lnk"
  Delete "$SMPROGRAMS\Spotifast-ivLyrics\사용 안내.lnk"
  RMDir "$SMPROGRAMS\Spotifast-ivLyrics"
  DeleteRegKey HKCU "${KEY}"
SectionEnd
