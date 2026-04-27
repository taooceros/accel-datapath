#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
TONIC_PROFILE_DIR=$(cd -- "${SCRIPT_DIR}/.." && pwd)
ACCEL_RPC_DIR=$(cd -- "${TONIC_PROFILE_DIR}/.." && pwd)
REPO_ROOT=$(cd -- "${ACCEL_RPC_DIR}/.." && pwd)
MANIFEST_PATH="${TONIC_PROFILE_DIR}/workloads/s03_idxd_matrix.json"
RUNNER_PATH="${TONIC_PROFILE_DIR}/scripts/run_s03_idxd_evidence.py"
TONIC_BINARY="${ACCEL_RPC_DIR}/target/release/tonic-profile"
OUTPUT_DIR=${S03_OUTPUT_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/s03-idxd-evidence.XXXXXX")}
PREFLIGHT_ARTIFACT="${OUTPUT_DIR}/preflight.selftest.json"
PREFLIGHT_TIMEOUT=${S03_PREFLIGHT_TIMEOUT:-20s}
RUN_TIMEOUT=${S03_RUN_TIMEOUT:-60s}

find_default_device() {
  if [[ -n "${S03_ACCELERATOR_DEVICE:-}" ]]; then
    printf '%s\n' "${S03_ACCELERATOR_DEVICE}"
    return 0
  fi

  local candidate
  for candidate in /dev/dsa/wq*; do
    if [[ -e "${candidate}" ]]; then
      printf '%s\n' "${candidate}"
      return 0
    fi
  done

  return 1
}

fail_phase() {
  local phase=$1
  shift
  printf '[verify_s03_idxd_path] phase=%s output_dir=%s %s\n' "${phase}" "${OUTPUT_DIR}" "$*" >&2
  exit 1
}

ACCELERATOR_DEVICE=$(find_default_device) || fail_phase preflight 'device_path=<none> launcher_status=missing_work_queue message=no /dev/dsa/wq* device found; set S03_ACCELERATOR_DEVICE explicitly'

printf '[verify_s03_idxd_path] phase=manifest manifest=%s output_dir=%s device_path=%s\n' "${MANIFEST_PATH}" "${OUTPUT_DIR}" "${ACCELERATOR_DEVICE}"
python3 "${RUNNER_PATH}" --manifest "${MANIFEST_PATH}" --validate-only

command -v devenv >/dev/null 2>&1 || fail_phase preflight "device_path=${ACCELERATOR_DEVICE} launcher_status=missing_devenv message=devenv command not found"

printf '[verify_s03_idxd_path] phase=build workspace=%s binary=%s\n' "${ACCEL_RPC_DIR}" "${TONIC_BINARY}"
(
  cd "${ACCEL_RPC_DIR}"
  cargo build --release -p tonic-profile
)

LAUNCHER_PATH="${REPO_ROOT}/tools/build/dsa_launcher"
if [[ ! -x "${LAUNCHER_PATH}" ]]; then
  fail_phase preflight "device_path=${ACCELERATOR_DEVICE} launcher_status=missing_launcher launcher_path=${LAUNCHER_PATH} message=build the launcher with launch in a privileged shell before running this verifier"
fi
if command -v getcap >/dev/null 2>&1; then
  LAUNCHER_CAPS=$(getcap "${LAUNCHER_PATH}" || true)
  if [[ "${LAUNCHER_CAPS}" != *"cap_sys_rawio"* ]]; then
    fail_phase preflight "device_path=${ACCELERATOR_DEVICE} launcher_status=missing_capability launcher_path=${LAUNCHER_PATH} message=launcher lacks cap_sys_rawio+eip"
  fi
fi

printf '[verify_s03_idxd_path] phase=preflight binary=%s artifact=%s device_path=%s launcher=launch timeout=%s\n' "${TONIC_BINARY}" "${PREFLIGHT_ARTIFACT}" "${ACCELERATOR_DEVICE}" "${PREFLIGHT_TIMEOUT}"
if ! timeout "${PREFLIGHT_TIMEOUT}" \
  devenv shell -- launch "${TONIC_BINARY}" \
    --mode selftest \
    --bind 127.0.0.1:50171 \
    --target 127.0.0.1:50171 \
    --payload-size 64 \
    --payload-kind repeated \
    --warmup-ms 0 \
    --measure-ms 20 \
    --requests 1 \
    --instrumentation on \
    --accelerated-path idxd \
    --accelerator-device "${ACCELERATOR_DEVICE}" \
    --json-out "${PREFLIGHT_ARTIFACT}"; then
  fail_phase preflight "device_path=${ACCELERATOR_DEVICE} launcher_status=failed artifact=${PREFLIGHT_ARTIFACT} message=launch-wrapped selftest failed"
fi

python3 - <<'PY' "${PREFLIGHT_ARTIFACT}" "${ACCELERATOR_DEVICE}" || fail_phase preflight "device_path=${ACCELERATOR_DEVICE} launcher_status=malformed artifact=${PREFLIGHT_ARTIFACT} message=preflight artifact validation failed"
import json
import sys
from pathlib import Path

artifact = Path(sys.argv[1])
device = sys.argv[2]
report = json.loads(artifact.read_text(encoding='utf-8'))
metadata = report.get('metadata') or {}
stages = report.get('stages') or {}
if metadata.get('selected_path') != 'idxd':
    raise SystemExit(f"artifact selected_path={metadata.get('selected_path')!r} expected 'idxd'")
if metadata.get('accelerated_device_path') != device:
    raise SystemExit(
        f"artifact accelerated_device_path={metadata.get('accelerated_device_path')!r} expected {device!r}"
    )
if metadata.get('accelerated_lane') != 'codec_memmove':
    raise SystemExit(f"artifact accelerated_lane={metadata.get('accelerated_lane')!r} expected 'codec_memmove'")
if metadata.get('accelerated_direction') != 'bidirectional':
    raise SystemExit(
        f"artifact accelerated_direction={metadata.get('accelerated_direction')!r} expected 'bidirectional'"
    )
if stages.get('enabled') is not True:
    raise SystemExit(f"artifact stages.enabled={stages.get('enabled')!r} expected True")
required = ['encode', 'decode', 'buffer_reserve', 'body_accum', 'frame_header']
if all((stages.get(name) or {}).get('bytes', 0) == 0 for name in required):
    raise SystemExit('artifact stage counters stayed placeholder-only')
print(
    f"[verify_s03_idxd_path] phase=preflight-validation artifact={artifact} device_path={device} verdict=pass"
)
PY

printf '[verify_s03_idxd_path] phase=runner output_dir=%s device_path=%s timeout=%s\n' "${OUTPUT_DIR}" "${ACCELERATOR_DEVICE}" "${RUN_TIMEOUT}"
if ! timeout "${RUN_TIMEOUT}" \
  python3 "${RUNNER_PATH}" \
    --manifest "${MANIFEST_PATH}" \
    --binary "${TONIC_BINARY}" \
    --output-dir "${OUTPUT_DIR}" \
    --accelerator-device "${ACCELERATOR_DEVICE}"; then
  fail_phase runner "device_path=${ACCELERATOR_DEVICE} artifact=${OUTPUT_DIR} message=split-process idxd runner failed"
fi

printf '[verify_s03_idxd_path] phase=done verdict=pass output_dir=%s device_path=%s preflight_artifact=%s\n' "${OUTPUT_DIR}" "${ACCELERATOR_DEVICE}" "${PREFLIGHT_ARTIFACT}"
