#!/bin/bash
SRC="${1:-./example/main.lisp}"
DST="${2:-./example/dist/main.wat}"
BASE="${DST%.wat}"

rm -f "$DST" "$BASE.wasm"
./target/release/fez-rs --wat --s "$SRC" --d "$DST" || exit 1
if [ ! -f "$DST" ]; then
  echo "WAT generation failed; no output file was produced."
  exit 1
fi

if ! command -v wat2wasm >/dev/null 2>&1; then
  echo "wat2wasm not found. Install wabt to run WAT (generation already succeeded)."
  exit 0
fi

if ! command -v wasmtime >/dev/null 2>&1; then
  echo "wasmtime not found. Install wasmtime to execute WASM (compilation already succeeded)."
  wat2wasm "$DST" -o "$BASE.wasm"
  exit 0
fi

wat2wasm "$DST" -o "$BASE.wasm" || exit 1

OUT=""
STATUS=0

OUT="$(wasmtime --invoke=main "$BASE.wasm" 2>&1)"
STATUS=$?

if [ $STATUS -ne 0 ]; then
  OUT="$(wasmtime --invoke main "$BASE.wasm" 2>&1)"
  STATUS=$?
fi

if [ $STATUS -ne 0 ]; then
  OUT="$(wasmtime run --invoke main "$BASE.wasm" 2>&1)"
  STATUS=$?
fi

if [ $STATUS -ne 0 ]; then
  echo "runtime error: $OUT"
  exit $STATUS
fi

CLEAN="$(printf "%s\n" "$OUT" | sed '/^warning: using `--invoke` with a function that returns values is experimental and may break in the future$/d' | sed '/^$/d')"

if [ -z "$CLEAN" ]; then
  echo "<ran ok, no cli return output>"
else
  RESULT="$(printf "%s\n" "$CLEAN" | tail -n 1)"
  echo "$RESULT"
fi
