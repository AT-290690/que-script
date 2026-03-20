#!/bin/bash
set -euo pipefail
SRC="${1:-./example/main.que}"
DST="${2:-./example/dist/main.js}"
./target/release/que-trs --js --s "$SRC" --d "$DST" \
  && node "$DST"
