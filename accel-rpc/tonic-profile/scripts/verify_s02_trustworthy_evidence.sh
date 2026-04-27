#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
TONIC_PROFILE_DIR=$(cd -- "${SCRIPT_DIR}/.." && pwd)
ACCEL_RPC_DIR=$(cd -- "${TONIC_PROFILE_DIR}/.." && pwd)
REPO_ROOT=$(cd -- "${ACCEL_RPC_DIR}/.." && pwd)
MANIFEST_PATH="${TONIC_PROFILE_DIR}/workloads/s02_trustworthy_matrix.json"
RUNNER_PATH="${TONIC_PROFILE_DIR}/scripts/run_s02_evidence.py"
ASYNC_VERIFY_PATH="${ACCEL_RPC_DIR}/async-bench/scripts/verify_control_floor_suite.sh"
TONIC_BINARY="${ACCEL_RPC_DIR}/target/release/tonic-profile"
OUTPUT_DIR=${S02_OUTPUT_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/s02-trustworthy-evidence.XXXXXX")}
CONTROL_FLOOR_SUMMARY="${ACCEL_RPC_DIR}/target/control-floor/async_control_floor_summary.json"

printf '[verify_s02_trustworthy_evidence] phase=manifest manifest=%s output_dir=%s\n' "${MANIFEST_PATH}" "${OUTPUT_DIR}"
python3 "${RUNNER_PATH}" --manifest "${MANIFEST_PATH}" --validate-only

printf '[verify_s02_trustworthy_evidence] phase=build workspace=%s binary=%s\n' "${ACCEL_RPC_DIR}" "${TONIC_BINARY}"
(
  cd "${ACCEL_RPC_DIR}"
  cargo build --release -p tonic-profile
)

printf '[verify_s02_trustworthy_evidence] phase=tonic-profile output_dir=%s\n' "${OUTPUT_DIR}"
python3 "${RUNNER_PATH}" \
  --manifest "${MANIFEST_PATH}" \
  --binary "${TONIC_BINARY}" \
  --output-dir "${OUTPUT_DIR}"

printf '[verify_s02_trustworthy_evidence] phase=async-bench workspace=%s\n' "${ACCEL_RPC_DIR}"
bash "${ASYNC_VERIFY_PATH}"

printf '[verify_s02_trustworthy_evidence] phase=control-floor-validation summary=%s\n' "${CONTROL_FLOOR_SUMMARY}"
python3 - <<'PY' "${MANIFEST_PATH}" "${CONTROL_FLOOR_SUMMARY}"
import json
import sys
from pathlib import Path

manifest_path = Path(sys.argv[1])
summary_path = Path(sys.argv[2])

with manifest_path.open('r', encoding='utf-8') as handle:
    manifest = json.load(handle)
expected = manifest.get('expected_benchmarks')
if not isinstance(expected, list) or not expected:
    raise SystemExit(
        f"phase=control-floor-validation benchmark=<manifest> artifact={manifest_path} missing expected_benchmarks"
    )

with summary_path.open('r', encoding='utf-8') as handle:
    summary = json.load(handle)
benchmarks = summary.get('benchmarks')
if not isinstance(benchmarks, dict):
    raise SystemExit(
        f"phase=control-floor-validation benchmark=<summary> artifact={summary_path} malformed benchmarks object"
    )

for benchmark in expected:
    if benchmark not in benchmarks:
        raise SystemExit(
            f"phase=control-floor-validation benchmark={benchmark} artifact={summary_path} missing benchmark"
        )
    print(
        f"phase=control-floor-validation benchmark={benchmark} artifact={summary_path} verdict=pass"
    )
PY

printf '[verify_s02_trustworthy_evidence] phase=done verdict=pass output_dir=%s control_floor_summary=%s\n' "${OUTPUT_DIR}" "${CONTROL_FLOOR_SUMMARY}"
