#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXT_DIR="${ROOT_DIR}/miscs/extension/que-lang"
OUT_DIR="${ROOT_DIR}/releases"

cd "${EXT_DIR}"

if [[ ! -d node_modules ]]; then
  npm install
fi

mkdir -p "${OUT_DIR}"
VERSION="$(node -p "require('./package.json').version")"
npm run package -- --out "${OUT_DIR}/que-lang-${VERSION}.vsix"
