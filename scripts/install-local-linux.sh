#!/bin/bash
set -euo pipefail

APP_NAME="que"
BIN_SOURCE="./target/release/queio"
BIN_PATH="/usr/local/bin/${APP_NAME}"
LIB_DIR="/usr/local/share/que"
LIB_PATH="${LIB_DIR}/que-lib.lisp"
BUILD=1

usage() {
  cat <<'EOF'
Usage: ./scripts/install-local-linux.sh [--no-build]

Builds the local Linux Que executable and installs it on this machine.

Installs:
  /usr/local/bin/que
  /usr/local/share/que/que-lib.lisp

Options:
  --no-build   Install existing target/release/queio and a freshly baked library.
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

if [ "$(uname -s)" != "Linux" ]; then
  echo "This local installer is intended for Linux only." >&2
  exit 1
fi

if [ "$BUILD" -eq 1 ]; then
  echo "Building local ${APP_NAME} release binary..."
  cargo build --release --no-default-features --features shell-runtime --bin queio
fi

if [ ! -x "$BIN_SOURCE" ]; then
  echo "Missing executable: ${BIN_SOURCE}" >&2
  echo "Run without --no-build, or build it first." >&2
  exit 1
fi

tmp_bin="$(mktemp "/tmp/${APP_NAME}.local.XXXXXX")"
tmp_lib="$(mktemp "/tmp/que-lib.local.XXXXXX")"
trap 'rm -f "$tmp_bin" "$tmp_lib"' EXIT

cp "$BIN_SOURCE" "$tmp_bin"
chmod +x "$tmp_bin"

echo "Baking local que-lib.lisp..."
cargo run --release --no-default-features --features repo-tools --bin quebake -- --out "$tmp_lib"

echo "Installing binary: ${BIN_PATH}"
sudo mkdir -p "$(dirname "$BIN_PATH")"
sudo mv "$tmp_bin" "$BIN_PATH"

echo "Installing library: ${LIB_PATH}"
sudo mkdir -p "$LIB_DIR"
sudo mv "$tmp_lib" "$LIB_PATH"

echo "Installed local Linux ${APP_NAME}."
echo "Check with: ${APP_NAME} --version"
