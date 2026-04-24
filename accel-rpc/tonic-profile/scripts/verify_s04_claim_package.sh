#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
TONIC_PROFILE_DIR=$(cd -- "${SCRIPT_DIR}/.." && pwd)
RUNNER_PATH="${TONIC_PROFILE_DIR}/scripts/run_s04_claim_package.py"
DEFAULT_MANIFEST_PATH="${TONIC_PROFILE_DIR}/workloads/s04_claim_package.json"

args=("$@")
has_manifest=false
for arg in "${args[@]}"; do
  if [[ "${arg}" == "--manifest" ]]; then
    has_manifest=true
    break
  fi
done

if [[ "${has_manifest}" == false ]]; then
  args=(--manifest "${DEFAULT_MANIFEST_PATH}" "${args[@]}")
fi

exec python3 "${RUNNER_PATH}" "${args[@]}"
