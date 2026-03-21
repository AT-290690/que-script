Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Enable-Tls12 {
    try {
        [Net.ServicePointManager]::SecurityProtocol = `
            [Net.ServicePointManager]::SecurityProtocol -bor `
            [Net.SecurityProtocolType]::Tls12
    } catch {}
}

Enable-Tls12

$ReleaseBase = "https://github.com/AT-290690/que-script/releases/latest/download"
$InstallRoot = Join-Path $env:LOCALAPPDATA "Programs\Que"
$BinDir = Join-Path $InstallRoot "bin"

function Ensure-UserPathContains([string]$PathEntry) {
    $current = [Environment]::GetEnvironmentVariable("Path", "User")
    $parts = @()
    if ($current) {
        $parts = $current -split ';' | Where-Object { $_ -ne "" }
    }
    if ($parts -notcontains $PathEntry) {
        $newPath = if ($current -and $current.Trim() -ne "") {
            "$current;$PathEntry"
        } else {
            $PathEntry
        }
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        Write-Host "Added to user PATH: $PathEntry"
    }
}

New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

$LspExe = Join-Path $BinDir "quelsp.exe"

Write-Host "Installing quelsp.exe..."
Invoke-WebRequest -Uri "$ReleaseBase/quelsp.exe" -OutFile $LspExe

Ensure-UserPathContains $BinDir

Write-Host "Installed:"
Write-Host "  $LspExe"
Write-Host ""
Write-Host "VS Code setting:"
Write-Host '  "que.languageServer.path": "'"$LspExe"'"'
