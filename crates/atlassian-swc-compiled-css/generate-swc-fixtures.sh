#!/bin/bash

# Script to emit out.js files from the SWC Rust transformer
# This script builds and runs the emit_fixtures binary

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "Building emit_fixtures binary..."
cargo build --bin emit_fixtures

echo "Running emit_fixtures..."
cargo run --bin emit_fixtures -- "$@"

npx prettier -w "tests/fixtures/*/{out,extract}.js"