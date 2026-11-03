; Instalador do Eavely (NSIS). Por usuario (sem admin), atalhos opcionais, desinstalacao limpa.
; Dados em %APPDATA%\Eavely so sao apagados se o usuario pedir (caixa na desinstalacao ou /DELETEDATA no modo silencioso).
; Compilar: makensis /DVERSION=0.1.0 /DEXE=caminho\eavely.exe /DOUT=caminho\Eavely-Setup.exe installer\eavely.nsi
Unicode true
!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "LogicLib.nsh"
!include "Sections.nsh"

!ifndef VERSION
  !define VERSION "0.1.0"
!endif
!ifndef EXE
  !define EXE "..\app\target\release\eavely.exe"
!endif
!ifndef OUT
  !define OUT "Eavely-Setup.exe"
!endif

Name "Eavely"
OutFile "${OUT}"
InstallDir "$LOCALAPPDATA\Programs\Eavely"
InstallDirRegKey HKCU "Software\Eavely" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
BrandingText "Eavely ${VERSION}"

!define MUI_ICON "..\assets\app\eavely-icon.ico"
!define MUI_UNICON "..\assets\app\eavely-icon.ico"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\Eavely.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Iniciar o Eavely agora"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\LICENSE"
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "PortugueseBR"

VIProductVersion "${VERSION}.0"
VIAddVersionKey /LANG=${LANG_PORTUGUESEBR} "ProductName" "Eavely"
VIAddVersionKey /LANG=${LANG_PORTUGUESEBR} "FileDescription" "Instalador do Eavely"
VIAddVersionKey /LANG=${LANG_PORTUGUESEBR} "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=${LANG_PORTUGUESEBR} "LegalCopyright" "MIT"

Section "Eavely (obrigatorio)" SecMain
  SectionIn RO
  SetOutPath "$INSTDIR"
  nsExec::Exec 'taskkill /IM Eavely.exe'      ; 1o pedido gracioso (WM_CLOSE): o app salva e remove o icone da bandeja
  Sleep 1500
  nsExec::Exec 'taskkill /F /IM Eavely.exe'   ; so forca se ainda estiver vivo (ignora erro)
  Sleep 300
  File "/oname=Eavely.exe" "${EXE}"
  File "..\LICENSE"
  WriteUninstaller "$INSTDIR\Desinstalar.exe"
  WriteRegStr HKCU "Software\Eavely" "InstallDir" "$INSTDIR"
  ; Adicionar ou remover programas (por usuario)
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Eavely" "DisplayName" "Eavely"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Eavely" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Eavely" "Publisher" "Eavely"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Eavely" "DisplayIcon" "$INSTDIR\Eavely.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Eavely" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Eavely" "UninstallString" '"$INSTDIR\Desinstalar.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Eavely" "QuietUninstallString" '"$INSTDIR\Desinstalar.exe" /S'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Eavely" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Eavely" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Eavely" "EstimatedSize" "$0"
SectionEnd

; Atalhos: o desinstalador so apaga o que o proprio instalador criou (gravado em HKCU\Software\Eavely).
; Se ja existia um Eavely.lnk, ele nao e tocado nem registrado.
Section "Atalho no Menu Iniciar" SecStart
  ${IfNot} ${FileExists} "$SMPROGRAMS\Eavely.lnk"
    CreateShortcut "$SMPROGRAMS\Eavely.lnk" "$INSTDIR\Eavely.exe"
    WriteRegDWORD HKCU "Software\Eavely" "CreatedStartLnk" 1
  ${EndIf}
SectionEnd

Section /o "Atalho na Area de Trabalho" SecDesk
  ${IfNot} ${FileExists} "$DESKTOP\Eavely.lnk"
    CreateShortcut "$DESKTOP\Eavely.lnk" "$INSTDIR\Eavely.exe"
    WriteRegDWORD HKCU "Software\Eavely" "CreatedDesktopLnk" 1
  ${EndIf}
SectionEnd

Section /o "Iniciar com o Windows" SecRun
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Eavely" '"$INSTDIR\Eavely.exe"'
SectionEnd

LangString DESC_Main ${LANG_PORTUGUESEBR} "O aplicativo (fica na bandeja do sistema)."
LangString DESC_Start ${LANG_PORTUGUESEBR} "Cria um atalho no Menu Iniciar."
LangString DESC_Desk ${LANG_PORTUGUESEBR} "Cria um atalho na Area de Trabalho (opcional)."
LangString DESC_Run ${LANG_PORTUGUESEBR} "Abre o Eavely na bandeja ao entrar no Windows (opcional)."
!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SecMain} $(DESC_Main)
  !insertmacro MUI_DESCRIPTION_TEXT ${SecStart} $(DESC_Start)
  !insertmacro MUI_DESCRIPTION_TEXT ${SecDesk} $(DESC_Desk)
  !insertmacro MUI_DESCRIPTION_TEXT ${SecRun} $(DESC_Run)
!insertmacro MUI_FUNCTION_DESCRIPTION_END

; modo silencioso: /DESKTOP e /AUTOSTART marcam os componentes opcionais
Function .onInit
  ${GetParameters} $R0
  ${GetOptions} $R0 "/DESKTOP" $R1
  ${IfNot} ${Errors}
    !insertmacro SelectSection ${SecDesk}
  ${EndIf}
  ClearErrors
  ${GetOptions} $R0 "/AUTOSTART" $R1
  ${IfNot} ${Errors}
    !insertmacro SelectSection ${SecRun}
  ${EndIf}
  ClearErrors
FunctionEnd

Section "Uninstall"
  nsExec::Exec 'taskkill /IM Eavely.exe'
  Sleep 1500
  nsExec::Exec 'taskkill /F /IM Eavely.exe'
  Sleep 300
  Delete "$INSTDIR\Eavely.exe"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\Desinstalar.exe"
  RMDir "$INSTDIR"
  ReadRegDWORD $R2 HKCU "Software\Eavely" "CreatedStartLnk"
  ${If} $R2 == 1
    Delete "$SMPROGRAMS\Eavely.lnk"
  ${EndIf}
  ReadRegDWORD $R2 HKCU "Software\Eavely" "CreatedDesktopLnk"
  ${If} $R2 == 1
    Delete "$DESKTOP\Eavely.lnk"
  ${EndIf}
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Eavely"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Eavely"
  DeleteRegKey HKCU "Software\Eavely"
  DeleteRegKey HKCU "Software\Classes\AppUserModelId\app.eavely"   ; AUMID dos toasts criado pelo app
  ; dados do usuario (lista do Shelf, ui.json, cache de letras): so se o usuario pedir
  ${GetParameters} $R0
  ${GetOptions} $R0 "/DELETEDATA" $R1
  ${IfNot} ${Errors}
    RMDir /r "$APPDATA\Eavely"
  ${ElseIf} ${Silent}
    ; desinstalacao silenciosa sem /DELETEDATA: preserva os dados
  ${Else}
    MessageBox MB_YESNO|MB_ICONQUESTION|MB_DEFBUTTON2 "Remover tambem seus dados (lista do Shelf, configuracoes e cache de letras) em $APPDATA\Eavely?" IDNO +2
    RMDir /r "$APPDATA\Eavely"
  ${EndIf}
SectionEnd
