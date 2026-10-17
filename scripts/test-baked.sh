#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"

LIB_OUT="${QUE_TEST_LIB_OUT:-$ROOT_DIR/dist/que-lib.lisp}"

echo "Baking library to: $LIB_OUT"
cargo run --no-default-features --features repo-tools --bin quebake -- --out "$LIB_OUT"

echo "Smoke-testing the baked library through the compiler"
# `cargo test` deliberately uses the embedded source library, so it cannot
# verify that the generated external file contains current macro definitions.
# This compile-only check does, without requiring a WASI runtime.
QUE_LIB_PATH="$LIB_OUT" cargo run --quiet --no-default-features --features compiler --bin que -- \
  --eval '(do (&mut cell 1) (&alter! cell 2) (&get cell))' --emit types >/dev/null

echo "Running test suite"
cargo test "$@"
