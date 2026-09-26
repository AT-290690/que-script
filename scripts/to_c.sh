#!/bin/sh
set -eu

# Backward-compatible entry point. The maintained wasm2c host lives in
# miscs/native-c and is wired by compile-native-c.sh.
script_dir="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
input_file="${1:-main.que}"
output_dir="${2:-build}"

exec "$script_dir/compile-native-c.sh" "$input_file" "$output_dir"
