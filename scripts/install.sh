#!/bin/bash
set -euo pipefail

APP_NAME="que"
RELEASE_BASE="https://github.com/AT-290690/que-script/releases/latest/download"
BIN_PATH="/usr/local/bin/$APP_NAME"
RUNTIME_PATH="/usr/local/bin/que-runtime"
COMPILER_PATH="/usr/local/bin/quec"
LIB_DIR="/usr/local/share/que"
LIB_PATH="$LIB_DIR/que-lib.lisp"

detect_target() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "${arch}" in
    x86_64|amd64) arch="x86_64" ;;
    arm64|aarch64) arch="aarch64" ;;
    *)
      echo "Unsupported architecture: ${arch}" >&2
      exit 1
      ;;
  esac

  case "${os}" in
    Linux) echo "${arch}-unknown-linux-gnu" ;;
    Darwin) echo "${arch}-apple-darwin" ;;
    *)
      echo "Unsupported operating system: ${os}" >&2
      exit 1
      ;;
  esac
}

TARGET="$(detect_target)"
BIN_URL="${RELEASE_BASE}/${APP_NAME}-${TARGET}"
RUNTIME_URL="${RELEASE_BASE}/que-runtime-${TARGET}"
COMPILER_URL="${RELEASE_BASE}/quec-${TARGET}"
LIB_URL="${RELEASE_BASE}/que-lib-${TARGET}.lisp"

echo "Installing $APP_NAME..."
echo "Resolved target: $TARGET"

curl -fsSL "$BIN_URL" -o "/tmp/$APP_NAME"
chmod +x "/tmp/$APP_NAME"
sudo mv "/tmp/$APP_NAME" "$BIN_PATH"
echo "Installed binary: $BIN_PATH"

curl -fsSL "$RUNTIME_URL" -o /tmp/que-runtime
chmod +x /tmp/que-runtime
sudo mv /tmp/que-runtime "$RUNTIME_PATH"
echo "Installed compatibility runtime: $RUNTIME_PATH"

curl -fsSL "$COMPILER_URL" -o /tmp/quec
chmod +x /tmp/quec
sudo mv /tmp/quec "$COMPILER_PATH"
echo "Installed compiler: $COMPILER_PATH"

if ! command -v wasmtime >/dev/null 2>&1; then
  echo "Installing external Wasmtime CLI..."
  curl https://wasmtime.dev/install.sh -sSf | bash
fi

echo "Installing que-lib.lisp..."
curl -fsSL "$LIB_URL" -o "/tmp/que-lib.lisp"
sudo mkdir -p "$LIB_DIR"
sudo mv "/tmp/que-lib.lisp" "$LIB_PATH"
echo "Installed library: $LIB_PATH"

echo "Done (que, quec, compatibility runtime, external Wasmtime, and que-lib.lisp)."
