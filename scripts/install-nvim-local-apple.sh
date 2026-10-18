#!/bin/bash
set -euo pipefail

PLUGIN_SOURCE_DIR="./miscs/neovim"
INSTALL_ROOT="${XDG_DATA_HOME:-$HOME/.local/share}/nvim/site/pack/que/start/que-nvim"
# Older development installers placed this plugin under the config runtime.
# Neovim loads config packs before data packs, so a stale copy there silently
# shadows the freshly installed plugin below.
LEGACY_INSTALL_ROOT="${XDG_CONFIG_HOME:-$HOME/.config}/nvim/pack/que/start/que-nvim"

usage() {
  cat <<'EOF'
Usage: ./scripts/install-nvim-local-apple.sh

Installs the local Calque / Eclisp Neovim plugin from this checkout onto this macOS machine.

Installs:
  ~/.local/share/nvim/site/pack/que/start/que-nvim

Notes:
  - This copies files from ./miscs/neovim
  - It does not install quelsp
  - It does not install nvim-lspconfig
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [ "$(uname -s)" != "Darwin" ]; then
  echo "This local Neovim installer is intended for macOS only." >&2
  exit 1
fi

if [ ! -d "$PLUGIN_SOURCE_DIR" ]; then
  echo "Missing plugin source directory: ${PLUGIN_SOURCE_DIR}" >&2
  exit 1
fi

echo "Installing local que-nvim..."
echo "Source: ${PLUGIN_SOURCE_DIR}"
echo "Target: ${INSTALL_ROOT}"

if [ -d "$LEGACY_INSTALL_ROOT" ]; then
  echo "Removing legacy Calque plugin that would shadow the current install: $LEGACY_INSTALL_ROOT"
  rm -rf "$LEGACY_INSTALL_ROOT"
fi

mkdir -p "$INSTALL_ROOT"
rm -rf "$INSTALL_ROOT"/*
cp -R "$PLUGIN_SOURCE_DIR"/. "$INSTALL_ROOT"/

echo "Installed local Neovim plugin."
echo "Next:"
echo "  1. Make sure quelsp is on PATH"
echo "  2. Make sure nvim-lspconfig is installed"
echo '  3. Add require("que").setup() to your Neovim config'
