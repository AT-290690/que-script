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
$ShareDir = Join-Path $InstallRoot "share\que"

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
New-Item -ItemType Directory -Force -Path $ShareDir | Out-Null

$QueExe = Join-Path $BinDir "que.exe"
$LibPath = Join-Path $ShareDir "que-lib.lisp"

Write-Host "Installing que.exe..."
Invoke-WebRequest -Uri "$ReleaseBase/que.exe" -OutFile $QueExe

Write-Host "Installing que-lib.lisp..."
Invoke-WebRequest -Uri "$ReleaseBase/que-lib.lisp" -OutFile $LibPath

Ensure-UserPathContains $BinDir

Write-Host "Installed:"
Write-Host "  $QueExe"
Write-Host "  $LibPath"
Write-Host ""
Write-Host 'Restart the terminal, or in PowerShell run:'
Write-Host '  $env:Path = [Environment]::GetEnvironmentVariable("Path", "User") + ";" + [Environment]::GetEnvironmentVariable("Path", "Machine")'
