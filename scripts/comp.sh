#!/bin/bash
SRC="${1:-./example/main.lisp}"
DST="${2:-./example/dist/main.txt}"
./target/release/fez-rs --comp --s "$SRC" --d "$DST"
