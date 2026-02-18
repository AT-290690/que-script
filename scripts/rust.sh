#!/bin/bash
SRC="${1:-./example/main.lisp}"
DST="${2:-./example/dist/main.rs}"
./target/release/fez-rs --rs --s "$SRC" --d "$DST"
