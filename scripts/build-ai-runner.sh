#!/bin/sh
set -eu

# Builds an uploadable, self-contained Node/Wasm runner. The archive is for
# temporary AI sandboxes: it carries its compiler and optimiser with it, but no
# native binaries and no permissions to access the host filesystem.

root_dir="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
ai_dir="$root_dir/miscs/ai"
out_dir="$root_dir/dist"
stage_dir="$(mktemp -d "${TMPDIR:-/tmp}/calque-ai-runner.XXXXXX")"
trap 'rm -rf "$stage_dir"' EXIT HUP INT TERM

command -v node >/dev/null 2>&1 || {
  echo "error: Node.js is required to build the AI runner" >&2
  exit 1
}

"$root_dir/scripts/build-wasm-c.sh"

if [ ! -f "$ai_dir/node_modules/binaryen/index.js" ]; then
  echo "Installing the pinned Binaryen build dependency..."
  npm --prefix "$ai_dir" ci --ignore-scripts --no-audit --no-fund
fi

bundle_dir="$stage_dir/calque-ai-runner"
mkdir -p "$bundle_dir/assets" "$bundle_dir/vendor/binaryen"
cp "$ai_dir/runner/calque-ai.mjs" "$bundle_dir/calque-ai.mjs"
cp "$ai_dir/runner/README.md" "$bundle_dir/README.md"
cp "$root_dir/dist/que_bg.wasm" "$bundle_dir/assets/que_bg.wasm"
cp "$ai_dir/node_modules/binaryen/index.js" "$bundle_dir/vendor/binaryen/index.js"
chmod 755 "$bundle_dir/calque-ai.mjs"

mkdir -p "$out_dir"
(cd "$stage_dir" && zip -q -r "$out_dir/calque-ai-runner.zip" calque-ai-runner)
echo "$out_dir/calque-ai-runner.zip"
