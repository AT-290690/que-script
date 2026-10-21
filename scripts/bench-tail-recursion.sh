#!/usr/bin/env bash
set -euo pipefail

# Compare Calque's direct self-tail-recursion lowerings through the normal
# Wasmtime runner. `return-call` is retained only as a regression baseline;
# `loop` is the default lowering and also works with wasm2c.

runs="${1:-15}"
script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
repo_dir="$(CDPATH= cd -- "$script_dir/.." && pwd)"
source_file="$repo_dir/tests/bench/tail-recursion.que"

if [[ -n "${QUE_BIN:-}" ]]; then
  que_bin="$QUE_BIN"
elif [[ -x "$repo_dir/target/release/que" ]]; then
  que_bin="$repo_dir/target/release/que"
elif command -v que >/dev/null 2>&1; then
  que_bin="$(command -v que)"
else
  echo "error: build Calque first or set QUE_BIN=/path/to/que" >&2
  exit 1
fi

python3 - "$que_bin" "$source_file" "$runs" <<'PY'
import os
import statistics
import subprocess
import sys
import time

que_bin, source_file, raw_runs = sys.argv[1:]
runs = int(raw_runs)
if runs < 3:
    raise SystemExit("error: use at least 3 measured runs")

print(f"Tail recursion: {runs} runs per lowering")
for mode in ("return-call", "loop"):
    env = os.environ | {
        "QUE_TCO": "aggressive",
        "QUE_SELF_TCO": mode,
    }
    samples = []
    output = None
    # Warm up compilation/JIT once, then measure the same whole-program path.
    subprocess.run([que_bin, source_file, "--opt"], env=env, check=True,
                   stdout=subprocess.PIPE, text=True)
    for _ in range(runs):
        started = time.perf_counter_ns()
        result = subprocess.run([que_bin, source_file, "--opt"], env=env,
                                check=True, stdout=subprocess.PIPE, text=True)
        samples.append((time.perf_counter_ns() - started) / 1_000_000)
        value = result.stdout.strip()
        if output is None:
            output = value
        elif value != output:
            raise SystemExit(f"error: {mode} changed output: {value!r} != {output!r}")
    print(f"{mode:12} median {statistics.median(samples):8.3f} ms  result {output}")
PY
