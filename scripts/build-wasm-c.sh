#!/bin/bash
set -euo pipefail

cargo build --release \
  --target wasm32-unknown-unknown \
  --lib \
  --no-default-features \
  --features compiler

mkdir -p dist

wasm-bindgen \
  --target web \
  --out-dir dist \
  --out-name quec \
  target/wasm32-unknown-unknown/release/que.wasm

# Convenience alias when consumers expect `quec.wasm` directly.
cp dist/quec_bg.wasm dist/quec.wasm
