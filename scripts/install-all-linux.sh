#!/usr/bin/env bash
set -euo pipefail

# One-shot Linux bootstrap for a fresh SSH shell.
#
# Installs:
#   - que and quelsp
#   - wasmtime (the default external WASI runtime)
#   - Neovim, git, ripgrep and the tools needed by Telescope
#   - the Que Neovim plugin and a self-contained Telescope/LSP config

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "This installer is for Linux. Use the platform installer for $(uname -s)." >&2
  exit 1
fi

REPO_RAW="${QUE_REPO_RAW:-https://raw.githubusercontent.com/AT-290690/que-script/main}"
NVIM_CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/nvim"
NVIM_INIT="$NVIM_CONFIG_DIR/init.lua"
NVIM_INIT_URL="${QUE_NVIM_INIT_URL:-$REPO_RAW/scripts/que-init.lua}"

as_root() {
  if [[ "$(id -u)" -eq 0 ]]; then
    "$@"
  elif command -v sudo >/dev/null 2>&1; then
    sudo "$@"
  else
    echo "This step needs root privileges, but sudo is not installed: $*" >&2
    exit 1
  fi
}

install_packages() {
  local packages=(ca-certificates curl git ripgrep make gcc g++)
  if command -v apt-get >/dev/null 2>&1; then
    as_root apt-get update
    as_root env DEBIAN_FRONTEND=noninteractive apt-get install -y neovim "${packages[@]}"
  elif command -v dnf >/dev/null 2>&1; then
    as_root dnf install -y neovim "${packages[@]}"
  elif command -v yum >/dev/null 2>&1; then
    as_root yum install -y neovim "${packages[@]}"
  elif command -v pacman >/dev/null 2>&1; then
    as_root pacman -Sy --needed --noconfirm neovim "${packages[@]}"
  else
    echo "No supported package manager found. Install neovim, git, curl, ripgrep and a C build tool, then rerun." >&2
    exit 1
  fi
}

install_wasmtime() {
  if command -v wasmtime >/dev/null 2>&1; then
    return
  fi
  echo "Installing wasmtime (Que's default external WASI runtime)..."
  curl --proto '=https' --tlsv1.2 -fsSL https://wasmtime.dev/install.sh | bash
  local wasmtime_bin="$HOME/.wasmtime/bin"
  if [[ -x "$wasmtime_bin/wasmtime" ]]; then
    export PATH="$wasmtime_bin:$PATH"
    mkdir -p "$HOME/.config"
    touch "$HOME/.profile"
    if ! grep -Fq "$wasmtime_bin" "$HOME/.profile"; then
      printf '\n# Wasmtime installed for Que\nexport PATH="$HOME/.wasmtime/bin:$PATH"\n' >> "$HOME/.profile"
    fi
  fi
  command -v wasmtime >/dev/null 2>&1 || {
    echo "wasmtime was installed, but is not on PATH. Start a new shell and rerun." >&2
    exit 1
  }
}

download_and_run() {
  local name="$1"
  local tmp
  tmp="$(mktemp)"
  trap 'rm -f "$tmp"' RETURN
  curl -fsSL "$REPO_RAW/scripts/$name" -o "$tmp"
  bash "$tmp"
}

echo "Installing Linux dependencies..."
install_packages
install_wasmtime

echo "Installing Que and the language server..."
download_and_run install.sh
download_and_run lsp.sh
download_and_run install-nvim.sh

mkdir -p "$NVIM_CONFIG_DIR"
if [[ -e "$NVIM_INIT" && "${QUE_OVERWRITE_NVIM:-0}" != "1" ]]; then
  backup="$NVIM_INIT.que-backup-$(date +%Y%m%d%H%M%S)"
  cp "$NVIM_INIT" "$backup"
  echo "Existing Neovim config preserved at $backup"
fi

if [[ ! -e "$NVIM_INIT" || "${QUE_OVERWRITE_NVIM:-0}" == "1" ]]; then
  curl -fsSL "$NVIM_INIT_URL" -o "$NVIM_INIT"
  echo "Installed Neovim config: $NVIM_INIT"
else
  echo "Kept existing Neovim config. Set QUE_OVERWRITE_NVIM=1 to install the Que config."
fi

echo
echo "Installation complete. Open a new shell (or run: source ~/.profile), then try:"
echo "  que --version"
echo "  nvim --version"
echo "  que nvim --allow all"
echo
echo "The first Neovim start downloads Telescope and the LSP plugins."
