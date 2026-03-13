#!/bin/bash
set -euo pipefail

cargo run --release --no-default-features --features repo-tools --bin quebake -- --out ./target/release/que-lib.lisp
