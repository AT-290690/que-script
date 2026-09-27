#!/bin/bash
set -euo pipefail

cargo build --release --no-default-features --features compiler --bin quewat
