#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

PACKAGE="bench_uniffi"
TARGET_DIR="target"
DIST_DIR="dist/kotlin"

cargo build --lib --release

rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

cargo run --bin uniffi-bindgen generate \
  --library "${TARGET_DIR}/release/lib${PACKAGE}.dylib" \
  --language kotlin \
  --out-dir "$DIST_DIR"
