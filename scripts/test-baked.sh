#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"

LIB_OUT="${QUE_TEST_LIB_OUT:-$ROOT_DIR/dist/que-lib.lisp}"

echo "Baking library to: $LIB_OUT"
cargo run --no-default-features --features repo-tools --bin quebake -- --out "$LIB_OUT"

echo "Running tests with QUE_LIB_PATH=$LIB_OUT"
QUE_LIB_PATH="$LIB_OUT" cargo test "$@"
