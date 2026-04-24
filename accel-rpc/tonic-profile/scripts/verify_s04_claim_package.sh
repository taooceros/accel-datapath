#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
TONIC_PROFILE_DIR=$(cd -- "${SCRIPT_DIR}/.." && pwd)
ACCEL_RPC_DIR=$(cd -- "${TONIC_PROFILE_DIR}/.." && pwd)
REPO_ROOT=$(cd -- "${ACCEL_RPC_DIR}/.." && pwd)
MANIFEST_PATH="${TONIC_PROFILE_DIR}/workloads/s04_claim_package.json"
RUNNER_PATH="${TONIC_PROFILE_DIR}/scripts/run_s04_claim_package.py"
TONIC_BINARY="${ACCEL_RPC_DIR}/target/release/tonic-profile"
RUN_ROOT="${ACCEL_RPC_DIR}/target/s04-claim-package/latest"
SUMMARY_DIR="${RUN_ROOT}/summary"
SUMMARY_JSON="${SUMMARY_DIR}/comparison_summary.json"
SUMMARY_CSV="${SUMMARY_DIR}/ordinary_vs_idxd.csv"
CLAIM_TABLE="${SUMMARY_DIR}/claim_table.md"
DEVICE_PATH=${S03_ACCELERATOR_DEVICE:-<auto>}

printf '[verify_s04_claim_package] phase=manifest manifest=%s run_root=%s summary_path=%s device_path=%s\n' \
  "${MANIFEST_PATH}" "${RUN_ROOT}" "${SUMMARY_JSON}" "${DEVICE_PATH}"
python3 "${RUNNER_PATH}" --manifest "${MANIFEST_PATH}" --validate-only

printf '[verify_s04_claim_package] phase=build workspace=%s binary=%s\n' "${ACCEL_RPC_DIR}" "${TONIC_BINARY}"
(
  cd "${ACCEL_RPC_DIR}"
  cargo build --release -p tonic-profile
)

printf '[verify_s04_claim_package] phase=workflow run_root=%s summary_path=%s device_path=%s\n' \
  "${RUN_ROOT}" "${SUMMARY_JSON}" "${DEVICE_PATH}"
python3 "${RUNNER_PATH}" --manifest "${MANIFEST_PATH}"

printf '[verify_s04_claim_package] phase=done verdict=pass run_root=%s summary_path=%s csv_path=%s claim_table_path=%s device_path=%s\n' \
  "${RUN_ROOT}" "${SUMMARY_JSON}" "${SUMMARY_CSV}" "${CLAIM_TABLE}" "${DEVICE_PATH}"
