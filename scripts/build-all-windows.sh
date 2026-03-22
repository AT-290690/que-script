#!/bin/bash
set -euo pipefail

TARGET="${1:-x86_64-pc-windows-msvc}"
OUT_DIR="./target/${TARGET}/release"

cargo build --release --target "${TARGET}" --no-default-features --features io --bin queio
cargo build --release --target "${TARGET}" --bin quewat
cargo build --release --target "${TARGET}" --features io --bin quelsp

mkdir -p "${OUT_DIR}"
cp "${OUT_DIR}/queio.exe" "${OUT_DIR}/que.exe"

# The baked std lib is platform-independent text, so generating it with the host tool is sufficient.
cargo run --release --no-default-features --features repo-tools --bin quebake -- --out "${OUT_DIR}/que-lib.lisp"

printf 'Windows release artifacts written to %s\n' "${OUT_DIR}"
printf '  %s\n' "${OUT_DIR}/que.exe"
printf '  %s\n' "${OUT_DIR}/quewat.exe"
printf '  %s\n' "${OUT_DIR}/quelsp.exe"
printf '  %s\n' "${OUT_DIR}/que-lib.lisp"
