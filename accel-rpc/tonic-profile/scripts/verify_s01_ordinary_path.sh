#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
MANIFEST="$ROOT_DIR/accel-rpc/tonic-profile/workloads/s01_ordinary_matrix.json"
RUNNER="$ROOT_DIR/accel-rpc/tonic-profile/scripts/run_s01_workloads.py"
BINARY="$ROOT_DIR/accel-rpc/target/release/tonic-profile"
OUTPUT_DIR="$(mktemp -d "${TMPDIR:-/tmp}/tonic-profile-s01-ordinary.XXXXXX")"

printf 'S01 ordinary-path verifier output directory: %s\n' "$OUTPUT_DIR"
printf 'Building tonic-profile release binary...\n'
cargo build --release -p tonic-profile --manifest-path "$ROOT_DIR/accel-rpc/Cargo.toml"

printf 'Running curated workloads from %s...\n' "$MANIFEST"
python3 "$RUNNER" \
  --manifest "$MANIFEST" \
  --binary "$BINARY" \
  --output-dir "$OUTPUT_DIR"

printf 'Validating paired artifacts in %s...\n' "$OUTPUT_DIR"
python3 "$RUNNER" \
  --manifest "$MANIFEST" \
  --output-dir "$OUTPUT_DIR" \
  --verify-only

printf 'S01 ordinary-path verification passed. Inspect artifacts under %s\n' "$OUTPUT_DIR"
