<#
    LibreOffice Collab - instalador de um clique / one-click installer
    ------------------------------------------------------------------
    PT: Descarrega a versao mais recente do GitHub, instala em
        %LocalAppData%\Programs, cria atalhos e abre a aplicacao.
    EN: Downloads the latest release from GitHub, installs it into
        %LocalAppData%\Programs, creates shortcuts and launches the app.

    Uso / Usage:
        irm https://raw.githubusercontent.com/Cmprfda/libreoffice-collab/main/scripts/install.ps1 | iex

    Nao requer permissoes de administrador. / No administrator rights required.
#>

[CmdletBinding()]
param(
    # CHANGE THESE WHEN YOU FORK THE PROJECT (see src/lib/config.ts).
    [string]$Owner = 'Cmprfda',
    [string]$Repo  = 'libreoffice-collab',
    # Optional: install a specific tag instead of the latest release.
    [string]$Tag   = ''
)

$ErrorActionPreference = 'Stop'
$ProgressPreference    = 'SilentlyContinue'   # much faster Invoke-WebRequest

# --------------------------------------------------------------------- i18n --

# Portuguese is the default; anything not pt-* falls back to English.
$isPortuguese = $true
try {
    $culture = (Get-UICulture).Name
    if ($culture -and -not $culture.StartsWith('pt', 'CurrentCultureIgnoreCase')) {
        $isPortuguese = $false
    }
} catch {
    # Locale unavailable (rare, e.g. Server Core) - keep the Portuguese default.
}

$T = if ($isPortuguese) {
    @{
        Title       = 'LibreOffice Collab - Instalacao'
        Checking    = 'A procurar a versao mais recente...'
        Found       = 'Versao encontrada:'
        Downloading = 'A descarregar ({0})...'
        Installing  = 'A instalar. Isto demora cerca de um minuto...'
        Shortcut    = 'A criar atalhos no Ambiente de Trabalho e no Menu Iniciar...'
        Launching   = 'A abrir o LibreOffice Collab...'
        Done        = 'Instalacao concluida.'
        FailApi     = 'Nao foi possivel contactar o GitHub. Verifique a ligacao a Internet.'
        FailAsset   = 'A versao mais recente nao inclui um instalador para Windows.'
        FailInstall = 'A instalacao falhou (codigo {0}).'
        PressKey    = 'Prima qualquer tecla para fechar esta janela.'
    }
} else {
    @{
        Title       = 'LibreOffice Collab - Setup'
        Checking    = 'Looking for the latest version...'
        Found       = 'Version found:'
        Downloading = 'Downloading ({0})...'
        Installing  = 'Installing. This takes about a minute...'
        Shortcut    = 'Creating Desktop and Start Menu shortcuts...'
        Launching   = 'Starting LibreOffice Collab...'
        Done        = 'Installation complete.'
        FailApi     = 'Could not reach GitHub. Check your internet connection.'
        FailAsset   = 'The latest release does not include a Windows installer.'
        FailInstall = 'Installation failed (exit code {0}).'
        PressKey    = 'Press any key to close this window.'
    }
}

function Write-Step([string]$Message) {
    Write-Host "  $Message" -ForegroundColor Cyan
}

function Write-Fail([string]$Message) {
    Write-Host ''
    Write-Host "  $Message" -ForegroundColor Red
    Write-Host ''
}

# ------------------------------------------------------------------ banner --

$Host.UI.RawUI.WindowTitle = $T.Title
Write-Host ''
Write-Host '  ===================================================' -ForegroundColor DarkCyan
Write-Host "   $($T.Title)" -ForegroundColor White
Write-Host '  ===================================================' -ForegroundColor DarkCyan
Write-Host ''

# ------------------------------------------------------- find the release --

Write-Step $T.Checking

$apiUrl = if ($Tag) {
    "https://api.github.com/repos/$Owner/$Repo/releases/tags/$Tag"
} else {
    "https://api.github.com/repos/$Owner/$Repo/releases/latest"
}

try {
    # GitHub requires a User-Agent header on every API call.
    $release = Invoke-RestMethod -Uri $apiUrl -Headers @{
        'User-Agent' = 'libreoffice-collab-installer'
        'Accept'     = 'application/vnd.github+json'
    }
} catch {
    Write-Fail $T.FailApi
    exit 1
}

Write-Host "  $($T.Found) $($release.tag_name)" -ForegroundColor Green

# Prefer the NSIS setup executable; fall back to an MSI if one is published.
# Match '-setup.exe' rather than any '.exe': the release also carries
# collab-server.exe, and the API happens to list it first, so a looser match
# downloads the server binary and the app is never installed.
$asset = $release.assets |
    Where-Object { $_.name -match '-setup\.exe$' -and $_.name -notmatch 'debug' } |
    Select-Object -First 1
if (-not $asset) {
    $asset = $release.assets | Where-Object { $_.name -match '\.msi$' } | Select-Object -First 1
}
if (-not $asset) {
    Write-Fail $T.FailAsset
    exit 1
}

# ----------------------------------------------------------------- download --

$sizeMb   = [math]::Round($asset.size / 1MB, 1)
Write-Step ($T.Downloading -f "$sizeMb MB")

$tempDir  = Join-Path $env:TEMP 'libreoffice-collab-setup'
New-Item -ItemType Directory -Path $tempDir -Force | Out-Null
$package  = Join-Path $tempDir $asset.name

Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $package -UseBasicParsing

# ------------------------------------------------------------------ install --

Write-Step $T.Installing

$installDir = Join-Path $env:LOCALAPPDATA 'Programs\LibreOffice Collab'

if ($package.EndsWith('.msi')) {
    $arguments = "/i `"$package`" /qn INSTALLDIR=`"$installDir`""
    $process = Start-Process -FilePath 'msiexec.exe' -ArgumentList $arguments -Wait -PassThru
} else {
    # NSIS: /S is silent, /D sets the target directory and must come last and
    # unquoted, so the whole argument list is passed as a single raw string.
    $process = Start-Process -FilePath $package -ArgumentList "/S /D=$installDir" -Wait -PassThru
}

if ($process.ExitCode -ne 0) {
    Write-Fail ($T.FailInstall -f $process.ExitCode)
    exit $process.ExitCode
}

Remove-Item $package -Force -ErrorAction SilentlyContinue

# ---------------------------------------------------------------- shortcuts --

Write-Step $T.Shortcut

$exePath = Join-Path $installDir 'LibreOffice Collab.exe'
if (-not (Test-Path $exePath)) {
    # Older installers may not honour /D; find the executable wherever it landed.
    $found = Get-ChildItem -Path (Join-Path $env:LOCALAPPDATA 'Programs') `
        -Filter 'LibreOffice Collab.exe' -Recurse -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if ($found) { $exePath = $found.FullName }
}

function New-Shortcut([string]$Path, [string]$Target) {
    $shell    = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($Path)
    $shortcut.TargetPath       = $Target
    $shortcut.WorkingDirectory = Split-Path $Target -Parent
    $shortcut.IconLocation     = "$Target,0"
    $shortcut.Description      = 'LibreOffice Collab'
    $shortcut.Save()
}

if (Test-Path $exePath) {
    $desktop   = [Environment]::GetFolderPath('Desktop')
    $startMenu = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'

    New-Shortcut (Join-Path $desktop   'LibreOffice Collab.lnk') $exePath
    New-Shortcut (Join-Path $startMenu 'LibreOffice Collab.lnk') $exePath
}

# ------------------------------------------------------------------- launch --

Write-Step $T.Launching
if (Test-Path $exePath) {
    Start-Process -FilePath $exePath
}

Write-Host ''
Write-Host "  $($T.Done)" -ForegroundColor Green
Write-Host ''

# Only pause when a human is watching a console we opened ourselves.
if ($Host.Name -eq 'ConsoleHost' -and -not $env:CI) {
    Write-Host "  $($T.PressKey)" -ForegroundColor DarkGray
    $null = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown')
}
