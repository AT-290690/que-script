#!/bin/bash
set -euo pipefail

RELEASE_BASE="https://github.com/AT-290690/que-script/releases/latest/download"
RAW_BASE="https://raw.githubusercontent.com/AT-290690/que-script/refs/heads/main/lisp"
LIB_DIR="/usr/local/share/que"
LIB_NAME="que-lib.lisp"
TMP_DIR="/tmp"

lib_url="$RELEASE_BASE/$LIB_NAME"
lib_tmp="$TMP_DIR/$LIB_NAME"
lib_path="$LIB_DIR/$LIB_NAME"

validate_bundle() {
  local file="$1"
  grep -q "(let std/vector/map " "$file" &&
    grep -q "(let map " "$file" &&
    grep -q "(let Table/new " "$file" &&
    grep -q "(let Set/new " "$file"
}

download_release_bundle() {
  echo "Installing $LIB_NAME (const + std + fp + ds bundle)..."
  curl -fsSL "$lib_url" -o "$lib_tmp"
}

build_bundle_from_sources() {
  echo "Falling back to source bundle from lisp/const.lisp + std.lisp + fp.lisp + ds.lisp..."
  : > "$lib_tmp"
  for part in const std fp ds; do
    curl -fsSL "$RAW_BASE/$part.lisp" >> "$lib_tmp"
    printf "\n" >> "$lib_tmp"
  done
}

install_bundle_file() {
  sudo mkdir -p "$LIB_DIR"
  sudo mv "$lib_tmp" "$lib_path"
  echo "Installed $lib_path"
}

if download_release_bundle && validate_bundle "$lib_tmp"; then
  install_bundle_file
  echo "Done."
  exit 0
fi

if build_bundle_from_sources && validate_bundle "$lib_tmp"; then
  install_bundle_file
  echo "Done."
  exit 0
fi

echo "Failed to install a valid que library bundle."
echo "Tried release asset: $lib_url"
echo "Tried source files from: $RAW_BASE/{const,std,fp,ds}.lisp"
echo "Check network/repo availability."
exit 1
