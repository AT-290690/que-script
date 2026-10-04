#!/bin/bash
set -euo pipefail

if command -v wasmtime >/dev/null 2>&1; then
  echo "Wasmtime already installed: $(command -v wasmtime)"
  wasmtime --version
  exit 0
fi

echo "Installing the external Wasmtime CLI required by runtime tests..."
touch "$HOME/.profile"
curl --proto '=https' --tlsv1.2 -fsSL https://wasmtime.dev/install.sh \
  | PROFILE="$HOME/.profile" bash

wasmtime_bin="$HOME/.wasmtime/bin/wasmtime"
if [[ ! -x "$wasmtime_bin" ]]; then
  echo "Wasmtime installation completed, but $wasmtime_bin was not created." >&2
  exit 1
fi

"$wasmtime_bin" --version
echo 'Wasmtime was installed. Run `source "$HOME/.profile"` if it is not yet on PATH.'
