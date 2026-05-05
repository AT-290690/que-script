#!/bin/bash
set -euo pipefail

./scripts/build-lib.sh

mkdir -p releases

copy_lib() {
  local target_dir="$1"
  local suffix="$2"

  if [[ -d "$target_dir" ]]; then
    cp ./target/release/que-lib.lisp "releases/que-lib-${suffix}.lisp"
    echo "wrote releases/que-lib-${suffix}.lisp"
  fi
}

copy_lib ./target/release "$(rustc -vV | sed -n 's/^host: //p')"
copy_lib ./target/x86_64-unknown-linux-gnu/release x86_64-unknown-linux-gnu
copy_lib ./target/x86_64-pc-windows-gnu/release x86_64-pc-windows-gnu
copy_lib ./target/x86_64-pc-windows-msvc/release x86_64-pc-windows-msvc

echo "lib-only release artifacts updated"