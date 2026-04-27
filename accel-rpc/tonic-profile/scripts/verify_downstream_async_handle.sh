#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
TONIC_PROFILE_DIR=$(cd -- "${SCRIPT_DIR}/.." && pwd)
ACCEL_RPC_DIR=$(cd -- "${TONIC_PROFILE_DIR}/.." && pwd)
REPO_ROOT=$(cd -- "${ACCEL_RPC_DIR}/.." && pwd)

OUTPUT_DIR=${TONIC_PROFILE_DOWNSTREAM_ASYNC_OUTPUT_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/tonic-profile-downstream-async.XXXXXX")}
REQUEST_BYTES=${TONIC_PROFILE_DOWNSTREAM_ASYNC_BYTES:-64}
BUILD_PROFILE=${TONIC_PROFILE_DOWNSTREAM_ASYNC_PROFILE:-dev}
if [[ "${BUILD_PROFILE}" == "dev" ]]; then
  TARGET_SUBDIR=debug
else
  TARGET_SUBDIR=${BUILD_PROFILE}
fi
PREFLIGHT_TIMEOUT=${TONIC_PROFILE_DOWNSTREAM_ASYNC_PREFLIGHT_TIMEOUT:-20s}
RUN_TIMEOUT=${TONIC_PROFILE_DOWNSTREAM_ASYNC_RUN_TIMEOUT:-20s}
SKIP_BUILD=${TONIC_PROFILE_DOWNSTREAM_ASYNC_SKIP_BUILD:-0}
ARTIFACT_PATH="${OUTPUT_DIR}/downstream_async_handle.json"
STDOUT_PATH="${OUTPUT_DIR}/downstream_async_handle.stdout"
STDERR_PATH="${OUTPUT_DIR}/downstream_async_handle.stderr"
PREFLIGHT_STDOUT_PATH="${OUTPUT_DIR}/preflight.stdout"
PREFLIGHT_STDERR_PATH="${OUTPUT_DIR}/preflight.stderr"
LAUNCHER_PATH=${TONIC_PROFILE_DOWNSTREAM_ASYNC_LAUNCHER_PATH:-${REPO_ROOT}/tools/build/dsa_launcher}
BINARY_PATH=${TONIC_PROFILE_DOWNSTREAM_ASYNC_BINARY:-${ACCEL_RPC_DIR}/target/${TARGET_SUBDIR}/downstream_async_handle}

find_default_device() {
  if [[ -n "${TONIC_PROFILE_DOWNSTREAM_ASYNC_DEVICE:-}" ]]; then
    printf '%s\n' "${TONIC_PROFILE_DOWNSTREAM_ASYNC_DEVICE}"
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
  printf '[verify_downstream_async_handle] phase=%s output_dir=%s artifact=%s %s\n' "${phase}" "${OUTPUT_DIR}" "${ARTIFACT_PATH}" "$*" >&2
  exit 1
}

log_phase() {
  local phase=$1
  shift
  printf '[verify_downstream_async_handle] phase=%s output_dir=%s artifact=%s %s\n' "${phase}" "${OUTPUT_DIR}" "${ARTIFACT_PATH}" "$*"
}

complete_with_explicit_failure() {
  local phase=$1
  shift
  log_phase done "verdict=expected_failure failure_phase=${phase} $*"
  exit 0
}

DEVICE_PATH=$(find_default_device) || complete_with_explicit_failure preflight 'device_path=<none> launcher_status=missing_work_queue message=no /dev/dsa/wq* device found; set TONIC_PROFILE_DOWNSTREAM_ASYNC_DEVICE explicitly'

mkdir -p "${OUTPUT_DIR}" 2>/dev/null || fail_phase preflight "device_path=${DEVICE_PATH} launcher_status=output_dir_unwritable message=failed to create output directory"
touch "${OUTPUT_DIR}/.write-test" 2>/dev/null || fail_phase preflight "device_path=${DEVICE_PATH} launcher_status=output_dir_unwritable message=failed to write into output directory"
rm -f "${OUTPUT_DIR}/.write-test"

command -v python3 >/dev/null 2>&1 || fail_phase preflight "device_path=${DEVICE_PATH} launcher_status=missing_python3 message=python3 command not found"
command -v timeout >/dev/null 2>&1 || fail_phase preflight "device_path=${DEVICE_PATH} launcher_status=missing_timeout message=timeout command not found"
command -v devenv >/dev/null 2>&1 || complete_with_explicit_failure preflight "device_path=${DEVICE_PATH} launcher_status=missing_devenv message=devenv command not found"

if [[ -n "${TONIC_PROFILE_DOWNSTREAM_ASYNC_BINARY:-}" && "${SKIP_BUILD}" != "1" ]]; then
  fail_phase preflight "device_path=${DEVICE_PATH} launcher_status=contradictory_overrides message=TONIC_PROFILE_DOWNSTREAM_ASYNC_BINARY requires TONIC_PROFILE_DOWNSTREAM_ASYNC_SKIP_BUILD=1 so the verifier does not build one binary and execute another"
fi

if [[ "${SKIP_BUILD}" != "1" ]]; then
  log_phase build "device_path=${DEVICE_PATH} workspace=${ACCEL_RPC_DIR} binary=${BINARY_PATH}"
  (
    cd "${ACCEL_RPC_DIR}"
    cargo build --profile "${BUILD_PROFILE}" -p tonic-profile --bin downstream_async_handle
  )
fi

[[ -x "${BINARY_PATH}" ]] || fail_phase preflight "device_path=${DEVICE_PATH} launcher_status=missing_binary binary=${BINARY_PATH} message=downstream_async_handle binary is not executable"
[[ -x "${LAUNCHER_PATH}" ]] || complete_with_explicit_failure preflight "device_path=${DEVICE_PATH} launcher_status=missing_launcher launcher_path=${LAUNCHER_PATH} message=build the launcher with launch in a privileged shell before running this verifier"

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
  complete_with_explicit_failure preflight "device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} stdout=${PREFLIGHT_STDOUT_PATH} stderr=${PREFLIGHT_STDERR_PATH} message=launch-wrapped CLI preflight exceeded timeout"
fi

if [[ "${PREFLIGHT_EXIT}" -ne 2 ]] || ! grep -q 'invalid value `abc` for `--bytes`' "${PREFLIGHT_STDERR_PATH}"; then
  fail_phase preflight "device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} stdout=${PREFLIGHT_STDOUT_PATH} stderr=${PREFLIGHT_STDERR_PATH} message=launch-wrapped CLI preflight failed"
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
  fail_phase runtime "device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} stdout=${STDOUT_PATH} stderr=${STDERR_PATH} message=launch-wrapped downstream async proof exceeded timeout before producing a classified artifact"
fi

ARTIFACT_FIELDS=$(python3 - <<'PY' "${ARTIFACT_PATH}" "${STDOUT_PATH}" "${DEVICE_PATH}" "${REQUEST_BYTES}"
import json
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
    'proof_seam',
    'consumer_package',
    'binding_package',
    'composition',
    'operation_count',
    'device_path',
    'requested_bytes',
    'phase',
    'error_kind',
    'lifecycle_failure_kind',
    'worker_failure_kind',
    'validation_phase',
    'validation_error_kind',
    'message',
}
missing = sorted(required - report.keys())
if missing:
    raise SystemExit(f"artifact missing required fields: {', '.join(missing)}")

expected_scalars = {
    'proof_seam': 'downstream_async_handle',
    'consumer_package': 'tonic-profile',
    'binding_package': 'idxd-rust',
    'composition': 'tokio_join',
    'operation_count': 2,
    'device_path': expected_device,
    'requested_bytes': expected_bytes,
}
for key, expected in expected_scalars.items():
    if report[key] != expected:
        raise SystemExit(f"artifact {key}={report[key]!r} expected {expected!r}")

if not isinstance(report['ok'], bool):
    raise SystemExit('artifact field ok must be boolean')
if not isinstance(report['phase'], str) or not report['phase']:
    raise SystemExit('artifact phase must be a non-empty string')
if not isinstance(report['message'], str) or not report['message']:
    raise SystemExit('artifact message must be a non-empty string')

ok = report['ok']
phase = report['phase']
error_kind = report['error_kind']
lifecycle_failure_kind = report['lifecycle_failure_kind']
worker_failure_kind = report['worker_failure_kind']
validation_phase = report['validation_phase']
validation_error_kind = report['validation_error_kind']

if ok:
    if phase != 'completed':
        raise SystemExit(f"successful artifact phase={phase!r} expected 'completed'")
    if error_kind is not None:
        raise SystemExit('successful artifact error_kind must be null')
    if lifecycle_failure_kind is not None:
        raise SystemExit('successful artifact lifecycle_failure_kind must be null')
    if worker_failure_kind is not None:
        raise SystemExit('successful artifact worker_failure_kind must be null')
    if validation_phase != 'completed':
        raise SystemExit('successful artifact validation_phase must be completed')
    if validation_error_kind is not None:
        raise SystemExit('successful artifact validation_error_kind must be null')
    if 'verified 2 joined cloned-handle async memmoves' not in report['message']:
        raise SystemExit('successful artifact message is missing downstream joined-handle proof')
else:
    if error_kind not in {'lifecycle_failure', 'worker_failure', 'validation_failure'}:
        raise SystemExit('failed artifact error_kind must be lifecycle_failure, worker_failure, or validation_failure')

    if error_kind == 'lifecycle_failure':
        if phase != 'async_lifecycle':
            raise SystemExit("lifecycle-failure artifact phase must be 'async_lifecycle'")
        if not isinstance(lifecycle_failure_kind, str) or not lifecycle_failure_kind:
            raise SystemExit('lifecycle-failure artifact lifecycle_failure_kind must be a non-empty string')
        if worker_failure_kind is not None:
            raise SystemExit('lifecycle-failure artifact worker_failure_kind must be null')
        if validation_phase is not None:
            raise SystemExit('lifecycle-failure artifact validation_phase must be null')
        if validation_error_kind is not None:
            raise SystemExit('lifecycle-failure artifact validation_error_kind must be null')
    elif error_kind == 'worker_failure':
        if phase != 'async_worker':
            raise SystemExit("worker-failure artifact phase must be 'async_worker'")
        if lifecycle_failure_kind is not None:
            raise SystemExit('worker-failure artifact lifecycle_failure_kind must be null')
        if not isinstance(worker_failure_kind, str) or not worker_failure_kind:
            raise SystemExit('worker-failure artifact worker_failure_kind must be a non-empty string')
        if validation_phase is not None:
            raise SystemExit('worker-failure artifact validation_phase must be null')
        if validation_error_kind is not None:
            raise SystemExit('worker-failure artifact validation_error_kind must be null')
    else:
        if lifecycle_failure_kind is not None:
            raise SystemExit('validation-failure artifact lifecycle_failure_kind must be null')
        if worker_failure_kind is not None:
            raise SystemExit('validation-failure artifact worker_failure_kind must be null')
        if not isinstance(validation_phase, str) or not validation_phase:
            raise SystemExit('validation-failure artifact validation_phase must be a non-empty string')
        if phase != validation_phase:
            raise SystemExit('validation-failure artifact phase must match validation_phase')
        if not isinstance(validation_error_kind, str) or not validation_error_kind:
            raise SystemExit('validation-failure artifact validation_error_kind must be a non-empty string')

print(f"ok={str(ok).lower()}")
print(f"proof_seam={report['proof_seam']}")
print(f"consumer_package={report['consumer_package']}")
print(f"binding_package={report['binding_package']}")
print(f"composition={report['composition']}")
print(f"operation_count={report['operation_count']}")
print(f"phase={phase}")
print(f"error_kind={error_kind if error_kind is not None else 'null'}")
print(f"lifecycle_failure_kind={lifecycle_failure_kind if lifecycle_failure_kind is not None else 'null'}")
print(f"worker_failure_kind={worker_failure_kind if worker_failure_kind is not None else 'null'}")
print(f"validation_phase={validation_phase if validation_phase is not None else 'null'}")
print(f"validation_error_kind={validation_error_kind if validation_error_kind is not None else 'null'}")
print(f"requested_bytes={report['requested_bytes']}")
PY
) || fail_phase artifact_validation "device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} stdout=${STDOUT_PATH} stderr=${STDERR_PATH} message=artifact validation failed"

ARTIFACT_OK=
ARTIFACT_PROOF_SEAM=
ARTIFACT_CONSUMER_PACKAGE=
ARTIFACT_BINDING_PACKAGE=
ARTIFACT_COMPOSITION=
ARTIFACT_OPERATION_COUNT=
ARTIFACT_PHASE=
ARTIFACT_ERROR_KIND=
ARTIFACT_LIFECYCLE_FAILURE_KIND=
ARTIFACT_WORKER_FAILURE_KIND=
ARTIFACT_VALIDATION_PHASE=
ARTIFACT_VALIDATION_ERROR_KIND=
ARTIFACT_REQUESTED_BYTES=
while IFS='=' read -r key value; do
  case "${key}" in
    ok) ARTIFACT_OK=${value} ;;
    proof_seam) ARTIFACT_PROOF_SEAM=${value} ;;
    consumer_package) ARTIFACT_CONSUMER_PACKAGE=${value} ;;
    binding_package) ARTIFACT_BINDING_PACKAGE=${value} ;;
    composition) ARTIFACT_COMPOSITION=${value} ;;
    operation_count) ARTIFACT_OPERATION_COUNT=${value} ;;
    phase) ARTIFACT_PHASE=${value} ;;
    error_kind) ARTIFACT_ERROR_KIND=${value} ;;
    lifecycle_failure_kind) ARTIFACT_LIFECYCLE_FAILURE_KIND=${value} ;;
    worker_failure_kind) ARTIFACT_WORKER_FAILURE_KIND=${value} ;;
    validation_phase) ARTIFACT_VALIDATION_PHASE=${value} ;;
    validation_error_kind) ARTIFACT_VALIDATION_ERROR_KIND=${value} ;;
    requested_bytes) ARTIFACT_REQUESTED_BYTES=${value} ;;
  esac
done <<< "${ARTIFACT_FIELDS}"

log_phase artifact_validation "device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} proof_seam=${ARTIFACT_PROOF_SEAM} consumer_package=${ARTIFACT_CONSUMER_PACKAGE} binding_package=${ARTIFACT_BINDING_PACKAGE} composition=${ARTIFACT_COMPOSITION} operation_count=${ARTIFACT_OPERATION_COUNT} phase=${ARTIFACT_PHASE} error_kind=${ARTIFACT_ERROR_KIND} lifecycle_failure_kind=${ARTIFACT_LIFECYCLE_FAILURE_KIND} worker_failure_kind=${ARTIFACT_WORKER_FAILURE_KIND} validation_phase=${ARTIFACT_VALIDATION_PHASE} validation_error_kind=${ARTIFACT_VALIDATION_ERROR_KIND} requested_bytes=${ARTIFACT_REQUESTED_BYTES}"

if [[ "${ARTIFACT_OK}" != "true" ]]; then
  complete_with_explicit_failure runtime "device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} proof_seam=${ARTIFACT_PROOF_SEAM} consumer_package=${ARTIFACT_CONSUMER_PACKAGE} binding_package=${ARTIFACT_BINDING_PACKAGE} composition=${ARTIFACT_COMPOSITION} operation_count=${ARTIFACT_OPERATION_COUNT} phase=${ARTIFACT_PHASE} error_kind=${ARTIFACT_ERROR_KIND} lifecycle_failure_kind=${ARTIFACT_LIFECYCLE_FAILURE_KIND} worker_failure_kind=${ARTIFACT_WORKER_FAILURE_KIND} validation_phase=${ARTIFACT_VALIDATION_PHASE} validation_error_kind=${ARTIFACT_VALIDATION_ERROR_KIND} requested_bytes=${ARTIFACT_REQUESTED_BYTES} stdout=${STDOUT_PATH} stderr=${STDERR_PATH} message=downstream async proof reported a typed failure"
fi

if [[ "${RUN_EXIT}" -ne 0 ]]; then
  fail_phase runtime "device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} phase=${ARTIFACT_PHASE} error_kind=${ARTIFACT_ERROR_KIND} lifecycle_failure_kind=${ARTIFACT_LIFECYCLE_FAILURE_KIND} worker_failure_kind=${ARTIFACT_WORKER_FAILURE_KIND} validation_phase=${ARTIFACT_VALIDATION_PHASE} validation_error_kind=${ARTIFACT_VALIDATION_ERROR_KIND} requested_bytes=${ARTIFACT_REQUESTED_BYTES} stdout=${STDOUT_PATH} stderr=${STDERR_PATH} message=downstream async proof exited non-zero despite a success artifact"
fi

log_phase done "device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} proof_seam=${ARTIFACT_PROOF_SEAM} consumer_package=${ARTIFACT_CONSUMER_PACKAGE} binding_package=${ARTIFACT_BINDING_PACKAGE} composition=${ARTIFACT_COMPOSITION} operation_count=${ARTIFACT_OPERATION_COUNT} requested_bytes=${ARTIFACT_REQUESTED_BYTES} phase=${ARTIFACT_PHASE} error_kind=${ARTIFACT_ERROR_KIND} lifecycle_failure_kind=${ARTIFACT_LIFECYCLE_FAILURE_KIND} worker_failure_kind=${ARTIFACT_WORKER_FAILURE_KIND} validation_phase=${ARTIFACT_VALIDATION_PHASE} validation_error_kind=${ARTIFACT_VALIDATION_ERROR_KIND} stdout=${STDOUT_PATH} stderr=${STDERR_PATH} verdict=pass"
