Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Build = $true
$InstallRoot = Join-Path $env:LOCALAPPDATA "Programs\Que"
$BinDir = Join-Path $InstallRoot "bin"
$BinSource = Join-Path (Join-Path $PSScriptRoot "..") "target\release\quelsp.exe"
$LspExe = Join-Path $BinDir "quelsp.exe"

function Show-Usage {
    Write-Host "Usage: .\scripts\install-lsp-local-windows.ps1 [-NoBuild]"
    Write-Host ""
    Write-Host "Builds the local Windows Que LSP executable and installs it on this machine."
    Write-Host ""
    Write-Host "Installs:"
    Write-Host "  $LspExe"
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
    throw "This local LSP installer is intended for Windows only."
}

Push-Location (Join-Path $PSScriptRoot "..")
try {
    if ($Build) {
        Write-Host "Building local quelsp release binary..."
        cargo build --release --no-default-features --features io --bin quelsp
    }

    if (-not (Test-Path $BinSource)) {
        throw "Missing executable: $BinSource`nRun without -NoBuild, or build it first."
    }

    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
    Copy-Item -Force $BinSource $LspExe

    Write-Host "Installed local Windows quelsp."
    Write-Host "Check with: quelsp --help"
} finally {
    Pop-Location
}
