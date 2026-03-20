#!/bin/bash
set -euo pipefail
SRC="${1:-./example/main.que}"
DST="${2:-./example/dist/main.clj}"
./target/release/que-trs --clj --s "$SRC" --d "$DST" \
  && clojure -M -e "(load-file \"$DST\")"
