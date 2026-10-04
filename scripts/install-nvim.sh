#!/bin/bash
set -euo pipefail

REPO_BASE="${QUE_REPO_RAW:-https://raw.githubusercontent.com/AT-290690/que-script/main}"
# Neovim's data pack path is present in distro, AppImage, and nightly builds.
INSTALL_ROOT="${XDG_DATA_HOME:-$HOME/.local/share}/nvim/site/pack/que/start/que-nvim"
NVIM_CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/nvim"
NVIM_INIT="$NVIM_CONFIG_DIR/init.lua"

clean_old_neovim() {
  [[ "${QUE_CLEAN_NVIM:-0}" == "1" ]] || return 0
  local nvim_data="${XDG_DATA_HOME:-$HOME/.local/share}/nvim"
  local nvim_state="${XDG_STATE_HOME:-$HOME/.local/state}/nvim"
  local nvim_cache="${XDG_CACHE_HOME:-$HOME/.cache}/nvim"
  rm -rf "$NVIM_CONFIG_DIR" "$nvim_data" "$nvim_state" "$nvim_cache"
  echo "Removed old Neovim config, plugins, state, and cache."
}

download_file() {
  local src="$1"
  local dest="$2"
  mkdir -p "$(dirname "$dest")"
  curl -fsSL -H 'Cache-Control: no-cache' "$src" -o "$dest"
}

run_nvim_checked() {
  local log status
  log="$(mktemp)"
  if nvim --headless "$@" >"$log" 2>&1; then
    status=0
  else
    status=$?
  fi
  cat "$log"
  if [[ "$status" -ne 0 ]] || grep -Eq '(^|[[:space:]])E[0-9]{3,4}:|Error (detected|in command line|in /)' "$log"; then
    rm -f "$log"
    echo "Neovim setup failed; the installation was not accepted as complete." >&2
    exit 1
  fi
  rm -f "$log"
}

echo "Installing que-nvim..."
echo "Target: $INSTALL_ROOT"

command -v nvim >/dev/null 2>&1 || {
  echo "Neovim is not installed. Run scripts/install-all-linux.sh on a fresh machine." >&2
  exit 1
}

clean_old_neovim

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
installed_managed_config=0
if [[ -f "$NVIM_INIT" ]] && grep -Fq 'Que workstation configuration installed by scripts/install-all-linux.sh' "$NVIM_INIT"; then
  managed_config=1
fi

if [[ ! -e "$NVIM_INIT" || "${QUE_OVERWRITE_NVIM:-0}" == "1" || "$managed_config" -eq 1 ]]; then
  download_file "$REPO_BASE/scripts/que-init.lua" "$NVIM_INIT"
  installed_managed_config=1
  echo "Installed Que workstation config: $NVIM_INIT"
else
  echo "Kept existing Neovim config. Set QUE_OVERWRITE_NVIM=1 to replace it with the Que workstation config."
fi

if [[ "$installed_managed_config" -eq 1 ]]; then
  echo "Installing Neovim plugins..."
  run_nvim_checked "+Lazy! sync" "+qa"

  echo "Verifying the installed Neovim experience..."
  run_nvim_checked \
    "+enew" \
    "+file que-install-check.que" \
    "+setfiletype que" \
    "+lua assert(vim.g.que_workstation_config == true, 'managed Que init.lua is not active')" \
    "+lua assert(vim.g.colors_name == 'catppuccin', 'Catppuccin is not active')" \
    "+lua assert(vim.o.tabstop == 2 and vim.o.shiftwidth == 2 and vim.o.softtabstop == 2 and vim.o.expandtab, 'Que tab settings are not active')" \
    "+lua local c=require('blink.cmp.config'); assert(c.appearance.kind_icons.Function == 'λ' and c.appearance.kind_icons.Keyword == 'β', 'Que completion label style is not active')" \
    "+lua assert(vim.fn.exists(':QueFormat') == 2, 'Que Neovim plugin commands are not active')" \
    "+qa"
fi

cat <<EOF
Installed que-nvim to:
  $INSTALL_ROOT

The managed config includes Catppuccin, Telescope, Que completion labels,
and the two-space indentation settings. Make sure 'quelsp' is on PATH.
EOF
