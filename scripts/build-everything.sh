#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

WINDOWS_TARGET="${1:-}"

echo "[1/4] Baking std library"
./scripts/bake.sh

echo "[2/4] Building native artifacts"
./scripts/build-all.sh

echo "[3/4] Building Windows artifacts"
if [[ -n "${WINDOWS_TARGET}" ]]; then
  ./scripts/build-all-windows.sh "${WINDOWS_TARGET}"
else
  ./scripts/build-all-windows.sh
fi

echo "[4/4] Building wasm artifacts"
./scripts/build-wasm-c.sh

echo "All builds completed successfully."
