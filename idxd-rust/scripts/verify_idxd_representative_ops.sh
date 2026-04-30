#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
CRATE_DIR=$(cd -- "${SCRIPT_DIR}/.." && pwd)
REPO_ROOT=$(cd -- "${CRATE_DIR}/.." && pwd)

OUTPUT_DIR=${IDXD_RUST_VERIFY_OUTPUT_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/idxd-rust-representative-ops.XXXXXX")}
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
LAUNCHER_PATH=${IDXD_RUST_VERIFY_LAUNCHER_PATH:-${REPO_ROOT}/tools/build/dsa_launcher}
BINARY_PATH=${IDXD_RUST_VERIFY_BINARY:-${REPO_ROOT}/target/${TARGET_SUBDIR}/live_idxd_op}
PREFLIGHT_STDOUT_PATH="${OUTPUT_DIR}/preflight.stdout"
PREFLIGHT_STDERR_PATH="${OUTPUT_DIR}/preflight.stderr"

declare -a TARGET_LABELS=()
declare -a TARGET_OPS=()
declare -a TARGET_DEVICES=()
declare -a TARGET_ARTIFACTS=()
declare -a TARGET_STDOUTS=()
declare -a TARGET_STDERRS=()

join_csv() {
  local IFS=,
  if [[ $# -eq 0 ]]; then
    printf '<none>'
  else
    printf '%s' "$*"
  fi
}

target_list() {
  local -a entries=()
  local index
  for index in "${!TARGET_LABELS[@]}"; do
    entries+=("${TARGET_LABELS[$index]}:${TARGET_DEVICES[$index]}")
  done
  join_csv "${entries[@]}"
}

artifact_path_list() {
  join_csv "${TARGET_ARTIFACTS[@]}"
}

stdout_path_list() {
  join_csv "${TARGET_STDOUTS[@]}"
}

stderr_path_list() {
  join_csv "${TARGET_STDERRS[@]}"
}

log_phase() {
  local phase=$1
  shift
  printf '[verify_idxd_representative_ops] phase=%s output_dir=%s %s\n' "${phase}" "${OUTPUT_DIR}" "$*"
}

fail_phase() {
  local phase=$1
  shift
  printf '[verify_idxd_representative_ops] phase=%s output_dir=%s targets=%s artifact_paths=%s stdout_paths=%s stderr_paths=%s %s\n' \
    "${phase}" \
    "${OUTPUT_DIR}" \
    "$(target_list)" \
    "$(artifact_path_list)" \
    "$(stdout_path_list)" \
    "$(stderr_path_list)" \
    "$*" >&2
  exit 1
}

complete_with_explicit_failure() {
  local phase=$1
  shift
  log_phase done "verdict=expected_failure failure_phase=${phase} targets=$(target_list) artifact_paths=$(artifact_path_list) stdout_paths=$(stdout_path_list) stderr_paths=$(stderr_path_list) $*"
  exit 0
}

find_first_device() {
  local pattern=$1
  local candidate
  for candidate in ${pattern}; do
    if [[ -e "${candidate}" ]]; then
      printf '%s\n' "${candidate}"
      return 0
    fi
  done
  return 1
}

find_dsa_device() {
  if [[ -n "${IDXD_RUST_VERIFY_DSA_DEVICE:-}" ]]; then
    printf '%s\n' "${IDXD_RUST_VERIFY_DSA_DEVICE}"
    return 0
  fi
  find_first_device '/dev/dsa/wq*'
}

find_iax_device() {
  if [[ -n "${IDXD_RUST_VERIFY_IAX_DEVICE:-}" ]]; then
    printf '%s\n' "${IDXD_RUST_VERIFY_IAX_DEVICE}"
    return 0
  fi
  find_first_device '/dev/iax/wq*'
}

find_shared_dsa_device() {
  local primary=$1
  if [[ -n "${IDXD_RUST_VERIFY_DSA_SHARED_DEVICE:-}" ]]; then
    printf '%s\n' "${IDXD_RUST_VERIFY_DSA_SHARED_DEVICE}"
    return 0
  fi

  local candidate
  for candidate in /dev/dsa/wq*; do
    if [[ -e "${candidate}" && "${candidate}" != "${primary}" ]]; then
      printf '%s\n' "${candidate}"
      return 0
    fi
  done

  return 1
}

add_target() {
  local label=$1
  local op=$2
  local device=$3
  local stem=$4

  TARGET_LABELS+=("${label}")
  TARGET_OPS+=("${op}")
  TARGET_DEVICES+=("${device}")
  TARGET_ARTIFACTS+=("${OUTPUT_DIR}/${stem}.json")
  TARGET_STDOUTS+=("${OUTPUT_DIR}/${stem}.stdout")
  TARGET_STDERRS+=("${OUTPUT_DIR}/${stem}.stderr")
}

mkdir -p "${OUTPUT_DIR}" 2>/dev/null || fail_phase preflight 'launcher_status=output_dir_unwritable message=failed to create output directory'
touch "${OUTPUT_DIR}/.write-test" 2>/dev/null || fail_phase preflight 'launcher_status=output_dir_unwritable message=failed to write into output directory'
rm -f "${OUTPUT_DIR}/.write-test"

DSA_DEVICE=$(find_dsa_device) || complete_with_explicit_failure preflight 'launcher_status=missing_work_queue missing_target=dsa-memmove message=no /dev/dsa/wq* device found; set IDXD_RUST_VERIFY_DSA_DEVICE explicitly'
add_target dsa-memmove dsa-memmove "${DSA_DEVICE}" dsa_memmove
if SHARED_DSA_DEVICE=$(find_shared_dsa_device "${DSA_DEVICE}"); then
  add_target dsa-memmove-shared dsa-memmove "${SHARED_DSA_DEVICE}" dsa_memmove_shared
fi

IAX_DEVICE=$(find_iax_device) || complete_with_explicit_failure preflight 'launcher_status=missing_work_queue missing_target=iax-crc64 message=no /dev/iax/wq* device found; set IDXD_RUST_VERIFY_IAX_DEVICE explicitly'
add_target iax-crc64 iax-crc64 "${IAX_DEVICE}" iax_crc64

command -v python3 >/dev/null 2>&1 || complete_with_explicit_failure preflight 'launcher_status=missing_python3 message=python3 command not found'
command -v timeout >/dev/null 2>&1 || complete_with_explicit_failure preflight 'launcher_status=missing_timeout message=timeout command not found'
command -v devenv >/dev/null 2>&1 || complete_with_explicit_failure preflight 'launcher_status=missing_devenv message=devenv command not found'

if [[ -n "${IDXD_RUST_VERIFY_BINARY:-}" && "${SKIP_BUILD}" != "1" ]]; then
  fail_phase preflight 'launcher_status=contradictory_overrides message=IDXD_RUST_VERIFY_BINARY requires IDXD_RUST_VERIFY_SKIP_BUILD=1 so the verifier does not build one binary and execute another'
fi

if [[ "${SKIP_BUILD}" != "1" ]]; then
  log_phase build "workspace=${REPO_ROOT} binary=${BINARY_PATH} profile=${BUILD_PROFILE} targets=$(target_list)"
  if ! cargo build --manifest-path "${REPO_ROOT}/Cargo.toml" --profile "${BUILD_PROFILE}" -p idxd-rust --bin live_idxd_op; then
    fail_phase build "launcher_status=build_failed binary=${BINARY_PATH} message=failed to build live_idxd_op"
  fi
fi

[[ -x "${BINARY_PATH}" ]] || complete_with_explicit_failure preflight "launcher_status=missing_binary binary=${BINARY_PATH} message=live_idxd_op binary is not executable"
[[ -x "${LAUNCHER_PATH}" ]] || complete_with_explicit_failure preflight "launcher_status=missing_launcher launcher_path=${LAUNCHER_PATH} message=build the launcher with launch in a privileged shell before running this verifier"

if command -v getcap >/dev/null 2>&1; then
  LAUNCHER_CAP_CHECK_PATH=$(readlink -f "${LAUNCHER_PATH}" 2>/dev/null || printf '%s' "${LAUNCHER_PATH}")
  LAUNCHER_CAPS=$(getcap "${LAUNCHER_CAP_CHECK_PATH}" || true)
  if [[ "${LAUNCHER_CAPS}" != *"cap_sys_rawio"* ]]; then
    complete_with_explicit_failure preflight "launcher_status=missing_capability launcher_path=${LAUNCHER_PATH} message=launcher lacks cap_sys_rawio+eip"
  fi
  LAUNCHER_STATUS=ready
else
  LAUNCHER_CAPS=unavailable
  LAUNCHER_STATUS=capability_unchecked
fi

log_phase preflight "launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} binary=${BINARY_PATH} timeout=${PREFLIGHT_TIMEOUT} targets=$(target_list)"

PREFLIGHT_EXIT=0
if timeout "${PREFLIGHT_TIMEOUT}" \
  devenv shell -- launch "${BINARY_PATH}" --bytes abc \
  >"${PREFLIGHT_STDOUT_PATH}" 2>"${PREFLIGHT_STDERR_PATH}"; then
  PREFLIGHT_EXIT=0
else
  PREFLIGHT_EXIT=$?
fi

if [[ "${PREFLIGHT_EXIT}" -eq 124 ]]; then
  complete_with_explicit_failure preflight "launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} failure_kind=timeout stdout=${PREFLIGHT_STDOUT_PATH} stderr=${PREFLIGHT_STDERR_PATH} message=launch-wrapped preflight exceeded timeout"
fi

if [[ "${PREFLIGHT_EXIT}" -ne 2 ]] || ! grep -q 'invalid value `abc` for `--bytes`' "${PREFLIGHT_STDERR_PATH}"; then
  fail_phase preflight "launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} stdout=${PREFLIGHT_STDOUT_PATH} stderr=${PREFLIGHT_STDERR_PATH} message=launch-wrapped invalid-argument preflight failed"
fi

normalize_launch_stdout() {
  local raw_stdout_path=$1
  local normalized_stdout_path=$2

  if [[ ! -f "${raw_stdout_path}" ]]; then
    return 0
  fi

  # The repo's `launch` wrapper prints its own "Running: .../dsa_launcher ..." banner
  # to stdout before execing the proof binary. Keep stdout_paths as the proof
  # binary's stdout contract by dropping only that wrapper banner; preserve a raw
  # `.raw` sibling for low-level launcher debugging.
  python3 - <<'PY' "${raw_stdout_path}" "${normalized_stdout_path}"
import sys
from pathlib import Path

raw_path = Path(sys.argv[1])
normalized_path = Path(sys.argv[2])
lines = raw_path.read_text(encoding='utf-8').splitlines()
filtered = [line for line in lines if not (line.startswith('Running: ') and 'dsa_launcher' in line)]
text = '\n'.join(filtered)
if text:
    text += '\n'
normalized_path.write_text(text, encoding='utf-8')
PY
}

validate_artifact() {
  local artifact=$1
  local stdout_path=$2
  local expected_op=$3
  local expected_device=$4
  local expected_bytes=$5
  local target_label=$6

  python3 - <<'PY' "${artifact}" "${stdout_path}" "${expected_op}" "${expected_device}" "${expected_bytes}" "${target_label}"
import json
import re
import sys
from pathlib import Path

artifact_path = Path(sys.argv[1])
stdout_path = Path(sys.argv[2])
expected_op = sys.argv[3]
expected_device = sys.argv[4]
expected_bytes = int(sys.argv[5])
target_label = sys.argv[6]
expected_accelerator = 'dsa' if expected_op == 'dsa-memmove' else 'iax'

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

FORBIDDEN_PAYLOAD_FIELDS = {
    'payload',
    'payload_bytes',
    'payload_dump',
    'raw_payload',
    'dumped_payload',
    'source',
    'src',
    'source_bytes',
    'source_payload',
    'src_bytes',
    'src_payload',
    'destination',
    'dst',
    'destination_bytes',
    'destination_payload',
    'dst_bytes',
    'dst_payload',
}


def reject_payload_dump_fields(value, path='report'):
    if isinstance(value, dict):
        for key, child in value.items():
            if key in FORBIDDEN_PAYLOAD_FIELDS:
                raise SystemExit(f"artifact contains forbidden payload dump field {path}.{key}")
            reject_payload_dump_fields(child, f"{path}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            reject_payload_dump_fields(child, f"{path}[{index}]")


reject_payload_dump_fields(report)

required = {
    'ok',
    'operation',
    'accelerator',
    'device_path',
    'requested_bytes',
    'page_fault_retries',
    'final_status',
    'phase',
    'error_kind',
    'completion_error_code',
    'invalid_flags',
    'fault_addr',
    'crc64',
    'expected_crc64',
    'crc64_verified',
    'message',
}
missing = sorted(required - report.keys())
if missing:
    raise SystemExit(f"artifact missing required fields: {', '.join(missing)}")

if not isinstance(report['ok'], bool):
    raise SystemExit('artifact field ok must be boolean')
if report['operation'] != expected_op:
    raise SystemExit(f"artifact operation={report['operation']!r} expected {expected_op!r}")
if report['accelerator'] != expected_accelerator:
    raise SystemExit(
        f"artifact accelerator={report['accelerator']!r} expected {expected_accelerator!r}"
    )
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


def is_hex(value, digits=None):
    if not isinstance(value, str):
        return False
    pattern = r'0x[0-9a-f]+'
    if digits is not None:
        pattern = rf'0x[0-9a-f]{{{digits}}}'
    return re.fullmatch(pattern, value) is not None

ok = report['ok']
phase = report['phase']
error_kind = report['error_kind']
page_fault_retries = report['page_fault_retries']
final_status = report['final_status']
completion_error_code = report['completion_error_code']
invalid_flags = report['invalid_flags']
fault_addr = report['fault_addr']
crc64 = report['crc64']
expected_crc64 = report['expected_crc64']
crc64_verified = report['crc64_verified']

if completion_error_code is not None and not is_hex(completion_error_code, 2):
    raise SystemExit('completion_error_code must be null or a 0xNN string')
if invalid_flags is not None and not is_hex(invalid_flags, 8):
    raise SystemExit('invalid_flags must be null or a 0xNNNNNNNN string')
if fault_addr is not None and not is_hex(fault_addr):
    raise SystemExit('fault_addr must be null or a hex string')

if ok:
    if phase != 'completed':
        raise SystemExit(f"successful artifact phase={phase!r} expected 'completed'")
    if error_kind is not None:
        raise SystemExit('successful artifact error_kind must be null')
    if completion_error_code is not None:
        raise SystemExit('successful artifact completion_error_code must be null')
    if invalid_flags is not None:
        raise SystemExit('successful artifact invalid_flags must be null')
    if fault_addr is not None:
        raise SystemExit('successful artifact fault_addr must be null')
    if not isinstance(page_fault_retries, int) or page_fault_retries < 0:
        raise SystemExit('successful artifact page_fault_retries must be a non-negative integer')
    if not is_hex(final_status, 2):
        raise SystemExit('successful artifact final_status must be a 0xNN string')
    if expected_op == 'dsa-memmove':
        if crc64 is not None or expected_crc64 is not None or crc64_verified is not None:
            raise SystemExit('successful DSA artifact must not carry CRC fields')
        if f"verified {expected_bytes} copied bytes" not in report['message']:
            raise SystemExit('successful DSA artifact message is missing copied-byte proof')
    else:
        if not is_hex(crc64) or not is_hex(expected_crc64):
            raise SystemExit('successful IAX artifact must carry hex crc64 and expected_crc64')
        if crc64_verified is not True:
            raise SystemExit('successful IAX artifact crc64_verified must be true')
        if 'crc64' not in report['message'].lower():
            raise SystemExit('successful IAX artifact message is missing CRC proof')
else:
    if not isinstance(error_kind, str) or not error_kind:
        raise SystemExit('failed artifact error_kind must be a non-empty string')
    if page_fault_retries is not None and (
        not isinstance(page_fault_retries, int) or page_fault_retries < 0
    ):
        raise SystemExit('failed artifact page_fault_retries must be null or non-negative integer')
    if final_status is not None and not is_hex(final_status, 2):
        raise SystemExit('failed artifact final_status must be null or a 0xNN string')
    if crc64_verified is not None and not isinstance(crc64_verified, bool):
        raise SystemExit('failed artifact crc64_verified must be null or boolean')

print(f"ok={str(ok).lower()}")
print(f"target={target_label}")
print(f"operation={report['operation']}")
print(f"accelerator={report['accelerator']}")
print(f"device_path={report['device_path']}")
print(f"requested_bytes={report['requested_bytes']}")
print(f"phase={phase}")
print(f"error_kind={error_kind if error_kind is not None else 'null'}")
print(f"page_fault_retries={page_fault_retries if page_fault_retries is not None else 'null'}")
print(f"final_status={final_status if final_status is not None else 'null'}")
print(f"crc64_verified={str(crc64_verified).lower() if crc64_verified is not None else 'null'}")
PY
}

run_target() {
  local index=$1
  local label=${TARGET_LABELS[$index]}
  local op=${TARGET_OPS[$index]}
  local device=${TARGET_DEVICES[$index]}
  local artifact=${TARGET_ARTIFACTS[$index]}
  local stdout_path=${TARGET_STDOUTS[$index]}
  local stderr_path=${TARGET_STDERRS[$index]}
  local raw_stdout_path="${stdout_path}.raw"

  log_phase runtime "target=${label} operation=${op} device_path=${device} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} binary=${BINARY_PATH} requested_bytes=${REQUEST_BYTES} artifact=${artifact} stdout=${stdout_path} stderr=${stderr_path} raw_stdout=${raw_stdout_path} timeout=${RUN_TIMEOUT}"

  local run_exit=0
  if timeout "${RUN_TIMEOUT}" \
    devenv shell -- launch "${BINARY_PATH}" \
      --op "${op}" \
      --device "${device}" \
      --bytes "${REQUEST_BYTES}" \
      --format json \
      --artifact "${artifact}" \
      >"${raw_stdout_path}" 2>"${stderr_path}"; then
    run_exit=0
  else
    run_exit=$?
  fi
  normalize_launch_stdout "${raw_stdout_path}" "${stdout_path}"

  if [[ "${run_exit}" -eq 124 ]]; then
    complete_with_explicit_failure runtime "target=${label} operation=${op} device_path=${device} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} failure_kind=timeout artifact=${artifact} stdout=${stdout_path} stderr=${stderr_path} message=launch-wrapped representative operation exceeded timeout"
  fi

  local artifact_fields
  artifact_fields=$(validate_artifact "${artifact}" "${stdout_path}" "${op}" "${device}" "${REQUEST_BYTES}" "${label}") || fail_phase artifact_validation "target=${label} operation=${op} device_path=${device} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} artifact=${artifact} stdout=${stdout_path} stderr=${stderr_path} message=artifact validation failed"

  local artifact_ok=
  local artifact_phase=
  local artifact_error_kind=
  local artifact_requested_bytes=
  local artifact_page_fault_retries=
  local artifact_final_status=
  local artifact_crc64_verified=
  while IFS='=' read -r key value; do
    case "${key}" in
      ok) artifact_ok=${value} ;;
      phase) artifact_phase=${value} ;;
      error_kind) artifact_error_kind=${value} ;;
      requested_bytes) artifact_requested_bytes=${value} ;;
      page_fault_retries) artifact_page_fault_retries=${value} ;;
      final_status) artifact_final_status=${value} ;;
      crc64_verified) artifact_crc64_verified=${value} ;;
    esac
  done <<< "${artifact_fields}"

  log_phase artifact_validation "target=${label} operation=${op} device_path=${device} launcher_status=${LAUNCHER_STATUS} validation_phase=${artifact_phase} validation_error_kind=${artifact_error_kind} requested_bytes=${artifact_requested_bytes} page_fault_retries=${artifact_page_fault_retries} final_status=${artifact_final_status} crc64_verified=${artifact_crc64_verified} artifact=${artifact} stdout=${stdout_path} stderr=${stderr_path}"

  if [[ "${artifact_ok}" != "true" ]]; then
    if [[ "${run_exit}" -eq 0 ]]; then
      fail_phase runtime "target=${label} operation=${op} device_path=${device} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} validation_phase=${artifact_phase} validation_error_kind=${artifact_error_kind} artifact=${artifact} stdout=${stdout_path} stderr=${stderr_path} message=live_idxd_op exited zero despite a failure artifact"
    fi
    complete_with_explicit_failure runtime "target=${label} operation=${op} device_path=${device} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} validation_phase=${artifact_phase} validation_error_kind=${artifact_error_kind} requested_bytes=${artifact_requested_bytes} page_fault_retries=${artifact_page_fault_retries} final_status=${artifact_final_status} crc64_verified=${artifact_crc64_verified} artifact=${artifact} stdout=${stdout_path} stderr=${stderr_path} message=live_idxd_op reported representative operation failure"
  fi

  if [[ "${run_exit}" -ne 0 ]]; then
    fail_phase runtime "target=${label} operation=${op} device_path=${device} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} validation_phase=${artifact_phase} validation_error_kind=${artifact_error_kind} artifact=${artifact} stdout=${stdout_path} stderr=${stderr_path} message=live_idxd_op exited non-zero despite a success artifact"
  fi
}

for index in "${!TARGET_LABELS[@]}"; do
  run_target "${index}"
done

log_phase done "verdict=pass launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} requested_bytes=${REQUEST_BYTES} targets=$(target_list) artifact_paths=$(artifact_path_list) stdout_paths=$(stdout_path_list) stderr_paths=$(stderr_path_list)"
