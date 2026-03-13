#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

"$SCRIPT_DIR/install-minimal.sh"
"$SCRIPT_DIR/install-std-only.sh"

echo "Done (binary + std library installed)."
