#!/bin/bash
SRC="${1:-./example/main.lisp}"
DST="${2:-./example/dist/main.kt}"
BASE="${DST%.kt}"
./target/release/fez-rs --kt --s "$SRC" --d "$DST" \
  && kotlinc "$DST" -include-runtime -d "$BASE.jar" \
  && java -jar "$BASE.jar"
