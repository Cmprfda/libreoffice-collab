@echo off
REM ============================================================================
REM  LibreOffice Collab - Servidor / Server
REM
REM  PT: Corra este ficheiro UMA vez, no computador que vai guardar e partilhar
REM      os documentos. Pede permissoes de administrador para abrir as portas
REM      na firewall. Nos outros computadores NAO e preciso correr nada disto.
REM  EN: Run this file ONCE, on the computer that will store and share the
REM      documents. It asks for administrator rights to open the firewall
REM      ports. The other computers do NOT need any of this.
REM ============================================================================

setlocal EnableExtensions

set "OWNER=Cmprfda"
set "REPO=libreoffice-collab"

REM --- Eleva para administrador se ainda nao estiver / self-elevate ----------
net session >nul 2>&1
if errorlevel 1 (
    echo.
    echo   A pedir permissoes de administrador... / Requesting administrator rights...
    powershell.exe -NoProfile -ExecutionPolicy Bypass -Command ^
        "Start-Process -FilePath '%~f0' -Verb RunAs"
    exit /b 0
)

set "SCRIPT=%~dp0host-setup.ps1"

if exist "%SCRIPT%" (
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT%" -Owner "%OWNER%" -Repo "%REPO%"
) else (
    powershell.exe -NoProfile -ExecutionPolicy Bypass -Command ^
        "$ErrorActionPreference='Stop'; $u='https://raw.githubusercontent.com/%OWNER%/%REPO%/main/scripts/host-setup.ps1'; $s=Join-Path $env:TEMP 'libreoffice-collab-host-setup.ps1'; Invoke-WebRequest -Uri $u -OutFile $s -UseBasicParsing; & $s -Owner '%OWNER%' -Repo '%REPO%'"
)

endlocal
exit /b 0
