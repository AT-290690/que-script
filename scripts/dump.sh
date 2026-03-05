#!/bin/bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "Usage: ./scripts.sh <input.que> <output.wat>"
  exit 1
fi

INPUT_FILE="$1"
OUTPUT_FILE="$2"

cargo run --quiet --release --bin quewat -- "$INPUT_FILE" > "$OUTPUT_FILE"
echo "WAT written to $OUTPUT_FILE"
