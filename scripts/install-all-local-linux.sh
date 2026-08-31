#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

BUILD=1
INSTALL_DEPS=1

usage() {
  cat <<'EOF'
Usage: ./scripts/install-all-local-linux.sh [--no-build] [--no-deps]

Installs the local Linux Que toolchain in one pass:
  - runtime dependencies: wasmtime, wabt (provides wasm2c)
  - local que binary
  - local que-lib.lisp
  - local quelsp binary

Options:
  --no-build   Reuse existing target/release binaries.
  --no-deps    Skip dependency installation checks.
  -h, --help   Show this help.
EOF
}

install_with_apt() {
  sudo apt-get update
  sudo apt-get install -y "$@"
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

if [ "$(uname -s)" != "Linux" ]; then
  echo "This installer is intended for Linux only." >&2
  exit 1
fi

if [ "$INSTALL_DEPS" -eq 1 ]; then
  if command -v apt-get >/dev/null 2>&1; then
    echo "Checking Linux dependencies..."
    if ! command -v clang >/dev/null 2>&1 && ! command -v cc >/dev/null 2>&1; then
      echo "Installing C compiler toolchain..."
      install_with_apt clang build-essential
    fi
    if ! command -v wasmtime >/dev/null 2>&1; then
      echo "Missing wasmtime in PATH." >&2
      echo "Install it first, for example from https://wasmtime.dev or your distro package source." >&2
      exit 1
    fi
    if ! command -v wasm2c >/dev/null 2>&1; then
      echo "Installing wabt (wasm2c)..."
      install_with_apt wabt
    fi
  else
    echo "Automatic dependency install is only wired for apt-based systems right now." >&2
    echo "Install these manually or rerun with --no-deps:" >&2
    echo "  - wasmtime" >&2
    echo "  - wabt (for wasm2c)" >&2
    echo "  - clang or cc" >&2
    exit 1
  fi
fi

if ! command -v clang >/dev/null 2>&1 && ! command -v cc >/dev/null 2>&1; then
  echo "Missing C compiler." >&2
  echo "Install `clang` or `build-essential`." >&2
  exit 1
fi

if ! command -v wasmtime >/dev/null 2>&1; then
  echo "Missing wasmtime in PATH." >&2
  echo "Install it before using runtime-enabled que/queio." >&2
  exit 1
fi

if ! command -v wasm2c >/dev/null 2>&1; then
  echo "Missing wasm2c in PATH." >&2
  echo "Install WABT: sudo apt-get install wabt" >&2
  exit 1
fi

if [ "$BUILD" -eq 1 ]; then
  echo "Building local release binaries..."
  cargo build --release --no-default-features --features shell-runtime --bin queio
  cargo build --release --no-default-features --features io --bin quelsp
fi

echo "Installing que..."
./scripts/install-local-linux.sh --no-build

echo "Installing quelsp..."
./scripts/install-lsp-local-linux.sh --no-build

echo "Installed local Linux Que toolchain."
echo "Check with:"
echo "  que --help"
echo "  que native-c --help"
echo "  quelsp --help"
