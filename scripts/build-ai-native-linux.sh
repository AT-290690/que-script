#!/bin/sh
set -eu

# Package the optional Linux wasm2c path separately so the normal AI runner
# remains portable and small. Unlike the compiler itself, wasm2c is a native
# executable that must be Linux x86_64 inside the archive. Downloading the
# pinned official Linux WABT release makes this reproducible from macOS too.
# Set WABT_LINUX_ROOT to an already-extracted matching Linux WABT release to
# avoid the download (useful for offline release builds).
WABT_VERSION="1.0.42"
WABT_ARCHIVE="wabt-${WABT_VERSION}-linux-x64.tar.gz"
WABT_URL="https://github.com/WebAssembly/wabt/releases/download/${WABT_VERSION}/${WABT_ARCHIVE}"
WABT_SHA256="84895407a6bbb80e918f33b16b2fb2206021c150b6bc9ff6f761263a745ab131"

root_dir="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
ai_dir="$root_dir/miscs/ai"
out_dir="$root_dir/dist"
stage_dir="$(mktemp -d "${TMPDIR:-/tmp}/calque-ai-native.XXXXXX")"
trap 'rm -rf "$stage_dir"' EXIT HUP INT TERM

if [ -n "${WABT_LINUX_ROOT:-}" ]; then
  wabt_root="$WABT_LINUX_ROOT"
else
  archive="$stage_dir/$WABT_ARCHIVE"
  echo "Downloading pinned Linux wasm2c from WABT ${WABT_VERSION}..."
  curl -fsSL "$WABT_URL" -o "$archive"
  actual_sha="$(shasum -a 256 "$archive" | awk '{print $1}')"
  if [ "$actual_sha" != "$WABT_SHA256" ]; then
    echo "error: WABT archive checksum mismatch" >&2
    exit 1
  fi
  tar -xzf "$archive" -C "$stage_dir"
  wabt_root="$stage_dir/wabt-${WABT_VERSION}"
fi

wasm2c_bin="$wabt_root/bin/wasm2c"
runtime_dir="$wabt_root/share/wabt/wasm2c"
include_dir="$wabt_root/include"
if [ ! -f "$runtime_dir/wasm-rt-impl.c" ] || [ ! -f "$include_dir/wasm-rt.h" ]; then
  echo "error: could not find wasm2c and its runtime under $wabt_root" >&2
  exit 1
fi

"$root_dir/scripts/build-ai-runner.sh" >/dev/null

bundle_dir="$stage_dir/calque-ai-native-linux-x86_64"
mkdir -p "$bundle_dir/assets" "$bundle_dir/vendor/binaryen" \
  "$bundle_dir/bin" "$bundle_dir/native-c" "$bundle_dir/wasm2c/include" "$bundle_dir/wasm2c/runtime"
cp "$ai_dir/runner/calque-ai.mjs" "$bundle_dir/calque-ai.mjs"
cp "$ai_dir/native/calque-ai-native.sh" "$bundle_dir/calque-ai-native.sh"
cp "$ai_dir/runner/README.md" "$bundle_dir/README.md"
cp "$root_dir/dist/que_bg.wasm" "$bundle_dir/assets/que_bg.wasm"
cp "$ai_dir/node_modules/binaryen/index.js" "$bundle_dir/vendor/binaryen/index.js"
cp "$wasm2c_bin" "$bundle_dir/bin/wasm2c"
cp "$root_dir/miscs/native-c/que_host.c" "$root_dir/miscs/native-c/que_host.h" "$bundle_dir/native-c/"
cp "$include_dir/wasm-rt.h" "$bundle_dir/wasm2c/include/"
# wasm-rt-impl.c includes companion headers and .inc files whose exact set
# varies by WABT version, so copy the complete runtime directory.
cp "$runtime_dir"/* "$bundle_dir/wasm2c/runtime/"
chmod 755 "$bundle_dir/calque-ai.mjs" "$bundle_dir/calque-ai-native.sh" "$bundle_dir/bin/wasm2c"

mkdir -p "$out_dir"
(cd "$stage_dir" && zip -q -r "$out_dir/calque-ai-native-linux-x86_64.zip" calque-ai-native-linux-x86_64)
echo "$out_dir/calque-ai-native-linux-x86_64.zip"
