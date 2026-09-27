#!/bin/bash
set -euo pipefail

if command -v wasmtime >/dev/null 2>&1; then
  echo "Wasmtime already installed: $(command -v wasmtime)"
  wasmtime --version
  exit 0
fi

echo "Installing the external Wasmtime CLI required by runtime tests..."
curl https://wasmtime.dev/install.sh -sSf | bash

echo "Wasmtime was installed. Restart your shell if it is not yet on PATH."
