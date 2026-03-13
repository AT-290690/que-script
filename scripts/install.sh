#!/bin/bash
set -euo pipefail

RAW_BASE="https://raw.githubusercontent.com/AT-290690/que-script/refs/heads/main/scripts"
SCRIPT_DIR=""

if [ -n "${BASH_SOURCE[0]-}" ] && [ -e "${BASH_SOURCE[0]}" ]; then
  SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
fi

run_step() {
  local step="$1"
  if [ -n "$SCRIPT_DIR" ] && [ -f "$SCRIPT_DIR/$step" ]; then
    bash "$SCRIPT_DIR/$step"
    return 0
  fi
  curl -fsSL "$RAW_BASE/$step" | bash
}

run_step "install-minimal.sh"
run_step "install-lib-only.sh"

echo "Done (binary + const/std/fp/ds library installed)."
