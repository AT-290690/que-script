#!/bin/bash
set -euo pipefail

REPO_BASE="https://raw.githubusercontent.com/AT-290690/que-script/main"
INSTALL_ROOT="${XDG_CONFIG_HOME:-$HOME/.config}/nvim/pack/que/start/que-nvim"

download_file() {
  local src="$1"
  local dest="$2"
  mkdir -p "$(dirname "$dest")"
  curl -fsSL "$src" -o "$dest"
}

echo "Installing que-nvim..."
echo "Target: $INSTALL_ROOT"

download_file \
  "$REPO_BASE/miscs/neovim/README.md" \
  "$INSTALL_ROOT/README.md"
download_file \
  "$REPO_BASE/miscs/neovim/ftdetect/que.lua" \
  "$INSTALL_ROOT/ftdetect/que.lua"
download_file \
  "$REPO_BASE/miscs/neovim/ftplugin/que.lua" \
  "$INSTALL_ROOT/ftplugin/que.lua"
download_file \
  "$REPO_BASE/miscs/neovim/lua/que/init.lua" \
  "$INSTALL_ROOT/lua/que/init.lua"
download_file \
  "$REPO_BASE/miscs/neovim/syntax/que.vim" \
  "$INSTALL_ROOT/syntax/que.vim"

cat <<EOF
Installed que-nvim to:
  $INSTALL_ROOT

Next steps:
  1. Make sure 'quelsp' is installed and on your PATH.
  2. Make sure 'nvim-lspconfig' is installed in Neovim.
  3. Add this to your Neovim config:

require("que").setup()
EOF
