#!/bin/bash
set -euo pipefail

BIN_DIR="/usr/local/bin"
BINARIES=("queio" "quec" "quer" "quelsp" "quewat" "que")
LIB_PATH="/usr/local/share/que/que-lib.lisp"

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

if [ -e "$LIB_PATH" ]; then
  sudo rm -f "$LIB_PATH"
  echo "Removed $LIB_PATH"
fi

if [ -d "/usr/local/share/que" ] && [ -z "$(ls -A /usr/local/share/que)" ]; then
  sudo rmdir /usr/local/share/que
  echo "Removed /usr/local/share/que"
fi
