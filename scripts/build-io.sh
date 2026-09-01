#!/bin/bash
set -euo pipefail

cargo build --release --no-default-features --features shell-runtime --bin queio
