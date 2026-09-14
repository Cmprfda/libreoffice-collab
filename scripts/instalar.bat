@echo off
REM ============================================================================
REM  LibreOffice Collab - Instalador de um clique / One-click installer
REM
REM  PT: Faca duplo clique neste ficheiro. Nao e preciso escrever nada.
REM  EN: Just double-click this file. Nothing to type.
REM
REM  Este ficheiro apenas prepara o ambiente e entrega o trabalho ao
REM  install.ps1, que faz o download e a instalacao.
REM  This file only prepares the environment and hands over to install.ps1,
REM  which performs the download and the installation.
REM
REM  Sem acentos de proposito: as consolas do Windows usam codepages
REM  diferentes e o texto sairia corrompido.
REM  Deliberately accent-free: Windows consoles use varying codepages.
REM ============================================================================

setlocal EnableExtensions EnableDelayedExpansion

REM --- ALTERE ESTES VALORES SE FIZER FORK / CHANGE THESE IF YOU FORK ---------
set "OWNER=your-org"
set "REPO=libreoffice-collab"
REM --------------------------------------------------------------------------

REM --- Deteta o idioma do Windows / Detect the Windows display language ------
set "LANGCODE=en"
for /f "tokens=3" %%L in ('reg query "HKCU\Control Panel\International" /v LocaleName 2^>nul ^| find "LocaleName"') do (
    set "LOCALE=%%L"
)
if defined LOCALE (
    echo !LOCALE! | findstr /b /i "pt" >nul && set "LANGCODE=pt"
) else (
    REM Sem informacao de locale, assumimos Portugues (predefinicao do produto).
    set "LANGCODE=pt"
)

if "%LANGCODE%"=="pt" (
    set "MSG_TITLE=LibreOffice Collab - Instalacao"
    set "MSG_START=A preparar a instalacao. Aguarde..."
    set "MSG_NOPS=Nao foi encontrado o PowerShell. Atualize o Windows e tente de novo."
    set "MSG_FAIL=A instalacao nao foi concluida. Contacte o suporte informatico."
) else (
    set "MSG_TITLE=LibreOffice Collab - Setup"
    set "MSG_START=Preparing the installation. Please wait..."
    set "MSG_NOPS=PowerShell was not found. Please update Windows and try again."
    set "MSG_FAIL=The installation did not complete. Please contact IT support."
)

title %MSG_TITLE%
echo.
echo   ===================================================
echo    %MSG_TITLE%
echo   ===================================================
echo.
echo   %MSG_START%
echo.

REM --- Verifica o PowerShell / Check PowerShell is available ----------------
where powershell.exe >nul 2>&1
if errorlevel 1 (
    echo   %MSG_NOPS%
    echo.
    pause
    exit /b 1
)

REM --- Usa o install.ps1 local se existir, senao vai busca-lo ao GitHub -----
REM --- Use the local install.ps1 when present, otherwise fetch it -----------
set "SCRIPT=%~dp0install.ps1"

if exist "%SCRIPT%" (
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT%" -Owner "%OWNER%" -Repo "%REPO%"
) else (
    powershell.exe -NoProfile -ExecutionPolicy Bypass -Command ^
        "$ErrorActionPreference='Stop'; $u='https://raw.githubusercontent.com/%OWNER%/%REPO%/main/scripts/install.ps1'; $s=Join-Path $env:TEMP 'libreoffice-collab-install.ps1'; Invoke-WebRequest -Uri $u -OutFile $s -UseBasicParsing; & $s -Owner '%OWNER%' -Repo '%REPO%'"
)

if errorlevel 1 (
    echo.
    echo   %MSG_FAIL%
    echo.
    pause
    exit /b 1
)

endlocal
exit /b 0
