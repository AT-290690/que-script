#!/bin/bash
set -euo pipefail

REPO="https://github.com/AT-290690/que-script/releases/latest/download"
LIB_ASSET="fusion-builtins.lisp"
TMP_DIR="/tmp"
DST_DIR="/usr/local/share/que"
DST_LIB="$DST_DIR/que-lib.lisp"
SRC_URL="$REPO/$LIB_ASSET"
TMP_LIB="$TMP_DIR/$LIB_ASSET"

echo "Installing fusion-only library to $DST_LIB"
if curl -fsSL "$SRC_URL" -o "$TMP_LIB"; then
  sudo mkdir -p "$DST_DIR"
  sudo mv "$TMP_LIB" "$DST_LIB"
  echo "Installed $DST_LIB (from release asset $LIB_ASSET)"
  echo "Done."
  echo "Active library is now fusion-only built-ins."
else
  echo "Failed to download $SRC_URL"
  echo "Check releases: https://github.com/AT-290690/que-script/releases"
  exit 1
fi
