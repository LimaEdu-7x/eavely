; Instalador do canal estavel ou dev (NSIS). Por usuario (sem admin), atalhos opcionais, desinstalacao limpa.
; Dados em %APPDATA%\${APPDIR} (Eavely ou EavelyDev) so sao apagados se o usuario pedir.
; Compilar: makensis /DVERSION=0.3.0 /DNUMVER=0.3.0.8 /DEXE=caminho\eavely.exe /DOUT=saida.exe [/DDEV] installer\eavely.nsinsi
Unicode true
!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "LogicLib.nsh"
!include "Sections.nsh"

; canal: makensis /DDEV ... gera o instalador Eavely Dev (nomes, pastas, atalhos e icone proprios; convive com a estavel)
!ifdef DEV
  !define APPNAME "Eavely Dev"
  !define APPDIR "EavelyDev"
  !define EXE_NAME "EavelyDev.exe"
  !define APPICON "..\assets\app\eavely-dev-icon.ico"
!else
  !define APPNAME "Eavely"
  !define APPDIR "Eavely"
  !define EXE_NAME "Eavely.exe"
  !define APPICON "..\assets\app\eavely-icon.ico"
!endif
!ifndef VERSION
  !define VERSION "0.1.0"
!endif
!ifndef EXE
  !define EXE "..\app\target\release\eavely.exe"
!endif
!ifndef OUT
  !define OUT "${APPNAME}-Setup.exe"
!endif

Name "${APPNAME}"
OutFile "${OUT}"
InstallDir "$LOCALAPPDATA\Programs\${APPDIR}"
InstallDirRegKey HKCU "Software\${APPDIR}" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
BrandingText "${APPNAME} ${VERSION}"

!define MUI_ICON "..\assets\app\eavely-icon.ico"
!define MUI_UNICON "..\assets\app\eavely-icon.ico"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\${EXE_NAME}"
!define MUI_FINISHPAGE_RUN_TEXT "Iniciar o ${APPNAME} agora"
!define MUI_FINISHPAGE_LINK "Politica de privacidade e de assinatura de codigo"
!define MUI_FINISHPAGE_LINK_LOCATION "https://github.com/LimaEdu-7x/eavely/blob/main/docs/PRIVACY.md"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\LICENSE"
!define MUI_PAGE_HEADER_TEXT "Privacidade"
!define MUI_PAGE_HEADER_SUBTEXT "Como o ${APPNAME} trata seus dados"
!define MUI_LICENSEPAGE_TEXT_TOP "Leia o resumo abaixo (use a barra de rolagem)."
!define MUI_LICENSEPAGE_TEXT_BOTTOM "A busca de letras e opcional e pode ser desligada nas Configuracoes depois da instalacao."
!define MUI_LICENSEPAGE_BUTTON "&Continuar"
!insertmacro MUI_PAGE_LICENSE "privacy.pt-BR.txt"
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "PortugueseBR"

!ifndef NUMVER
  !define NUMVER "${VERSION}.0"
!endif
VIProductVersion "${NUMVER}"
VIAddVersionKey /LANG=${LANG_PORTUGUESEBR} "ProductName" "${APPNAME}"
VIAddVersionKey /LANG=${LANG_PORTUGUESEBR} "FileDescription" "Instalador do ${APPNAME}"
VIAddVersionKey /LANG=${LANG_PORTUGUESEBR} "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=${LANG_PORTUGUESEBR} "LegalCopyright" "MIT"

Section "${APPNAME} (obrigatorio)" SecMain
  SectionIn RO
  SetOutPath "$INSTDIR"
  nsExec::Exec 'taskkill /IM ${EXE_NAME}'      ; 1o pedido gracioso (WM_CLOSE): o app salva e remove o icone da bandeja
  Sleep 1500
  nsExec::Exec 'taskkill /F /IM ${EXE_NAME}'   ; so forca se ainda estiver vivo (ignora erro)
  Sleep 300
  File "/oname=${EXE_NAME}" "${EXE}"
  File "..\LICENSE"
  WriteUninstaller "$INSTDIR\Desinstalar.exe"
  WriteRegStr HKCU "Software\${APPDIR}" "InstallDir" "$INSTDIR"
  ; Adicionar ou remover programas (por usuario)
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPDIR}" "DisplayName" "${APPNAME}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPDIR}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPDIR}" "Publisher" "${APPNAME}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPDIR}" "DisplayIcon" "$INSTDIR\${EXE_NAME}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPDIR}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPDIR}" "UninstallString" '"$INSTDIR\Desinstalar.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPDIR}" "QuietUninstallString" '"$INSTDIR\Desinstalar.exe" /S'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPDIR}" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPDIR}" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPDIR}" "EstimatedSize" "$0"
SectionEnd

; Atalhos: o desinstalador so apaga o que o proprio instalador criou (gravado em HKCU\Software\${APPDIR}).
; Se ja existia um ${APPNAME}.lnk, ele nao e tocado nem registrado.
Section "Atalho no Menu Iniciar" SecStart
  ${IfNot} ${FileExists} "$SMPROGRAMS\${APPNAME}.lnk"
    CreateShortcut "$SMPROGRAMS\${APPNAME}.lnk" "$INSTDIR\${EXE_NAME}"
    WriteRegDWORD HKCU "Software\${APPDIR}" "CreatedStartLnk" 1
  ${EndIf}
SectionEnd

Section /o "Atalho na Area de Trabalho" SecDesk
  ${IfNot} ${FileExists} "$DESKTOP\${APPNAME}.lnk"
    CreateShortcut "$DESKTOP\${APPNAME}.lnk" "$INSTDIR\${EXE_NAME}"
    WriteRegDWORD HKCU "Software\${APPDIR}" "CreatedDesktopLnk" 1
  ${EndIf}
SectionEnd

Section /o "Iniciar com o Windows" SecRun
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${APPNAME}" '"$INSTDIR\${EXE_NAME}"'
SectionEnd

LangString DESC_Main ${LANG_PORTUGUESEBR} "O aplicativo (fica na bandeja do sistema)."
LangString DESC_Start ${LANG_PORTUGUESEBR} "Cria um atalho no Menu Iniciar."
LangString DESC_Desk ${LANG_PORTUGUESEBR} "Cria um atalho na Area de Trabalho (opcional)."
LangString DESC_Run ${LANG_PORTUGUESEBR} "Abre o ${APPNAME} na bandeja ao entrar no Windows (opcional)."
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
  nsExec::Exec 'taskkill /IM ${EXE_NAME}'
  Sleep 1500
  nsExec::Exec 'taskkill /F /IM ${EXE_NAME}'
  Sleep 300
  Delete "$INSTDIR\${EXE_NAME}"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\Desinstalar.exe"
  RMDir "$INSTDIR"
  ReadRegDWORD $R2 HKCU "Software\${APPDIR}" "CreatedStartLnk"
  ${If} $R2 == 1
    Delete "$SMPROGRAMS\${APPNAME}.lnk"
  ${EndIf}
  ReadRegDWORD $R2 HKCU "Software\${APPDIR}" "CreatedDesktopLnk"
  ${If} $R2 == 1
    Delete "$DESKTOP\${APPNAME}.lnk"
  ${EndIf}
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${APPNAME}"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPDIR}"
  DeleteRegKey HKCU "Software\${APPDIR}"
  DeleteRegKey HKCU "Software\Classes\AppUserModelId\app.eavely"   ; AUMID dos toasts criado pelo app
  ; dados do usuario (lista do Shelf, ui.json, cache de letras): so se o usuario pedir
  ${GetParameters} $R0
  ${GetOptions} $R0 "/DELETEDATA" $R1
  ${IfNot} ${Errors}
    RMDir /r "$APPDATA\${APPNAME}"
  ${ElseIf} ${Silent}
    ; desinstalacao silenciosa sem /DELETEDATA: preserva os dados
  ${Else}
    MessageBox MB_YESNO|MB_ICONQUESTION|MB_DEFBUTTON2 "Remover tambem seus dados (lista do Shelf, configuracoes e cache de letras) em $APPDATA\${APPNAME}?" IDNO +2
    RMDir /r "$APPDATA\${APPNAME}"
  ${EndIf}
SectionEnd
