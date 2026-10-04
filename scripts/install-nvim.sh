#!/bin/bash
set -euo pipefail

REPO_BASE="${QUE_REPO_RAW:-https://raw.githubusercontent.com/AT-290690/que-script/main}"
# Neovim's data pack path is present in distro, AppImage, and nightly builds.
INSTALL_ROOT="${XDG_DATA_HOME:-$HOME/.local/share}/nvim/site/pack/que/start/que-nvim"
NVIM_CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/nvim"
NVIM_INIT="$NVIM_CONFIG_DIR/init.lua"

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

mkdir -p "$NVIM_CONFIG_DIR"
managed_config=0
if [[ -f "$NVIM_INIT" ]] && grep -Fq 'Que workstation configuration installed by scripts/install-all-linux.sh' "$NVIM_INIT"; then
  managed_config=1
fi

if [[ ! -e "$NVIM_INIT" || "${QUE_OVERWRITE_NVIM:-0}" == "1" || "$managed_config" -eq 1 ]]; then
  download_file "$REPO_BASE/scripts/que-init.lua" "$NVIM_INIT"
  echo "Installed Que workstation config: $NVIM_INIT"
else
  echo "Kept existing Neovim config. Set QUE_OVERWRITE_NVIM=1 to replace it with the Que workstation config."
fi

cat <<EOF
Installed que-nvim to:
  $INSTALL_ROOT

The managed config includes Catppuccin, Telescope, Que completion labels,
and the two-space indentation settings. Make sure 'quelsp' is on PATH.
EOF
