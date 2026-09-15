@echo off
REM ============================================================================
REM  LibreOffice Collab - Servidor em Python / Python server
REM
REM  PT: Alternativa ao instalar-servidor.bat que nao descarrega nenhum
REM      executavel: corre o servidor a partir de um unico script Python.
REM      Precisa do Python 3 (instala-se sozinho com o winget) e do Docker
REM      Desktop para o motor Collabora. Os outros PCs nao precisam de nada:
REM      basta abrirem o endereco que aparece na janela.
REM  EN: Alternative to instalar-servidor.bat that downloads no executable:
REM      runs the server from a single Python script. Needs Python 3 (installed
REM      through winget if missing) and Docker Desktop for the Collabora engine.
REM      The other PCs need nothing: they open the address shown in the window.
REM
REM  Argumentos passam para o script / arguments are forwarded, e.g.:
REM      servidor-python.bat --dir "D:\Docs" --port 7373
REM ============================================================================

setlocal EnableExtensions
title LibreOffice Collab - Servidor / Server (Python)
cd /d "%~dp0"

set "OWNER=Cmprfda"
set "REPO=libreoffice-collab"

REM --- localizar (ou instalar) o Python / find (or install) Python ------------
set "PY="
where python >nul 2>nul && set "PY=python"
if not defined PY where py >nul 2>nul && set "PY=py -3"
if not defined PY for /d %%D in ("%LOCALAPPDATA%\Programs\Python\Python3*") do set "PY=%%D\python.exe"

if not defined PY (
    echo   Python nao encontrado. A instalar com o winget... / Python not found. Installing with winget...
    winget install -e --id Python.Python.3.12 --scope user --silent --accept-package-agreements --accept-source-agreements
    for /d %%D in ("%LOCALAPPDATA%\Programs\Python\Python3*") do set "PY=%%D\python.exe"
)

if not defined PY (
    echo.
    echo   Nao foi possivel instalar o Python. Instale-o de https://www.python.org/downloads/
    echo   ^(marque "Add python.exe to PATH"^) e volte a correr este ficheiro.
    echo   Could not install Python. Install it from https://www.python.org/downloads/
    echo   ^(tick "Add python.exe to PATH"^) and run this file again.
    echo.
    pause
    exit /b 1
)

REM --- localizar o script / locate the script ---------------------------------
REM Ao lado deste .bat, na pasta server\ do repositorio, ou descarregado para
REM %LocalAppData% quando este .bat foi descarregado sozinho das Releases.
set "INSTALL=%LOCALAPPDATA%\Programs\LibreOffice Collab Server"
set "SCRIPT=%~dp0collab_server.py"
if not exist "%SCRIPT%" set "SCRIPT=%~dp0..\server\collab_server.py"
if not exist "%SCRIPT%" set "SCRIPT=%INSTALL%\collab_server.py"
if not exist "%SCRIPT%" (
    if not exist "%INSTALL%" mkdir "%INSTALL%"
    echo   A descarregar collab_server.py... / Downloading collab_server.py...
    powershell -NoProfile -Command "$ErrorActionPreference='Stop'; Invoke-WebRequest -Uri 'https://raw.githubusercontent.com/%OWNER%/%REPO%/main/server/collab_server.py' -OutFile '%SCRIPT%' -UseBasicParsing"
)
if not exist "%SCRIPT%" (
    echo   Nao foi possivel obter o collab_server.py. / Could not fetch collab_server.py.
    pause
    exit /b 1
)

REM --- opcional: mDNS para o servidor aparecer sozinho na aplicacao -----------
REM optional: mDNS so the server shows up on its own in the desktop app
%PY% -c "import zeroconf" >nul 2>nul || (
    echo   A instalar o zeroconf ^(opcional, so na primeira vez^)... / Installing zeroconf ^(optional, first time only^)...
    %PY% -m pip install --user --quiet zeroconf
    if errorlevel 1 echo   Aviso: sem zeroconf o servidor nao aparece sozinho na aplicacao; escreva o endereco em Definicoes. / Warning: without zeroconf type the address in Settings.
)

echo.
%PY% "%SCRIPT%" %*
echo.
pause
endlocal
