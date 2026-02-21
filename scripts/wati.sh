#!/bin/bash
SRC="${1:-./example/main.lisp}"
DST="${2:-./example/dist/main.wat}"
./target/release/fez-rs --wati --s "$SRC" --d "$DST"
