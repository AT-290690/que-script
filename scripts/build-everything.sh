#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

WINDOWS_TARGET="${1:-}"
LINUX_TARGET="${LINUX_TARGET:-x86_64-unknown-linux-gnu}"
HOST_TARGET="$(rustc -vV | sed -n 's/^host: //p')"

copy_release_artifacts() {
  local source_dir="$1"
  local target_suffix="$2"
  local exe_suffix="$3"

  cp "${source_dir}/que${exe_suffix}" "releases/que-${target_suffix}${exe_suffix}"
  cp "${source_dir}/quelsp${exe_suffix}" "releases/quelsp-${target_suffix}${exe_suffix}"
  cp "${source_dir}/que-lib.lisp" "releases/que-lib-${target_suffix}.lisp"
}

echo "[1/9] Baking language library"
./scripts/bake.sh

echo "[2/9] Building native artifacts"
./scripts/build-all.sh

echo "[3/9] Building Linux artifacts"
./scripts/build-all-linux.sh "${LINUX_TARGET}"

echo "[4/9] Building Windows artifacts"
if [[ -n "${WINDOWS_TARGET}" ]]; then
  ./scripts/build-all-windows.sh "${WINDOWS_TARGET}"
else
  ./scripts/build-all-windows.sh
fi

WINDOWS_TARGET="${WINDOWS_TARGET:-}"
if [[ -z "${WINDOWS_TARGET}" ]]; then
  HOST_OS="$(uname -s)"
  WINDOWS_TARGET="x86_64-pc-windows-msvc"
  if [[ "${HOST_OS}" != "MINGW"* && "${HOST_OS}" != "MSYS"* && "${HOST_OS}" != "CYGWIN"* ]]; then
    WINDOWS_TARGET="x86_64-pc-windows-gnu"
  fi
fi

echo "[5/9] Building wasm artifacts"
./scripts/build-wasm-c.sh

echo "[6/9] Building AI execution bundles"
./scripts/build-ai-native-linux.sh

echo "[7/9] Collecting release artifacts"
rm -rf releases
mkdir -p releases

copy_release_artifacts "./target/release" "${HOST_TARGET}" ""
copy_release_artifacts "./target/${LINUX_TARGET}/release" "${LINUX_TARGET}" ""
copy_release_artifacts "./target/${WINDOWS_TARGET}/release" "${WINDOWS_TARGET}" ".exe"
cp -R "miscs/neovim" "releases/que-nvim"
cp -R "miscs/emacs" "releases/que-emacs"
cp "dist/calque-ai-runner.zip" "releases/"
cp "dist/calque-ai-native-linux-x86_64.zip" "releases/"

echo "[8/9] Building VS Code extension"
./scripts/build-extension.sh

echo "[9/9] Editor integrations staged"

echo "All builds completed successfully."
echo "Release artifacts copied to ./releases"
