<#
    LibreOffice Collab - configuracao do computador anfitriao
    LibreOffice Collab - host machine setup
    -----------------------------------------------------------------------
    PT: Corre UMA vez, no computador que vai partilhar os documentos.
        Instala o servidor, arranca o motor Collabora e abre as portas na
        firewall. Os restantes computadores nao precisam de nada disto.
    EN: Run ONCE, on the computer that will share the documents. Installs the
        server, starts the Collabora engine and opens the firewall ports.
        The other computers need none of this.

    Requer / Requires: Docker Desktop (https://docker.com/products/docker-desktop)
    Requer privilegios de administrador para as regras de firewall.
    Needs administrator rights for the firewall rules.
#>

[CmdletBinding()]
param(
    [string]$Owner    = 'your-org',
    [string]$Repo     = 'libreoffice-collab',
    # Pasta partilhada / shared folder
    [string]$DocsDir  = (Join-Path $env:PUBLIC 'Documentos Partilhados'),
    [int]$Port        = 7373,
    # Nome visivel na lista de servidores / name shown in the server list
    [string]$Name     = $env:COMPUTERNAME
)

$ErrorActionPreference = 'Stop'
$ProgressPreference    = 'SilentlyContinue'

$isPortuguese = $true
try {
    $culture = (Get-UICulture).Name
    if ($culture -and -not $culture.StartsWith('pt', 'CurrentCultureIgnoreCase')) {
        $isPortuguese = $false
    }
} catch { }

$T = if ($isPortuguese) {
    @{
        Title      = 'LibreOffice Collab - Servidor'
        NoDocker   = 'O Docker Desktop nao esta instalado. Instale-o a partir de https://docker.com e volte a correr este ficheiro.'
        DockerOff  = 'O Docker Desktop esta instalado mas nao esta a correr. Abra o Docker Desktop, espere que fique verde e volte a correr este ficheiro.'
        Folder     = 'Pasta partilhada:'
        Downloading= 'A descarregar o servidor...'
        Engine     = 'A arrancar o motor de colaboracao (Collabora). Na primeira vez demora alguns minutos...'
        Firewall   = 'A abrir as portas na firewall do Windows...'
        NoAdmin    = 'Sem permissoes de administrador: as regras de firewall nao foram criadas. Pode ter de as autorizar manualmente.'
        Autostart  = 'A configurar o arranque automatico com o Windows...'
        Done       = 'Servidor pronto.'
        Share      = 'Coloque os documentos nesta pasta:'
        Clients    = 'Nos outros computadores basta abrir o LibreOffice Collab: este servidor aparece sozinho.'
        PressKey   = 'Prima qualquer tecla para fechar.'
    }
} else {
    @{
        Title      = 'LibreOffice Collab - Server'
        NoDocker   = 'Docker Desktop is not installed. Install it from https://docker.com and run this file again.'
        DockerOff  = 'Docker Desktop is installed but not running. Start Docker Desktop, wait until it turns green, then run this file again.'
        Folder     = 'Shared folder:'
        Downloading= 'Downloading the server...'
        Engine     = 'Starting the collaboration engine (Collabora). The first run takes a few minutes...'
        Firewall   = 'Opening the Windows Firewall ports...'
        NoAdmin    = 'No administrator rights: firewall rules were not created. You may have to allow them manually.'
        Autostart  = 'Configuring automatic start with Windows...'
        Done       = 'Server ready.'
        Share      = 'Put the documents in this folder:'
        Clients    = 'On the other computers just open LibreOffice Collab: this server shows up on its own.'
        PressKey   = 'Press any key to close.'
    }
}

function Step([string]$m) { Write-Host "  $m" -ForegroundColor Cyan }
function Fail([string]$m) { Write-Host ''; Write-Host "  $m" -ForegroundColor Red; Write-Host ''; }

$Host.UI.RawUI.WindowTitle = $T.Title
Write-Host ''
Write-Host '  ===================================================' -ForegroundColor DarkCyan
Write-Host "   $($T.Title)" -ForegroundColor White
Write-Host '  ===================================================' -ForegroundColor DarkCyan
Write-Host ''

# ------------------------------------------------------------------ Docker --

if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
    Fail $T.NoDocker
    Start-Process 'https://www.docker.com/products/docker-desktop/'
    if ($Host.Name -eq 'ConsoleHost') { $null = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown') }
    exit 1
}

docker info *> $null
if ($LASTEXITCODE -ne 0) {
    Fail $T.DockerOff
    if ($Host.Name -eq 'ConsoleHost') { $null = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown') }
    exit 1
}

# ----------------------------------------------------- address + folder ----

# The LAN address other machines will use. Picking the interface that carries
# the default route avoids VPN and virtual adapters.
$hostIp = (
    Get-NetIPConfiguration |
    Where-Object { $_.IPv4DefaultGateway -and $_.NetAdapter.Status -eq 'Up' } |
    Select-Object -First 1
).IPv4Address.IPAddress

if (-not $hostIp) {
    $hostIp = (Get-NetIPAddress -AddressFamily IPv4 |
        Where-Object { $_.IPAddress -notlike '127.*' -and $_.IPAddress -notlike '169.254.*' } |
        Select-Object -First 1).IPAddress
}

New-Item -ItemType Directory -Path $DocsDir -Force | Out-Null
Write-Host "  $($T.Folder) $DocsDir" -ForegroundColor Green
Write-Host "  IP: $hostIp" -ForegroundColor Green
Write-Host ''

$installDir = Join-Path $env:LOCALAPPDATA 'Programs\LibreOffice Collab Server'
New-Item -ItemType Directory -Path $installDir -Force | Out-Null

# ----------------------------------------------------------- server binary --

Step $T.Downloading

$release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Owner/$Repo/releases/latest" -Headers @{
    'User-Agent' = 'libreoffice-collab-host-setup'
    'Accept'     = 'application/vnd.github+json'
}
$asset = $release.assets | Where-Object { $_.name -eq 'collab-server.exe' } | Select-Object -First 1
if (-not $asset) {
    Fail 'collab-server.exe not found in the latest release.'
    exit 1
}

$serverExe = Join-Path $installDir 'collab-server.exe'
# Stop a previous instance before overwriting it.
Get-Process -Name 'collab-server' -ErrorAction SilentlyContinue | Stop-Process -Force
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $serverExe -UseBasicParsing

# ------------------------------------------------------------- compose file --

$composeDir = Join-Path $installDir 'docker'
New-Item -ItemType Directory -Path $composeDir -Force | Out-Null

$composeUrl = "https://raw.githubusercontent.com/$Owner/$Repo/main/docker/docker-compose.yml"
Invoke-WebRequest -Uri $composeUrl -OutFile (Join-Path $composeDir 'docker-compose.yml') -UseBasicParsing

# docker compose reads .env from the compose file's directory.
@"
HOST_IP=$hostIp
COLLAB_PORT=$Port
COOL_ADMIN_PASSWORD=collab
"@ | Set-Content -Path (Join-Path $composeDir '.env') -Encoding utf8

Step $T.Engine
Push-Location $composeDir
docker compose up -d
$composeExit = $LASTEXITCODE
Pop-Location
if ($composeExit -ne 0) {
    Fail "docker compose failed (exit $composeExit)."
    exit $composeExit
}

# ---------------------------------------------------------------- firewall --

Step $T.Firewall
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()
    ).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if ($isAdmin) {
    # Private profile only: this is an office LAN tool, not a public service.
    $rules = @(
        @{ Name = 'LibreOffice Collab (WOPI)';      Port = $Port },
        @{ Name = 'LibreOffice Collab (Collabora)'; Port = 9980 },
        @{ Name = 'LibreOffice Collab (mDNS)';      Port = 5353; Protocol = 'UDP' }
    )
    foreach ($rule in $rules) {
        $protocol = if ($rule.Protocol) { $rule.Protocol } else { 'TCP' }
        Remove-NetFirewallRule -DisplayName $rule.Name -ErrorAction SilentlyContinue
        New-NetFirewallRule -DisplayName $rule.Name -Direction Inbound -Action Allow `
            -Protocol $protocol -LocalPort $rule.Port -Profile Private, Domain | Out-Null
    }
} else {
    Write-Host "  $($T.NoAdmin)" -ForegroundColor Yellow
}

# --------------------------------------------------------------- autostart --

Step $T.Autostart

$startup  = [Environment]::GetFolderPath('Startup')
$shell    = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut((Join-Path $startup 'LibreOffice Collab Server.lnk'))
$shortcut.TargetPath       = $serverExe
$shortcut.Arguments        = "--dir `"$DocsDir`" --port $Port --cool http://${hostIp}:9980 --name `"$Name`""
$shortcut.WorkingDirectory = $installDir
$shortcut.Description      = 'LibreOffice Collab Server'
$shortcut.Save()

# Start it now as well, so nobody has to reboot.
Start-Process -FilePath $serverExe -ArgumentList @(
    '--dir', $DocsDir, '--port', $Port, '--cool', "http://${hostIp}:9980", '--name', $Name
)

Write-Host ''
Write-Host "  $($T.Done)" -ForegroundColor Green
Write-Host "  $($T.Share) $DocsDir"
Write-Host "  $($T.Clients)"
Write-Host ''

if ($Host.Name -eq 'ConsoleHost' -and -not $env:CI) {
    Write-Host "  $($T.PressKey)" -ForegroundColor DarkGray
    $null = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown')
}
