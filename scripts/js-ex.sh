#!/bin/bash
SRC="${1:-./example/main.lisp}"
DST="${2:-./example/dist/main.js}"
./target/release/fez-rs --js --s "$SRC" --d "$DST" && node "$DST"
