#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

OUT_DIR="${ROOT_DIR}/dist"

cargo build --release \
  --target wasm32-unknown-unknown \
  --lib \
  --no-default-features \
  --features compiler

mkdir -p "${OUT_DIR}"

wasm-bindgen \
  --target web \
  --out-dir "${OUT_DIR}" \
  --out-name que \
  target/wasm32-unknown-unknown/release/que.wasm

echo "Built browser compiler:"
echo "  ${OUT_DIR}/que.js"
echo "  ${OUT_DIR}/que_bg.wasm"
