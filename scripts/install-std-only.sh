#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

echo "Note: install-std-only.sh is kept for compatibility."
echo "Installing full library bundle (const/std/fp/ds)..."
bash "$SCRIPT_DIR/install-lib-only.sh"
