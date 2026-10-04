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

clean_old_neovim() {
  [[ "${QUE_CLEAN_NVIM:-0}" == "1" ]] || return
  local nvim_data="${XDG_DATA_HOME:-$HOME/.local/share}/nvim"
  local nvim_state="${XDG_STATE_HOME:-$HOME/.local/state}/nvim"
  local nvim_cache="${XDG_CACHE_HOME:-$HOME/.cache}/nvim"
  rm -rf "$NVIM_CONFIG_DIR" "$nvim_data" "$nvim_state" "$nvim_cache"
  mkdir -p "$NVIM_CONFIG_DIR"
  echo "Removed old Neovim config, plugins, state, and cache."
}

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
  local packages=()
  [[ -f /etc/ssl/certs/ca-certificates.crt ]] || packages+=(ca-certificates)
  command -v curl >/dev/null 2>&1 || packages+=(curl)
  command -v git >/dev/null 2>&1 || packages+=(git)
  command -v rg >/dev/null 2>&1 || packages+=(ripgrep)
  command -v make >/dev/null 2>&1 || packages+=(make)
  command -v gcc >/dev/null 2>&1 || packages+=(gcc)
  command -v g++ >/dev/null 2>&1 || packages+=(g++)

  if [[ ${#packages[@]} -eq 0 ]]; then
    echo "Linux dependencies are already installed."
    return
  fi

  echo "Installing missing packages: ${packages[*]}"
  if command -v apt-get >/dev/null 2>&1; then
    local apt_updated=0
    local package
    for package in "${packages[@]}"; do
      if ! as_root env DEBIAN_FRONTEND=noninteractive apt-get \
        -o APT::Install-Recommends=false \
        -o APT::Install-Suggests=false \
        install -y "$package"; then
        if [[ "$apt_updated" -eq 0 ]]; then
          as_root apt-get -o Acquire::Languages=none update
          apt_updated=1
        fi
        as_root env DEBIAN_FRONTEND=noninteractive apt-get \
          -o APT::Install-Recommends=false \
          -o APT::Install-Suggests=false \
          install -y "$package"
      fi
    done
  elif command -v dnf >/dev/null 2>&1; then
    as_root dnf install -y "${packages[@]}"
  elif command -v yum >/dev/null 2>&1; then
    as_root yum install -y "${packages[@]}"
  elif command -v pacman >/dev/null 2>&1; then
    as_root pacman -Sy --needed --noconfirm "${packages[@]}"
  else
    echo "No supported package manager found. Install neovim, git, curl, ripgrep and a C build tool, then rerun." >&2
    exit 1
  fi
}

ensure_modern_neovim() {
  local minor=0
  if command -v nvim >/dev/null 2>&1; then
    minor="$(nvim --clean --headless +'lua io.write(vim.version().minor)' +qa 2>/dev/null || echo 0)"
  fi

  # The supplied config uses vim.pack/PackChanged.  Those APIs require the
  # current Neovim development release; distro packages are often much older.
  if [[ "$minor" =~ ^[0-9]+$ ]] && (( minor >= 12 )); then
    return
  fi

  local arch archive_dir install_dir
  case "$(uname -m)" in
    x86_64|amd64) arch="x86_64" ;;
    aarch64|arm64) arch="arm64" ;;
    *) echo "Unsupported Linux architecture for Neovim: $(uname -m)" >&2; exit 1 ;;
  esac

  archive_dir="$HOME/.local/opt"
  install_dir="$archive_dir/nvim-linux-$arch"
  mkdir -p "$archive_dir" "$HOME/.local/bin"
  echo "Installing modern Neovim (the distro package is too old)..."
  curl --proto '=https' --tlsv1.2 -fsSL \
    "https://github.com/neovim/neovim/releases/download/nightly/nvim-linux-$arch.tar.gz" \
    | tar -xzf - -C "$archive_dir"
  ln -sf "$install_dir/bin/nvim" "$HOME/.local/bin/nvim"
  # Make the upgraded editor visible to commands launched after this script
  # exits, without requiring the user to modify PATH manually.
  as_root ln -sf "$install_dir/bin/nvim" /usr/local/bin/nvim
  export PATH="$HOME/.local/bin:$PATH"
  touch "$HOME/.profile"
  if ! grep -Fq 'HOME/.local/bin' "$HOME/.profile"; then
    printf '\n# User-installed tools\nexport PATH="$HOME/.local/bin:$PATH"\n' >> "$HOME/.profile"
  fi

  command -v nvim >/dev/null 2>&1 || {
    echo "Neovim installation completed, but nvim is not on PATH." >&2
    exit 1
  }
}

install_wasmtime() {
  if command -v wasmtime >/dev/null 2>&1; then
    return
  fi
  echo "Installing wasmtime (Que's default external WASI runtime)..."
  # Fresh cloud images sometimes have no shell startup file. Wasmtime's
  # installer expects one when it records WASMTIME_HOME.
  touch "$HOME/.profile"
  curl --proto '=https' --tlsv1.2 -fsSL https://wasmtime.dev/install.sh | bash
  local wasmtime_bin="$HOME/.wasmtime/bin"
  if [[ -x "$wasmtime_bin/wasmtime" ]]; then
    export PATH="$wasmtime_bin:$PATH"
    as_root ln -sf "$wasmtime_bin/wasmtime" /usr/local/bin/wasmtime
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

install_que_plugin() {
  local root="$HOME/.local/share/nvim/site/pack/que/start/que-nvim"
  local files=(
    README.md
    ftdetect/que.lua
    ftplugin/que.lua
    lua/que/init.lua
    syntax/que.vim
  )
  echo "Installing Que's Neovim plugin..."
  for file in "${files[@]}"; do
    mkdir -p "$root/$(dirname "$file")"
    curl -fsSL "$REPO_RAW/miscs/neovim/$file" -o "$root/$file"
  done
}

echo "Installing Linux dependencies..."
install_packages
ensure_modern_neovim
install_wasmtime

echo "Installing Que and the language server..."
download_and_run install.sh
download_and_run lsp.sh
clean_old_neovim
install_que_plugin

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

echo "Installing Neovim plugins..."
nvim --headless "+Lazy! sync" "+qa"

test -x /usr/local/bin/que
test -x /usr/local/bin/quelsp
test -f "$HOME/.local/share/nvim/site/pack/que/start/que-nvim/lua/que/init.lua"

echo
echo "Installation complete. Everything needed by the Que Neovim environment is installed."
echo "  que --version"
echo "  nvim --version"
echo "  que nvim --allow all"
