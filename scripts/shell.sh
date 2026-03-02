#!/bin/bash
SRC="${1:-./example/main.lisp}"
DST="${2:-./example/dist/main.wat}"

rm -f "$DST"

./target/release/fez-rs --wat --s "$SRC" --d "$DST" || exit 1
if [ ! -f "$DST" ]; then
  echo "WAT generation failed; no output file was produced."
  exit 1
fi

OUT="$(./target/release/fez-rs "$SRC" "${@:3}" 2>&1)"
STATUS=$?

if [ $STATUS -ne 0 ]; then
  echo "Exception: $OUT"
  exit $STATUS
fi

printf "%s\n" "$OUT"
