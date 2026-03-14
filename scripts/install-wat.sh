#!/bin/bash
set -euo pipefail

APP_NAME="quewat"
BIN_URL="https://github.com/AT-290690/que-script/releases/latest/download/$APP_NAME"
LIB_URL="https://github.com/AT-290690/que-script/releases/latest/download/que-lib.lisp"
BIN_PATH="/usr/local/bin/$APP_NAME"
LIB_DIR="/usr/local/share/que"
LIB_PATH="$LIB_DIR/que-lib.lisp"

echo "Installing $APP_NAME..."

curl -fsSL "$BIN_URL" -o "/tmp/$APP_NAME"
chmod +x "/tmp/$APP_NAME"
sudo mv "/tmp/$APP_NAME" "$BIN_PATH"
echo "Installed binary: $BIN_PATH"

echo "Installing que-lib.lisp..."
curl -fsSL "$LIB_URL" -o "/tmp/que-lib.lisp"
sudo mkdir -p "$LIB_DIR"
sudo mv "/tmp/que-lib.lisp" "$LIB_PATH"
echo "Installed library: $LIB_PATH"

echo "Done (quewat + que-lib.lisp)."
