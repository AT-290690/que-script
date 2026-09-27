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
  --out-name que \
  target/wasm32-unknown-unknown/release/que.wasm
