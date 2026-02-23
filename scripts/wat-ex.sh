#!/bin/bash
SRC="${1:-./example/main.lisp}"
DST="${2:-./example/dist/main.wat}"

rm -f "$DST"

# Generate WAT (for inspection / tooling)
./target/release/fez-rs --wat --s "$SRC" --d "$DST" || exit 1
if [ ! -f "$DST" ]; then
  echo "WAT generation failed; no output file was produced."
  exit 1
fi

# Decode the wasm result via the CLI's --deref-wasm helper
OUT="$(./target/release/fez-rs --deref-wasm --s "$SRC" 2>&1)"
STATUS=$?

if [ $STATUS -ne 0 ]; then
  echo "runtime error: $OUT"
  exit $STATUS
fi

printf "%s\n" "$OUT"
