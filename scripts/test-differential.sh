#!/bin/sh
set -eu

script_dir="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
repo_dir="$(CDPATH= cd -- "$script_dir/.." && pwd)"
cd "$repo_dir"

for tool in wasmtime wasm2c; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "error: $tool is required for differential testing" >&2
    exit 1
  fi
done

if command -v clang >/dev/null 2>&1; then
  cc_bin=clang
elif command -v cc >/dev/null 2>&1; then
  cc_bin=cc
else
  echo "error: clang or cc is required for differential testing" >&2
  exit 1
fi

echo "Building Calque test compiler..."
cargo build --bin que

echo "Running deterministic debug/opt/wasm2c differential corpus..."
QUE_DIFFERENTIAL_QUE="$repo_dir/target/debug/que" \
  cargo test --test differential_targets -- --ignored

echo "Differential targets passed (wasmtime, wasm2c, $cc_bin)."
