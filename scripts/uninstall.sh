#!/bin/bash
set -euo pipefail

BIN_DIR="/usr/local/bin"
BINARIES=("queio" "quec" "quer" "quelsp" "quewat", "que")

removed=0
for bin in "${BINARIES[@]}"; do
  target="$BIN_DIR/$bin"
  if [ -e "$target" ]; then
    sudo rm -f "$target"
    echo "Removed $target"
    removed=1
  fi
done

if [ "$removed" -eq 0 ]; then
  echo "No Que binaries found in $BIN_DIR"
fi
