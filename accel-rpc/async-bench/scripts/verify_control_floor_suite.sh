#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
ASYNC_BENCH_DIR=$(cd -- "${SCRIPT_DIR}/.." && pwd)
WORKSPACE_DIR=$(cd -- "${ASYNC_BENCH_DIR}/.." && pwd)
CRITERION_ROOT="${WORKSPACE_DIR}/target/criterion"
EXPORT_DIR="${WORKSPACE_DIR}/target/control-floor"
TMP_DIR=${TMPDIR:-/tmp}/async-bench-control-floor-$$
EXPECTED_BENCHMARKS=(
  tokio_spawn_join
  tokio_oneshot_completion
  tokio_mpsc_round_trip
  tokio_same_thread_wake
  tokio_cross_thread_wake
)

mkdir -p "${TMP_DIR}" "${EXPORT_DIR}"
SUMMARY_PATH="${EXPORT_DIR}/async_control_floor_summary.json"
MALFORMED_PATH="${TMP_DIR}/control_floor_summary.malformed.json"

cleanup() {
  rm -rf "${TMP_DIR}"
}
trap cleanup EXIT

expected_args=()
for benchmark in "${EXPECTED_BENCHMARKS[@]}"; do
  expected_args+=(--expected-benchmark "${benchmark}")
done

printf '[verify_control_floor_suite] phase=bench workspace=%s\n' "${WORKSPACE_DIR}"
(
  cd "${WORKSPACE_DIR}"
  cargo bench -p async-bench --bench async_overhead -- --warm-up-time 0.1 --measurement-time 0.1 --sample-size 10
)

printf '[verify_control_floor_suite] phase=export criterion_root=%s summary=%s\n' "${CRITERION_ROOT}" "${SUMMARY_PATH}"
python3 "${SCRIPT_DIR}/export_control_floor.py" \
  --criterion-root "${CRITERION_ROOT}" \
  --out "${SUMMARY_PATH}" \
  "${expected_args[@]}"

printf '[verify_control_floor_suite] phase=validate summary=%s\n' "${SUMMARY_PATH}"
python3 "${SCRIPT_DIR}/export_control_floor.py" \
  --validate-existing "${SUMMARY_PATH}" \
  "${expected_args[@]}"

printf '[verify_control_floor_suite] phase=negative-test case=missing-benchmark summary=%s\n' "${MALFORMED_PATH}"
python3 - <<'PY' "${SUMMARY_PATH}" "${MALFORMED_PATH}"
import json
import sys
from pathlib import Path

source = Path(sys.argv[1])
out = Path(sys.argv[2])
with source.open('r', encoding='utf-8') as handle:
    payload = json.load(handle)
payload['benchmarks'].pop('tokio_same_thread_wake', None)
with out.open('w', encoding='utf-8') as handle:
    json.dump(payload, handle, indent=2, sort_keys=True)
    handle.write('\n')
PY

if python3 "${SCRIPT_DIR}/export_control_floor.py" --validate-existing "${MALFORMED_PATH}" "${expected_args[@]}"; then
  echo "[verify_control_floor_suite] phase=negative-test verdict=unexpected-pass benchmark=tokio_same_thread_wake artifact=${MALFORMED_PATH}" >&2
  exit 1
else
  echo "[verify_control_floor_suite] phase=negative-test verdict=expected-fail benchmark=tokio_same_thread_wake artifact=${MALFORMED_PATH}"
fi

echo "[verify_control_floor_suite] phase=done verdict=pass artifact=${SUMMARY_PATH}"
