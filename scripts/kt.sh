#!/bin/bash
SRC="${1:-./example/main.lisp}"
DST="${2:-./example/dist/main.kt}"
./target/release/fez-rs --kt --s "$SRC" --d "$DST"
