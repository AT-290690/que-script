#!/bin/bash
set -euo pipefail

cargo build --release --no-default-features --features compiler --bin que
./scripts/build-lib.sh
./scripts/build-lsp.sh
