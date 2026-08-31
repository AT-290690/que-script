#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

BUILD=1
INSTALL_DEPS=1

usage() {
  cat <<'EOF'
Usage: ./scripts/install-all-local-apple.sh [--no-build] [--no-deps]

Installs the local macOS Que toolchain in one pass:
  - Homebrew dependencies: wasmtime, wabt (provides wasm2c)
  - local que binary
  - local que-lib.lisp
  - local quelsp binary

Options:
  --no-build   Reuse existing target/release binaries.
  --no-deps    Skip Homebrew dependency installation checks.
  -h, --help   Show this help.
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --no-build)
      BUILD=0
      shift
      ;;
    --no-deps)
      INSTALL_DEPS=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [ "$(uname -s)" != "Darwin" ]; then
  echo "This installer is intended for macOS only." >&2
  exit 1
fi

if [ "$INSTALL_DEPS" -eq 1 ]; then
  if ! command -v brew >/dev/null 2>&1; then
    echo "Homebrew is required to install wasmtime and wabt." >&2
    echo "Install Homebrew first: https://brew.sh" >&2
    exit 1
  fi

  echo "Checking runtime dependencies..."
  if ! brew list --formula wasmtime >/dev/null 2>&1; then
    echo "Installing wasmtime..."
    brew install wasmtime
  fi
  if ! brew list --formula wabt >/dev/null 2>&1; then
    echo "Installing wabt (wasm2c)..."
    brew install wabt
  fi
fi

if ! command -v clang >/dev/null 2>&1 && ! command -v cc >/dev/null 2>&1; then
  echo "Missing C compiler." >&2
  echo "Install Xcode Command Line Tools: xcode-select --install" >&2
  exit 1
fi

if ! command -v wasmtime >/dev/null 2>&1; then
  echo "Missing wasmtime in PATH." >&2
  echo "Install it with Homebrew: brew install wasmtime" >&2
  exit 1
fi

if ! command -v wasm2c >/dev/null 2>&1; then
  echo "Missing wasm2c in PATH." >&2
  echo "Install WABT with Homebrew: brew install wabt" >&2
  exit 1
fi

if [ "$BUILD" -eq 1 ]; then
  echo "Building local release binaries..."
  cargo build --release --no-default-features --features shell-runtime --bin queio
  cargo build --release --no-default-features --features io --bin quelsp
fi

echo "Installing que..."
if [ "$BUILD" -eq 1 ]; then
  ./scripts/install-local-apple.sh --no-build
else
  ./scripts/install-local-apple.sh --no-build
fi

echo "Installing quelsp..."
if [ "$BUILD" -eq 1 ]; then
  ./scripts/install-lsp-local-apple.sh --no-build
else
  ./scripts/install-lsp-local-apple.sh --no-build
fi

echo "Installed local macOS Que toolchain."
echo "Check with:"
echo "  que --help"
echo "  que native-c --help"
echo "  quelsp --help"
