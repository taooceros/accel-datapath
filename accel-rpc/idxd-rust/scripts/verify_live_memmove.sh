#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
CRATE_DIR=$(cd -- "${SCRIPT_DIR}/.." && pwd)
ACCEL_RPC_DIR=$(cd -- "${CRATE_DIR}/.." && pwd)
REPO_ROOT=$(cd -- "${ACCEL_RPC_DIR}/.." && pwd)

OUTPUT_DIR=${IDXD_RUST_VERIFY_OUTPUT_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/idxd-rust-live-memmove.XXXXXX")}
REQUEST_BYTES=${IDXD_RUST_VERIFY_BYTES:-64}
BUILD_PROFILE=${IDXD_RUST_VERIFY_PROFILE:-dev}
if [[ "${BUILD_PROFILE}" == "dev" ]]; then
  TARGET_SUBDIR=debug
else
  TARGET_SUBDIR=${BUILD_PROFILE}
fi
PREFLIGHT_TIMEOUT=${IDXD_RUST_VERIFY_PREFLIGHT_TIMEOUT:-20s}
RUN_TIMEOUT=${IDXD_RUST_VERIFY_RUN_TIMEOUT:-20s}
SKIP_BUILD=${IDXD_RUST_VERIFY_SKIP_BUILD:-0}
ARTIFACT_PATH="${OUTPUT_DIR}/live_memmove.json"
STDOUT_PATH="${OUTPUT_DIR}/live_memmove.stdout"
STDERR_PATH="${OUTPUT_DIR}/live_memmove.stderr"
PREFLIGHT_STDOUT_PATH="${OUTPUT_DIR}/preflight.stdout"
PREFLIGHT_STDERR_PATH="${OUTPUT_DIR}/preflight.stderr"
LAUNCHER_PATH=${IDXD_RUST_VERIFY_LAUNCHER_PATH:-${REPO_ROOT}/tools/build/dsa_launcher}
BINARY_PATH=${IDXD_RUST_VERIFY_BINARY:-${ACCEL_RPC_DIR}/target/${TARGET_SUBDIR}/live_memmove}

find_default_device() {
  if [[ -n "${IDXD_RUST_VERIFY_DEVICE:-}" ]]; then
    printf '%s\n' "${IDXD_RUST_VERIFY_DEVICE}"
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
  printf '[verify_live_memmove] phase=%s output_dir=%s artifact=%s %s\n' "${phase}" "${OUTPUT_DIR}" "${ARTIFACT_PATH}" "$*" >&2
  exit 1
}

log_phase() {
  local phase=$1
  shift
  printf '[verify_live_memmove] phase=%s output_dir=%s artifact=%s %s\n' "${phase}" "${OUTPUT_DIR}" "${ARTIFACT_PATH}" "$*"
}

complete_with_explicit_failure() {
  local phase=$1
  shift
  log_phase done "verdict=expected_failure failure_phase=${phase} $*"
  exit 0
}

DEVICE_PATH=$(find_default_device) || fail_phase preflight 'device_path=<none> launcher_status=missing_work_queue message=no /dev/dsa/wq* device found; set IDXD_RUST_VERIFY_DEVICE explicitly'

mkdir -p "${OUTPUT_DIR}" 2>/dev/null || fail_phase preflight "device_path=${DEVICE_PATH} launcher_status=output_dir_unwritable message=failed to create output directory"
touch "${OUTPUT_DIR}/.write-test" 2>/dev/null || fail_phase preflight "device_path=${DEVICE_PATH} launcher_status=output_dir_unwritable message=failed to write into output directory"
rm -f "${OUTPUT_DIR}/.write-test"

command -v python3 >/dev/null 2>&1 || fail_phase preflight "device_path=${DEVICE_PATH} launcher_status=missing_python3 message=python3 command not found"
command -v timeout >/dev/null 2>&1 || fail_phase preflight "device_path=${DEVICE_PATH} launcher_status=missing_timeout message=timeout command not found"
command -v devenv >/dev/null 2>&1 || fail_phase preflight "device_path=${DEVICE_PATH} launcher_status=missing_devenv message=devenv command not found"

if [[ "${SKIP_BUILD}" != "1" ]]; then
  log_phase build "device_path=${DEVICE_PATH} workspace=${ACCEL_RPC_DIR} binary=${BINARY_PATH}"
  (
    cd "${ACCEL_RPC_DIR}"
    cargo build --profile "${BUILD_PROFILE}" -p idxd-rust --bin live_memmove
  )
fi

[[ -x "${BINARY_PATH}" ]] || fail_phase preflight "device_path=${DEVICE_PATH} launcher_status=missing_binary binary=${BINARY_PATH} message=live_memmove binary is not executable"
[[ -x "${LAUNCHER_PATH}" ]] || fail_phase preflight "device_path=${DEVICE_PATH} launcher_status=missing_launcher launcher_path=${LAUNCHER_PATH} message=build the launcher with launch in a privileged shell before running this verifier"

if command -v getcap >/dev/null 2>&1; then
  LAUNCHER_CAPS=$(getcap "${LAUNCHER_PATH}" || true)
  if [[ "${LAUNCHER_CAPS}" != *"cap_sys_rawio"* ]]; then
    complete_with_explicit_failure preflight "device_path=${DEVICE_PATH} launcher_status=missing_capability launcher_path=${LAUNCHER_PATH} message=launcher lacks cap_sys_rawio+eip"
  fi
  LAUNCHER_STATUS=ready
else
  LAUNCHER_CAPS=unavailable
  LAUNCHER_STATUS=capability_unchecked
fi

log_phase preflight "device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} binary=${BINARY_PATH} timeout=${PREFLIGHT_TIMEOUT}"

PREFLIGHT_EXIT=0
if timeout "${PREFLIGHT_TIMEOUT}" \
  devenv shell -- launch "${BINARY_PATH}" --bytes abc \
  >"${PREFLIGHT_STDOUT_PATH}" 2>"${PREFLIGHT_STDERR_PATH}"; then
  PREFLIGHT_EXIT=0
else
  PREFLIGHT_EXIT=$?
fi

if [[ "${PREFLIGHT_EXIT}" -eq 124 ]]; then
  fail_phase preflight "device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} stdout=${PREFLIGHT_STDOUT_PATH} stderr=${PREFLIGHT_STDERR_PATH} message=launch-wrapped preflight exceeded timeout"
fi

if [[ "${PREFLIGHT_EXIT}" -ne 2 ]] || ! grep -q 'invalid value `abc` for `--bytes`' "${PREFLIGHT_STDERR_PATH}"; then
  fail_phase preflight "device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} stdout=${PREFLIGHT_STDOUT_PATH} stderr=${PREFLIGHT_STDERR_PATH} message=launch-wrapped preflight failed"
fi

log_phase runtime "device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} binary=${BINARY_PATH} requested_bytes=${REQUEST_BYTES} timeout=${RUN_TIMEOUT}"

RUN_EXIT=0
if timeout "${RUN_TIMEOUT}" \
  devenv shell -- launch "${BINARY_PATH}" \
    --device "${DEVICE_PATH}" \
    --bytes "${REQUEST_BYTES}" \
    --format json \
    --artifact "${ARTIFACT_PATH}" \
    >"${STDOUT_PATH}" 2>"${STDERR_PATH}"; then
  RUN_EXIT=0
else
  RUN_EXIT=$?
fi

if [[ "${RUN_EXIT}" -eq 124 ]]; then
  fail_phase runtime_timeout "device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} stdout=${STDOUT_PATH} stderr=${STDERR_PATH} message=launch-wrapped validation exceeded timeout"
fi

ARTIFACT_FIELDS=$(python3 - <<'PY' "${ARTIFACT_PATH}" "${STDOUT_PATH}" "${DEVICE_PATH}" "${REQUEST_BYTES}"
import json
import re
import sys
from pathlib import Path

artifact_path = Path(sys.argv[1])
stdout_path = Path(sys.argv[2])
expected_device = sys.argv[3]
expected_bytes = int(sys.argv[4])

if not artifact_path.is_file():
    raise SystemExit(f"missing artifact file: {artifact_path}")
if artifact_path.stat().st_size == 0:
    raise SystemExit(f"empty artifact file: {artifact_path}")
if not stdout_path.is_file():
    raise SystemExit(f"missing stdout file: {stdout_path}")

artifact_text = artifact_path.read_text(encoding='utf-8').strip()
stdout_text = stdout_path.read_text(encoding='utf-8').strip()
if artifact_text != stdout_text:
    raise SystemExit('stdout and artifact diverged')

try:
    report = json.loads(artifact_text)
except json.JSONDecodeError as exc:
    raise SystemExit(f"artifact is not valid JSON: {exc}")

required = {
    'ok',
    'device_path',
    'requested_bytes',
    'page_fault_retries',
    'final_status',
    'phase',
    'error_kind',
    'message',
}
missing = sorted(required - report.keys())
if missing:
    raise SystemExit(f"artifact missing required fields: {', '.join(missing)}")

if not isinstance(report['ok'], bool):
    raise SystemExit('artifact field ok must be boolean')
if report['device_path'] != expected_device:
    raise SystemExit(
        f"artifact device_path={report['device_path']!r} expected {expected_device!r}"
    )
if report['requested_bytes'] != expected_bytes:
    raise SystemExit(
        f"artifact requested_bytes={report['requested_bytes']!r} expected {expected_bytes!r}"
    )
if not isinstance(report['phase'], str) or not report['phase']:
    raise SystemExit('artifact phase must be a non-empty string')
if not isinstance(report['message'], str) or not report['message']:
    raise SystemExit('artifact message must be a non-empty string')

ok = report['ok']
page_fault_retries = report['page_fault_retries']
final_status = report['final_status']
error_kind = report['error_kind']
phase = report['phase']

if ok:
    if phase != 'completed':
        raise SystemExit(f"successful artifact phase={phase!r} expected 'completed'")
    if error_kind is not None:
        raise SystemExit('successful artifact error_kind must be null')
    if not isinstance(page_fault_retries, int) or page_fault_retries < 0:
        raise SystemExit('successful artifact page_fault_retries must be a non-negative integer')
    if not isinstance(final_status, str) or not re.fullmatch(r'0x[0-9a-f]{2}', final_status):
        raise SystemExit('successful artifact final_status must be a 0xNN string')
    if f"verified {expected_bytes} copied bytes" not in report['message']:
        raise SystemExit('successful artifact message is missing copied-bytes proof')
else:
    if not isinstance(error_kind, str) or not error_kind:
        raise SystemExit('failed artifact error_kind must be a non-empty string')
    if page_fault_retries is not None and (not isinstance(page_fault_retries, int) or page_fault_retries < 0):
        raise SystemExit('failed artifact page_fault_retries must be null or a non-negative integer')
    if final_status is not None and (not isinstance(final_status, str) or not re.fullmatch(r'0x[0-9a-f]{2}', final_status)):
        raise SystemExit('failed artifact final_status must be null or a 0xNN string')

print(f"ok={str(ok).lower()}")
print(f"phase={phase}")
print(f"error_kind={error_kind if error_kind is not None else 'null'}")
print(f"requested_bytes={report['requested_bytes']}")
print(f"page_fault_retries={page_fault_retries if page_fault_retries is not None else 'null'}")
print(f"final_status={final_status if final_status is not None else 'null'}")
PY
) || fail_phase artifact_validation "device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} stdout=${STDOUT_PATH} stderr=${STDERR_PATH} message=artifact validation failed"

ARTIFACT_OK=
ARTIFACT_PHASE=
ARTIFACT_ERROR_KIND=
ARTIFACT_REQUESTED_BYTES=
ARTIFACT_PAGE_FAULT_RETRIES=
ARTIFACT_FINAL_STATUS=
while IFS='=' read -r key value; do
  case "${key}" in
    ok) ARTIFACT_OK=${value} ;;
    phase) ARTIFACT_PHASE=${value} ;;
    error_kind) ARTIFACT_ERROR_KIND=${value} ;;
    requested_bytes) ARTIFACT_REQUESTED_BYTES=${value} ;;
    page_fault_retries) ARTIFACT_PAGE_FAULT_RETRIES=${value} ;;
    final_status) ARTIFACT_FINAL_STATUS=${value} ;;
  esac
done <<< "${ARTIFACT_FIELDS}"

log_phase artifact_validation "device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} validation_phase=${ARTIFACT_PHASE} validation_error_kind=${ARTIFACT_ERROR_KIND} requested_bytes=${ARTIFACT_REQUESTED_BYTES}"

if [[ "${ARTIFACT_OK}" != "true" ]]; then
  complete_with_explicit_failure runtime "device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} validation_phase=${ARTIFACT_PHASE} validation_error_kind=${ARTIFACT_ERROR_KIND} requested_bytes=${ARTIFACT_REQUESTED_BYTES} page_fault_retries=${ARTIFACT_PAGE_FAULT_RETRIES} final_status=${ARTIFACT_FINAL_STATUS} stdout=${STDOUT_PATH} stderr=${STDERR_PATH} message=live validation reported failure"
fi

if [[ "${RUN_EXIT}" -ne 0 ]]; then
  fail_phase runtime "device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} validation_phase=${ARTIFACT_PHASE} validation_error_kind=${ARTIFACT_ERROR_KIND} requested_bytes=${ARTIFACT_REQUESTED_BYTES} page_fault_retries=${ARTIFACT_PAGE_FAULT_RETRIES} final_status=${ARTIFACT_FINAL_STATUS} stdout=${STDOUT_PATH} stderr=${STDERR_PATH} message=live validation exited non-zero despite a success artifact"
fi

log_phase done "device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} requested_bytes=${ARTIFACT_REQUESTED_BYTES} page_fault_retries=${ARTIFACT_PAGE_FAULT_RETRIES} final_status=${ARTIFACT_FINAL_STATUS} validation_phase=${ARTIFACT_PHASE} stdout=${STDOUT_PATH} stderr=${STDERR_PATH} verdict=pass"
