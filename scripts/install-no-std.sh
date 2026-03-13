#!/bin/bash
set -euo pipefail

REPO="https://github.com/AT-290690/que-script/releases/latest/download"
BIN_DIR="/usr/local/bin"
TMP_DIR="/tmp"
SUPPORTED=("que" "quec" "quer" "queio" "quelsp" "quewat")
DEFAULT=("quec" "quer" "queio" "quelsp" "quewat")

is_supported() {
  local candidate="$1"
  for item in "${SUPPORTED[@]}"; do
    if [ "$item" = "$candidate" ]; then
      return 0
    fi
  done
  return 1
}

if [ "$#" -eq 0 ] || [ "${1:-}" = "all" ]; then
  TARGETS=("${DEFAULT[@]}")
else
  TARGETS=("$@")
fi

for app in "${TARGETS[@]}"; do
  if ! is_supported "$app"; then
    echo "Unsupported binary: $app"
    echo "Supported: ${SUPPORTED[*]}"
    exit 1
  fi

  url="$REPO/$app"
  tmp_path="$TMP_DIR/$app"
  install_path="$BIN_DIR/$app"

  echo "Installing $app..."
  if curl -fsSL "$url" -o "$tmp_path"; then
    chmod +x "$tmp_path"
    sudo mv "$tmp_path" "$install_path"
    echo "Installed $install_path"
  else
    echo "Failed to download $url"
    echo "Check releases: https://github.com/AT-290690/que-script/releases"
    exit 1
  fi
done

echo "Done (no std library installed)."
