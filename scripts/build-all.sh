#!/bin/bash
set -euo pipefail

./scripts/build-io.sh
cargo build --release --no-default-features --features compiler --bin quec
cargo build --release --no-default-features --features runtime --bin quer
./scripts/build-wat.sh
./scripts/build-lib.sh
./scripts/build-lsp.sh

cp ./target/release/queio ./target/release/que
