#!/bin/bash
set -euo pipefail
SRC="${1:-./example/main.que}"
DST="${2:-./example/dist/main.ml}"
BASE="${DST%.ml}"
./target/release/que-trs --ml --s "$SRC" --d "$DST" \
  && ocamlopt -c "$DST" \
  && ocamlopt -o "$BASE.native" "$BASE.cmx" \
  && "$BASE.native"
