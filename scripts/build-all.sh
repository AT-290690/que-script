#!/bin/bash
set -euo pipefail

cargo build --release --no-default-features --bin que
cargo build --release --no-default-features --features compiler --bin quec
./scripts/build-wat.sh
./scripts/build-lib.sh
./scripts/build-lsp.sh
