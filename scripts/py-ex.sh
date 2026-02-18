#!/bin/bash
SRC="${1:-./example/main.lisp}"
DST="${2:-./example/dist/main.py}"
./target/release/fez-rs --py --s "$SRC" --d "$DST" && python3 "$DST"
