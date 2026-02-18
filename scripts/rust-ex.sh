#!/bin/bash
SRC="${1:-./example/main.lisp}"
DST="${2:-./example/dist/main.rs}"
BASE="${DST%.rs}"
./target/release/fez-rs --rs --s "$SRC" --d "$DST" \
  && rustc "$DST" -C opt-level=3 -o "$BASE.native" \
  && "$BASE.native"
