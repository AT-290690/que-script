#!/bin/bash
set -euo pipefail

REPO="https://github.com/AT-290690/que-script/releases/latest/download"
LIB_DIR="/usr/local/share/que"
LIB_NAME="que-lib.lisp"
TMP_DIR="/tmp"

lib_url="$REPO/$LIB_NAME"
lib_tmp="$TMP_DIR/$LIB_NAME"
lib_path="$LIB_DIR/$LIB_NAME"

echo "Installing $LIB_NAME..."
if curl -fsSL "$lib_url" -o "$lib_tmp"; then
  sudo mkdir -p "$LIB_DIR"
  sudo mv "$lib_tmp" "$lib_path"
  echo "Installed $lib_path"
else
  echo "Failed to download $lib_url"
  echo "Check releases: https://github.com/AT-290690/que-script/releases"
  exit 1
fi

echo "Done."
