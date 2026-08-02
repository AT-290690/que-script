#!/bin/bash
set -euo pipefail

APP_NAME="quelsp"
BIN_SOURCE="./target/release/${APP_NAME}"
BIN_PATH="/usr/local/bin/${APP_NAME}"
BUILD=1

usage() {
  cat <<'EOF'
Usage: ./scripts/install-lsp-local-apple.sh [--no-build]

Builds the local macOS Que LSP executable and installs it on this machine.

Installs:
  /usr/local/bin/quelsp

Options:
  --no-build   Install existing target/release/quelsp.
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
  echo "This local LSP installer is intended for macOS only." >&2
  exit 1
fi

if [ "$BUILD" -eq 1 ]; then
  echo "Building local ${APP_NAME} release binary..."
  cargo build --release --features io --bin "$APP_NAME"
fi

if [ ! -x "$BIN_SOURCE" ]; then
  echo "Missing executable: ${BIN_SOURCE}" >&2
  echo "Run without --no-build, or build it first." >&2
  exit 1
fi

tmp_bin="$(mktemp "/tmp/${APP_NAME}.local.XXXXXX")"
trap 'rm -f "$tmp_bin"' EXIT

cp "$BIN_SOURCE" "$tmp_bin"
chmod +x "$tmp_bin"

echo "Installing binary: ${BIN_PATH}"
sudo mkdir -p "$(dirname "$BIN_PATH")"
sudo mv "$tmp_bin" "$BIN_PATH"

echo "Installed local macOS ${APP_NAME}."
echo "Check with: ${APP_NAME} --help"
