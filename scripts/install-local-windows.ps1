Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Build = $true
$InstallRoot = Join-Path $env:LOCALAPPDATA "Programs\Que"
$BinDir = Join-Path $InstallRoot "bin"
$ShareDir = Join-Path $InstallRoot "share\que"
$BinSource = Join-Path (Join-Path $PSScriptRoot "..") "target\release\queio.exe"
$LibSource = Join-Path $env:TEMP "que-lib.local.lisp"
$QueExe = Join-Path $BinDir "que.exe"
$LibPath = Join-Path $ShareDir "que-lib.lisp"

function Show-Usage {
    Write-Host "Usage: .\scripts\install-local-windows.ps1 [-NoBuild]"
    Write-Host ""
    Write-Host "Builds the local Windows Que executable and installs it on this machine."
    Write-Host ""
    Write-Host "Installs:"
    Write-Host "  $QueExe"
    Write-Host "  $LibPath"
}

foreach ($Arg in $args) {
    switch ($Arg) {
        "-NoBuild" { $Build = $false }
        "-h" { Show-Usage; exit 0 }
        "--help" { Show-Usage; exit 0 }
        default { throw "Unknown option: $Arg" }
    }
}

if (-not $IsWindows) {
    throw "This local installer is intended for Windows only."
}

Push-Location (Join-Path $PSScriptRoot "..")
try {
    if ($Build) {
        Write-Host "Building local que release binary..."
        cargo build --release --no-default-features --features shell-runtime --bin queio
    }

    if (-not (Test-Path $BinSource)) {
        throw "Missing executable: $BinSource`nRun without -NoBuild, or build it first."
    }

    Write-Host "Baking local que-lib.lisp..."
    cargo run --release --no-default-features --features repo-tools --bin quebake -- --out $LibSource

    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
    New-Item -ItemType Directory -Force -Path $ShareDir | Out-Null

    Copy-Item -Force $BinSource $QueExe
    Move-Item -Force $LibSource $LibPath

    Write-Host "Installed local Windows que."
    Write-Host "Check with: que --version"
} finally {
    if (Test-Path $LibSource) {
        Remove-Item -Force $LibSource
    }
    Pop-Location
}
