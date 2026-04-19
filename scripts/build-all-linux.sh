#!/bin/bash
set -euo pipefail

TARGET="${1:-x86_64-unknown-linux-gnu}"
OUT_DIR="./target/${TARGET}/release"
TARGET_ENV_KEY="${TARGET//-/_}"
TARGET_ENV_KEY_UPPER="$(printf '%s' "${TARGET_ENV_KEY}" | tr '[:lower:]' '[:upper:]')"

if ! rustup target list --installed | grep -qx "${TARGET}"; then
  echo "Rust target '${TARGET}' is not installed." >&2
  echo "Install it with:" >&2
  echo "  rustup target add ${TARGET}" >&2
  exit 1
fi

if ! command -v zig >/dev/null 2>&1; then
  echo "zig is required for Linux cross-builds." >&2
  echo "Install it with:" >&2
  echo "  brew install zig" >&2
  exit 1
fi

if ! command -v cargo-zigbuild >/dev/null 2>&1; then
  echo "cargo-zigbuild is required for Linux cross-builds." >&2
  echo "Install it with:" >&2
  echo "  cargo install cargo-zigbuild" >&2
  exit 1
fi

LINUX_AR="$(command -v llvm-ar || command -v ar)"
if [[ -z "${LINUX_AR}" ]]; then
  echo "Could not find 'llvm-ar' or 'ar' on PATH." >&2
  exit 1
fi

# cargo-zigbuild's wrapped `ar` is unreliable for Wasmtime's helper archives when cross-building
# from macOS. Use the host archiver instead; archiving object files is target-agnostic here.
export "AR_${TARGET_ENV_KEY}=${LINUX_AR}"
export "CARGO_TARGET_${TARGET_ENV_KEY_UPPER}_AR=${LINUX_AR}"

cargo zigbuild --release --target "${TARGET}" --no-default-features --features io --bin queio
cargo zigbuild --release --target "${TARGET}" --no-default-features --features compiler --bin quec
cargo zigbuild --release --target "${TARGET}" --no-default-features --features runtime --bin quer
cargo zigbuild --release --target "${TARGET}" --bin quewat
cargo zigbuild --release --target "${TARGET}" --features io --bin quelsp

mkdir -p "${OUT_DIR}"
cp "${OUT_DIR}/queio" "${OUT_DIR}/que"

# The baked std lib is platform-independent text, so generating it with the host tool is sufficient.
cargo run --release --no-default-features --features repo-tools --bin quebake -- --out "${OUT_DIR}/que-lib.lisp"

printf 'Linux release artifacts written to %s\n' "${OUT_DIR}"
printf '  %s\n' "${OUT_DIR}/que"
printf '  %s\n' "${OUT_DIR}/quec"
printf '  %s\n' "${OUT_DIR}/quer"
printf '  %s\n' "${OUT_DIR}/quewat"
printf '  %s\n' "${OUT_DIR}/quelsp"
printf '  %s\n' "${OUT_DIR}/que-lib.lisp"
