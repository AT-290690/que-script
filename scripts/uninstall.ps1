Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$InstallRoot = Join-Path $env:LOCALAPPDATA "Programs\Que"
$BinDir = Join-Path $InstallRoot "bin"
$ShareDir = Join-Path $InstallRoot "share\que"
$LibPath = Join-Path $ShareDir "que-lib.lisp"
$Binaries = @(
    (Join-Path $BinDir "que.exe"),
    (Join-Path $BinDir "quewat.exe"),
    (Join-Path $BinDir "quelsp.exe")
)

function Remove-UserPathEntry([string]$PathEntry) {
    $current = [Environment]::GetEnvironmentVariable("Path", "User")
    if (-not $current) {
        return
    }
    $parts = $current -split ';' | Where-Object { $_ -and $_ -ne $PathEntry }
    [Environment]::SetEnvironmentVariable("Path", ($parts -join ';'), "User")
}

foreach ($target in $Binaries) {
    if (Test-Path $target) {
        Remove-Item -Force $target
        Write-Host "Removed $target"
    }
}

if (Test-Path $LibPath) {
    Remove-Item -Force $LibPath
    Write-Host "Removed $LibPath"
}

if (Test-Path $ShareDir) {
    try { Remove-Item -Force $ShareDir } catch {}
}
if (Test-Path (Join-Path $InstallRoot "share")) {
    try { Remove-Item -Force (Join-Path $InstallRoot "share") } catch {}
}
if (Test-Path $BinDir) {
    try { Remove-Item -Force $BinDir } catch {}
}

Remove-UserPathEntry $BinDir

Write-Host "Uninstall complete."
