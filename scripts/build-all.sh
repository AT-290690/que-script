#!/bin/bash
set -euo pipefail

./scripts/build-io.sh
./scripts/build-wat.sh
./scripts/build-lib.sh
./scripts/build-lsp.sh

cp ./target/release/queio ./target/release/que
