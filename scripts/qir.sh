#!/bin/bash
SRC="${1:-./example/main.lisp}"
DST="${2:-./example/dist/main.qir}"
./target/release/fez-rs --qir --s "$SRC" --d "$DST"
