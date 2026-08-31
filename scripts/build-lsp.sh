#!/bin/bash
set -euo pipefail

# WebAssembly LSP module (exports from src/wasm_api.rs).
cargo build --release --target wasm32-unknown-unknown --lib --no-default-features --features io

mkdir -p dist
wasm-bindgen \
  target/wasm32-unknown-unknown/release/que.wasm \
  --out-dir ./dist \
  --target web \
  --out-name quelsp

# Native LSP binary for editor integration.
cargo build --release --no-default-features --features io --bin quelsp
