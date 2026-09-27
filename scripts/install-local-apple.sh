#!/bin/bash
set -euo pipefail

APP_NAME="que"
BIN_SOURCE="./target/release/que"
BIN_PATH="/usr/local/bin/que"
COMPILER_SOURCE="./target/release/quec"
COMPILER_PATH="/usr/local/bin/quec"
LEGACY_ECLISP_PATH="/usr/local/bin/eclisp"
LIB_DIR="/usr/local/share/que"
LIB_PATH="${LIB_DIR}/que-lib.lisp"
BUILD=1

usage() {
  cat <<'EOF'
Usage: ./scripts/install-local-apple.sh [--no-build]

Builds the lightweight Que frontend and compiler.

Installs:
  /usr/local/bin/que
  /usr/local/bin/quec
  /usr/local/share/que/que-lib.lisp
  /usr/local/share/que/compile-native-c.sh and its C host

Options:
  --no-build   Install existing release binaries and a freshly baked library.
  -h, --help   Show this help.
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --no-build)
      BUILD=0
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
  echo "This local installer is intended for macOS only." >&2
  exit 1
fi

if [ "$BUILD" -eq 1 ]; then
  echo "Building local ${APP_NAME} release binary..."
  cargo build --release --no-default-features --bin que
  cargo build --release --no-default-features --features compiler --bin quec
fi
if [ ! -x "$COMPILER_SOURCE" ]; then
  echo "Missing executable: ${COMPILER_SOURCE}" >&2
  exit 1
fi

if [ ! -x "$BIN_SOURCE" ]; then
  echo "Missing executable: ${BIN_SOURCE}" >&2
  echo "Run without --no-build, or build it first." >&2
  exit 1
fi
if ! command -v wasm2c >/dev/null 2>&1; then
  if command -v brew >/dev/null 2>&1; then
    echo "Installing WABT (provides the external wasm2c runtime)..."
    brew install wabt
  else
    echo "warning: wasm2c is not installed; quec can compile Wasm but cannot use `quec run`" >&2
  fi
fi

tmp_bin="$(mktemp "/tmp/${APP_NAME}.local.XXXXXX")"
tmp_compiler="$(mktemp "/tmp/quec.local.XXXXXX")"
tmp_lib="$(mktemp "/tmp/que-lib.local.XXXXXX")"
trap 'rm -f "$tmp_bin" "$tmp_compiler" "$tmp_lib"' EXIT

cp "$BIN_SOURCE" "$tmp_bin"
chmod +x "$tmp_bin"
cp "$COMPILER_SOURCE" "$tmp_compiler"
chmod +x "$tmp_compiler"

echo "Baking local que-lib.lisp..."
cargo run --release --no-default-features --features repo-tools --bin quebake -- --out "$tmp_lib"

echo "Installing binary: ${BIN_PATH}"
sudo mkdir -p "$(dirname "$BIN_PATH")"
sudo mv "$tmp_bin" "$BIN_PATH"
echo "Installing compiler: ${COMPILER_PATH}"
sudo mv "$tmp_compiler" "$COMPILER_PATH"
if [ -e "$LEGACY_ECLISP_PATH" ]; then
  sudo rm -f "$LEGACY_ECLISP_PATH"
  echo "Removed provisional executable: ${LEGACY_ECLISP_PATH}"
fi

echo "Installing library: ${LIB_PATH}"
sudo mkdir -p "$LIB_DIR"
sudo mv "$tmp_lib" "$LIB_PATH"
sudo cp ./scripts/compile-native-c.sh "${LIB_DIR}/compile-native-c.sh"
sudo chmod +x "${LIB_DIR}/compile-native-c.sh"
sudo mkdir -p "${LIB_DIR}/native-c"
sudo cp ./miscs/native-c/que_host.c ./miscs/native-c/que_host.h "${LIB_DIR}/native-c/"

echo "Installed local macOS ${APP_NAME}."
echo "Check with: ${APP_NAME} --version"
echo "Install a WASI runtime separately: wasmtime (default), wasmer, or iwasm/WAMR."
echo "Select it with: que program.que --runtime <runtime>"
