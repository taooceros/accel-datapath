#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: tokio_memmove_sweep.sh [options]

Runs the idxd-rust Tokio memmove benchmark over message sizes and concurrency
levels. This is direct descriptor submission only: batch_n=1, no DSA BATCH
descriptor.

Options:
  --device PATH          DSA work queue path [default: IDXD_RUST_DSA_WQ or /dev/dsa/wq0.0]
  --message-sizes LIST   Comma-separated bytes [default: 64,256,1024,4096,16384,65536]
  --concurrency LIST     Comma-separated levels [default: 1,2,4,8,16,32,64,128]
  --total-bytes BYTES    Bytes per benchmark point [default: 1073741824]
  --threads N            Tokio runtime worker threads [default: binary default]
  --repeat N             Repetitions per point [default: 1]
  --output PATH          JSONL output path [default: docs/report/benchmarking/tokio_memmove_sweep_<timestamp>.jsonl]
  --verify               Verify destination bytes in the benchmark loop
  --no-build             Reuse existing target/release/tokio_memmove_bench
  -h, --help             Print this help

Example:
  IDXD_RUST_DSA_WQ=/dev/dsa/wq0.0 \
    ./idxd-rust/scripts/tokio_memmove_sweep.sh \
      --message-sizes 64,256,4096,65536 \
      --concurrency 1,8,64,128 \
      --repeat 3
EOF
}

repo_root=$(git rev-parse --show-toplevel)
device=${IDXD_RUST_DSA_WQ:-/dev/dsa/wq0.0}
message_sizes_csv="64,256,1024,4096,16384,65536"
concurrency_csv="1,2,4,8,16,32,64,128"
total_bytes=1073741824
threads=""
repeat=1
output=""
verify=0
build=1

while [[ $# -gt 0 ]]; do
  case "$1" in
    --device) device=$2; shift 2 ;;
    --message-sizes) message_sizes_csv=$2; shift 2 ;;
    --concurrency) concurrency_csv=$2; shift 2 ;;
    --total-bytes) total_bytes=$2; shift 2 ;;
    --threads) threads=$2; shift 2 ;;
    --repeat) repeat=$2; shift 2 ;;
    --output) output=$2; shift 2 ;;
    --verify) verify=1; shift ;;
    --no-build) build=0; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

if [[ -z "$output" ]]; then
  mkdir -p "$repo_root/docs/report/benchmarking"
  output="$repo_root/docs/report/benchmarking/tokio_memmove_sweep_$(date +%Y%m%d_%H%M%S).jsonl"
fi

IFS=',' read -r -a message_sizes <<< "$message_sizes_csv"
IFS=',' read -r -a concurrencies <<< "$concurrency_csv"

if [[ $build -eq 1 ]]; then
  cargo build --release --manifest-path "$repo_root/Cargo.toml" -p idxd-rust --bin tokio_memmove_bench
fi

binary="$repo_root/target/release/tokio_memmove_bench"
if [[ ! -x "$binary" ]]; then
  echo "missing benchmark binary: $binary" >&2
  exit 1
fi

printf '# tokio_memmove_sweep strategy=tokio_naive_direct_descriptor batch_n=1 device=%s total_bytes=%s repeat=%s\n' \
  "$device" "$total_bytes" "$repeat" | tee "$output"

for msg in "${message_sizes[@]}"; do
  for conc in "${concurrencies[@]}"; do
    for rep in $(seq 1 "$repeat"); do
      echo "[tokio_memmove_sweep] message_bytes=$msg concurrency=$conc repeat=$rep/$repeat" >&2
      cmd=(
        "$binary"
        --device "$device"
        --total-bytes "$total_bytes"
        --message-bytes "$msg"
        --concurrency "$conc"
        --json
      )
      if [[ -n "$threads" ]]; then
        cmd+=(--threads "$threads")
      fi
      if [[ $verify -eq 1 ]]; then
        cmd+=(--verify)
      fi
      run_output=$(IDXD_RUST_DSA_WQ="$device" launch "${cmd[@]}")
      printf '%s\n' "$run_output" >&2
      json_line=$(printf '%s\n' "$run_output" | grep '^{' | tail -n 1)
      if [[ -z "$json_line" ]]; then
        echo "benchmark did not emit JSON output" >&2
        exit 1
      fi
      printf '%s\n' "$json_line" | tee -a "$output"
    done
  done
done

echo "[tokio_memmove_sweep] wrote $output" >&2
