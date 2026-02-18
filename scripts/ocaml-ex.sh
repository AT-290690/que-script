#!/bin/bash
SRC="${1:-./example/main.lisp}"
DST="${2:-./example/dist/main.ml}"
BASE="${DST%.ml}"
./target/release/fez-rs --ml --s "$SRC" --d "$DST" \
  && ocamlopt -c "$DST" \
  && ocamlopt -o "$BASE.native" "$BASE.cmx" \
  && "$BASE.native"
