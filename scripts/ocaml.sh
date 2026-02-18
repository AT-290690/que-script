#!/bin/bash
SRC="${1:-./example/main.lisp}"
DST="${2:-./example/dist/main.ml}"
./target/release/fez-rs --ml --s "$SRC" --d "$DST"
