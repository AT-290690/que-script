#!/bin/bash
set -euo pipefail

RAW_INSTALL_URL="https://raw.githubusercontent.com/AT-290690/que-script/refs/heads/main/scripts/install.sh"
SCRIPT_DIR=""

if [ -n "${BASH_SOURCE[0]-}" ] && [ -e "${BASH_SOURCE[0]}" ]; then
  SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
fi

if [ -n "$SCRIPT_DIR" ] && [ -f "$SCRIPT_DIR/install.sh" ]; then
  bash "$SCRIPT_DIR/install.sh"
else
  curl -fsSL "$RAW_INSTALL_URL" | bash
fi
