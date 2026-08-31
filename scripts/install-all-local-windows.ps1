Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Build = $true
$InstallDeps = $true
$RootDir = Join-Path $PSScriptRoot ".."

function Show-Usage {
    Write-Host "Usage: .\scripts\install-all-local-windows.ps1 [-NoBuild] [-NoDeps]"
    Write-Host ""
    Write-Host "Installs the local Windows Que toolchain in one pass:"
    Write-Host "  - runtime dependencies: wasmtime, wabt (provides wasm2c)"
    Write-Host "  - local que binary"
    Write-Host "  - local que-lib.lisp"
    Write-Host "  - local quelsp binary"
}

function Test-Command([string]$Name) {
    return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

function Install-WithWinget([string]$Id) {
    winget install --id $Id --accept-package-agreements --accept-source-agreements
}

foreach ($Arg in $args) {
    switch ($Arg) {
        "-NoBuild" { $Build = $false }
        "-NoDeps" { $InstallDeps = $false }
        "-h" { Show-Usage; exit 0 }
        "--help" { Show-Usage; exit 0 }
        default { throw "Unknown option: $Arg" }
    }
}

if (-not $IsWindows) {
    throw "This installer is intended for Windows only."
}

Push-Location $RootDir
try {
    if ($InstallDeps) {
        if (Test-Command "winget") {
            Write-Host "Checking Windows dependencies..."
            if (-not (Test-Command "clang") -and -not (Test-Command "cc")) {
                Write-Host "Missing clang/cc. Install a C toolchain manually, for example LLVM or Visual Studio Build Tools."
                throw "Missing C compiler."
            }
            if (-not (Test-Command "wasmtime")) {
                Write-Host "Installing wasmtime..."
                Install-WithWinget "BytecodeAlliance.Wasmtime"
            }
            if (-not (Test-Command "wasm2c")) {
                Write-Host "Install WABT manually so `wasm2c.exe` is on PATH."
                throw "Missing wasm2c."
            }
        } else {
            throw "Automatic dependency installation is only wired for winget systems right now. Install wasmtime, wabt (wasm2c), and clang/cc manually or rerun with -NoDeps."
        }
    }

    if (-not (Test-Command "clang") -and -not (Test-Command "cc")) {
        throw "Missing C compiler. Install LLVM clang or Visual Studio Build Tools."
    }
    if (-not (Test-Command "wasmtime")) {
        throw "Missing wasmtime in PATH. Install it before using runtime-enabled que/queio."
    }
    if (-not (Test-Command "wasm2c")) {
        throw "Missing wasm2c in PATH. Install WABT and make sure wasm2c.exe is on PATH."
    }

    if ($Build) {
        Write-Host "Building local release binaries..."
        cargo build --release --no-default-features --features shell-runtime --bin queio
        cargo build --release --no-default-features --features io --bin quelsp
    }

    Write-Host "Installing que..."
    & (Join-Path $PSScriptRoot "install-local-windows.ps1") -NoBuild

    Write-Host "Installing quelsp..."
    & (Join-Path $PSScriptRoot "install-lsp-local-windows.ps1") -NoBuild

    Write-Host "Installed local Windows Que toolchain."
    Write-Host "Check with:"
    Write-Host "  que --help"
    Write-Host "  que native-c --help"
    Write-Host "  quelsp --help"
} finally {
    Pop-Location
}
