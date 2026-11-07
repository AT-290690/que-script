#!/bin/bash
set -euo pipefail

REPO_BASE="${QUE_REPO_RAW:-https://raw.githubusercontent.com/AT-290690/que-script/main}"
EMACS_DIR="${QUE_EMACS_DIR:-$HOME/.emacs.d}"
INSTALL_ROOT="$EMACS_DIR/site-lisp/que"

download_file() {
  local source="$1"
  local destination="$2"
  mkdir -p "$(dirname "$destination")"
  curl -fsSL -H 'Cache-Control: no-cache' "$source" -o "$destination"
}

echo "Installing Calque Emacs support..."
echo "Target: $INSTALL_ROOT"

download_file "$REPO_BASE/miscs/emacs/que-mode.el" "$INSTALL_ROOT/que-mode.el"
download_file "$REPO_BASE/miscs/emacs/README.md" "$INSTALL_ROOT/README.md"

cat <<EOF
Installed que-mode to:
  $INSTALL_ROOT

Add this to your Emacs init file:

  (add-to-list 'load-path "$INSTALL_ROOT")
  (require 'que-mode)

Make sure quelsp is on PATH. Emacs 29.1+ provides Eglot out of the box.
EOF
