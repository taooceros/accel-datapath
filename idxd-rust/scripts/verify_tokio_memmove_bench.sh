#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
CRATE_DIR=$(cd -- "${SCRIPT_DIR}/.." && pwd)
REPO_ROOT=$(cd -- "${CRATE_DIR}/.." && pwd)

OUTPUT_DIR=${IDXD_RUST_VERIFY_OUTPUT_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/idxd-rust-tokio-memmove-bench.XXXXXX")}
BACKEND=${IDXD_RUST_VERIFY_BACKEND:-hardware}
SUITE=${IDXD_RUST_VERIFY_SUITE:-canonical}
REQUEST_BYTES=${IDXD_RUST_VERIFY_BYTES:-64}
ITERATIONS=${IDXD_RUST_VERIFY_ITERATIONS:-2}
CONCURRENCY=${IDXD_RUST_VERIFY_CONCURRENCY:-2}
DURATION_MS=${IDXD_RUST_VERIFY_DURATION_MS:-10}
BUILD_PROFILE=${IDXD_RUST_VERIFY_PROFILE:-dev}
if [[ "${BUILD_PROFILE}" == "dev" ]]; then
  TARGET_SUBDIR=debug
else
  TARGET_SUBDIR=${BUILD_PROFILE}
fi
PREFLIGHT_TIMEOUT=${IDXD_RUST_VERIFY_PREFLIGHT_TIMEOUT:-20s}
RUN_TIMEOUT=${IDXD_RUST_VERIFY_RUN_TIMEOUT:-30s}
SKIP_BUILD=${IDXD_RUST_VERIFY_SKIP_BUILD:-0}
ARTIFACT_PATH="${OUTPUT_DIR}/tokio_memmove_bench.json"
STDOUT_PATH="${OUTPUT_DIR}/tokio_memmove_bench.stdout"
STDERR_PATH="${OUTPUT_DIR}/tokio_memmove_bench.stderr"
PREFLIGHT_STDOUT_PATH="${OUTPUT_DIR}/preflight.stdout"
PREFLIGHT_STDERR_PATH="${OUTPUT_DIR}/preflight.stderr"
LAUNCHER_PATH=${IDXD_RUST_VERIFY_LAUNCHER_PATH:-${REPO_ROOT}/tools/build/dsa_launcher}
BINARY_PATH=${IDXD_RUST_VERIFY_BINARY:-${REPO_ROOT}/target/${TARGET_SUBDIR}/tokio_memmove_bench}

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
  printf '[verify_tokio_memmove_bench] phase=%s output_dir=%s artifact=%s %s\n' "${phase}" "${OUTPUT_DIR}" "${ARTIFACT_PATH}" "$*" >&2
  exit 1
}

log_phase() {
  local phase=$1
  shift
  printf '[verify_tokio_memmove_bench] phase=%s output_dir=%s artifact=%s %s\n' "${phase}" "${OUTPUT_DIR}" "${ARTIFACT_PATH}" "$*"
}

complete_with_explicit_failure() {
  local phase=$1
  shift
  log_phase done "verdict=expected_failure failure_phase=${phase} $*"
  exit 0
}

case "${BACKEND}" in
  hardware|software) ;;
  *) fail_phase preflight "backend=${BACKEND} launcher_status=invalid_backend message=IDXD_RUST_VERIFY_BACKEND must be hardware or software" ;;
esac

case "${SUITE}" in
  canonical|latency|concurrency|throughput) ;;
  *) fail_phase preflight "backend=${BACKEND} suite=${SUITE} launcher_status=invalid_suite message=IDXD_RUST_VERIFY_SUITE must be canonical, latency, concurrency, or throughput" ;;
esac

mkdir -p "${OUTPUT_DIR}" 2>/dev/null || fail_phase preflight "backend=${BACKEND} launcher_status=output_dir_unwritable message=failed to create output directory"
touch "${OUTPUT_DIR}/.write-test" 2>/dev/null || fail_phase preflight "backend=${BACKEND} launcher_status=output_dir_unwritable message=failed to write into output directory"
rm -f "${OUTPUT_DIR}/.write-test"

command -v python3 >/dev/null 2>&1 || fail_phase preflight "backend=${BACKEND} launcher_status=missing_python3 message=python3 command not found"
command -v timeout >/dev/null 2>&1 || fail_phase preflight "backend=${BACKEND} launcher_status=missing_timeout message=timeout command not found"

if [[ -n "${IDXD_RUST_VERIFY_BINARY:-}" && "${SKIP_BUILD}" != "1" ]]; then
  fail_phase preflight "backend=${BACKEND} launcher_status=contradictory_overrides message=IDXD_RUST_VERIFY_BINARY requires IDXD_RUST_VERIFY_SKIP_BUILD=1 so the verifier does not build one binary and execute another"
fi

if [[ "${SKIP_BUILD}" != "1" ]]; then
  log_phase build "backend=${BACKEND} workspace=${REPO_ROOT} binary=${BINARY_PATH} profile=${BUILD_PROFILE}"
  (
    cd "${REPO_ROOT}"
    cargo build --profile "${BUILD_PROFILE}" -p idxd-rust --bin tokio_memmove_bench
  )
fi

[[ -x "${BINARY_PATH}" ]] || fail_phase preflight "backend=${BACKEND} launcher_status=missing_binary binary=${BINARY_PATH} message=tokio_memmove_bench binary is not executable"

DEVICE_PATH=${IDXD_RUST_VERIFY_DEVICE:-/dev/dsa/wq0.0}
LAUNCHER_STATUS=not_required
RUNNER=("${BINARY_PATH}")

if [[ "${BACKEND}" == "hardware" ]]; then
  DEVICE_PATH=$(find_default_device) || complete_with_explicit_failure preflight 'backend=hardware device_path=<none> launcher_status=missing_work_queue message=no /dev/dsa/wq* device found; set IDXD_RUST_VERIFY_DEVICE explicitly'
  command -v devenv >/dev/null 2>&1 || complete_with_explicit_failure preflight "backend=hardware device_path=${DEVICE_PATH} launcher_status=missing_devenv message=devenv command not found"
  [[ -x "${LAUNCHER_PATH}" ]] || complete_with_explicit_failure preflight "backend=hardware device_path=${DEVICE_PATH} launcher_status=missing_launcher launcher_path=${LAUNCHER_PATH} message=build the launcher with launch in a privileged shell before running this verifier"

  if command -v getcap >/dev/null 2>&1; then
    LAUNCHER_CAPS=$(getcap "${LAUNCHER_PATH}" || true)
    if [[ "${LAUNCHER_CAPS}" != *"cap_sys_rawio"* ]]; then
      complete_with_explicit_failure preflight "backend=hardware device_path=${DEVICE_PATH} launcher_status=missing_capability launcher_path=${LAUNCHER_PATH} message=launcher lacks cap_sys_rawio+eip"
    fi
    LAUNCHER_STATUS=ready
  else
    LAUNCHER_CAPS=unavailable
    LAUNCHER_STATUS=capability_unchecked
  fi

  log_phase preflight "backend=hardware device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} binary=${BINARY_PATH} timeout=${PREFLIGHT_TIMEOUT}"

  PREFLIGHT_EXIT=0
  if timeout "${PREFLIGHT_TIMEOUT}" \
    devenv shell -- launch "${BINARY_PATH}" --bytes abc \
    >"${PREFLIGHT_STDOUT_PATH}" 2>"${PREFLIGHT_STDERR_PATH}"; then
    PREFLIGHT_EXIT=0
  else
    PREFLIGHT_EXIT=$?
  fi

  if [[ "${PREFLIGHT_EXIT}" -eq 124 ]]; then
    fail_phase preflight "backend=hardware device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} stdout=${PREFLIGHT_STDOUT_PATH} stderr=${PREFLIGHT_STDERR_PATH} message=launch-wrapped preflight exceeded timeout"
  fi

  if [[ "${PREFLIGHT_EXIT}" -ne 2 ]] || ! grep -q 'invalid value `abc` for `--bytes`' "${PREFLIGHT_STDERR_PATH}"; then
    fail_phase preflight "backend=hardware device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} stdout=${PREFLIGHT_STDOUT_PATH} stderr=${PREFLIGHT_STDERR_PATH} message=launch-wrapped preflight failed"
  fi

  RUNNER=(devenv shell -- launch "${BINARY_PATH}")
fi

log_phase runtime "backend=${BACKEND} suite=${SUITE} device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} binary=${BINARY_PATH} requested_bytes=${REQUEST_BYTES} iterations=${ITERATIONS} concurrency=${CONCURRENCY} duration_ms=${DURATION_MS} timeout=${RUN_TIMEOUT}"

RUN_EXIT=0
if timeout "${RUN_TIMEOUT}" \
  "${RUNNER[@]}" \
    --backend "${BACKEND}" \
    --device "${DEVICE_PATH}" \
    --suite "${SUITE}" \
    --bytes "${REQUEST_BYTES}" \
    --iterations "${ITERATIONS}" \
    --concurrency "${CONCURRENCY}" \
    --duration-ms "${DURATION_MS}" \
    --format json \
    --artifact "${ARTIFACT_PATH}" \
    >"${STDOUT_PATH}" 2>"${STDERR_PATH}"; then
  RUN_EXIT=0
else
  RUN_EXIT=$?
fi

if [[ "${RUN_EXIT}" -eq 124 ]]; then
  fail_phase runtime "backend=${BACKEND} suite=${SUITE} device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} stdout=${STDOUT_PATH} stderr=${STDERR_PATH} message=benchmark exceeded timeout"
fi

ARTIFACT_FIELDS=$(python3 - <<'PY' "${ARTIFACT_PATH}" "${STDOUT_PATH}" "${BACKEND}" "${SUITE}" "${DEVICE_PATH}" "${REQUEST_BYTES}" "${ITERATIONS}" "${CONCURRENCY}" "${DURATION_MS}"
import json
import math
import re
import sys
from pathlib import Path

artifact_path = Path(sys.argv[1])
stdout_path = Path(sys.argv[2])
expected_backend = sys.argv[3]
expected_suite = sys.argv[4]
expected_device = sys.argv[5]
expected_bytes = int(sys.argv[6])
expected_iterations = int(sys.argv[7])
expected_concurrency = int(sys.argv[8])
expected_duration_ms = int(sys.argv[9])

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

required_top = {
    'schema_version', 'ok', 'verdict', 'device_path', 'backend', 'claim_eligible',
    'suite', 'runtime_flavor', 'worker_threads', 'requested_bytes', 'iterations',
    'concurrency', 'duration_ms', 'failure_class', 'error_kind',
    'direct_failure_kind', 'validation_phase', 'validation_error_kind',
    'direct_retry_budget', 'direct_retry_count', 'completion_status',
    'completion_result', 'completion_bytes_completed', 'completion_fault_addr',
    'results',
}
missing = sorted(required_top - report.keys())
if missing:
    raise SystemExit(f"artifact missing required top-level fields: {', '.join(missing)}")

if report['schema_version'] != 1:
    raise SystemExit('schema_version must be 1')
if not isinstance(report['ok'], bool):
    raise SystemExit('ok must be boolean')
if report['backend'] != expected_backend:
    raise SystemExit(f"backend={report['backend']!r} expected {expected_backend!r}")
if report['suite'] != expected_suite:
    raise SystemExit(f"suite={report['suite']!r} expected {expected_suite!r}")
if report['device_path'] != expected_device:
    raise SystemExit(f"device_path={report['device_path']!r} expected {expected_device!r}")
for key, expected in [
    ('requested_bytes', expected_bytes),
    ('iterations', expected_iterations),
    ('concurrency', expected_concurrency),
    ('duration_ms', expected_duration_ms),
]:
    if report[key] != expected:
        raise SystemExit(f"{key}={report[key]!r} expected {expected!r}")
if report['runtime_flavor'] != 'current_thread':
    raise SystemExit('runtime_flavor must be current_thread')
if report['worker_threads'] != 1:
    raise SystemExit('worker_threads must be 1')
if not isinstance(report['results'], list):
    raise SystemExit('results must be array')

valid_top_verdicts = {'pass', 'fail', 'expected_failure'}
if report['verdict'] not in valid_top_verdicts:
    raise SystemExit('verdict must be pass, fail, or expected_failure')
if report['ok'] and report['verdict'] != 'pass':
    raise SystemExit('ok artifacts must use verdict=pass')
if not report['ok'] and report['verdict'] == 'pass':
    raise SystemExit('failed artifacts must not use verdict=pass')
if report['ok'] and not report['results']:
    raise SystemExit('successful artifacts must include result rows')

if expected_backend == 'software':
    if report['claim_eligible'] is not False:
        raise SystemExit('software artifacts must not be claim eligible')
    if any(row.get('claim_eligible') is not False for row in report['results']):
        raise SystemExit('software result rows must not be claim eligible')
    if any(row.get('target') != 'software_direct_async_diagnostic' for row in report['results']):
        raise SystemExit('software rows must target software_direct_async_diagnostic')
else:
    if report['ok'] and report['claim_eligible'] is not True:
        raise SystemExit('successful hardware artifacts must be claim eligible')
    if not report['ok'] and report['claim_eligible'] is not False:
        raise SystemExit('failed hardware artifacts must not be claim eligible')

row_required = {
    'mode', 'target', 'comparison_target', 'requested_bytes', 'iterations',
    'concurrency', 'duration_ms', 'completed_operations', 'failed_operations',
    'elapsed_ns', 'min_latency_ns', 'mean_latency_ns', 'max_latency_ns',
    'ops_per_sec', 'bytes_per_sec', 'verdict', 'failure_class', 'error_kind',
    'direct_failure_kind', 'validation_phase', 'validation_error_kind',
    'direct_retry_budget', 'direct_retry_count', 'completion_status',
    'completion_result', 'completion_bytes_completed', 'completion_fault_addr',
    'claim_eligible',
}
canonical_modes = {'single_latency', 'concurrent_submissions', 'fixed_duration_throughput'}
expected_modes = {
    'canonical': canonical_modes,
    'latency': {'single_latency'},
    'concurrency': {'concurrent_submissions'},
    'throughput': {'fixed_duration_throughput'},
}[expected_suite]
async_modes = set()
sync_comparison_seen = False
completed_operations = 0
failed_operations = 0
row_targets = []

for index, row in enumerate(report['results']):
    if not isinstance(row, dict):
        raise SystemExit(f'result row {index} must be object')
    missing_row = sorted(row_required - row.keys())
    if missing_row:
        raise SystemExit(f"result row {index} missing required fields: {', '.join(missing_row)}")
    if row['requested_bytes'] != expected_bytes:
        raise SystemExit(f'result row {index} requested_bytes mismatch')
    if row['iterations'] != expected_iterations:
        raise SystemExit(f'result row {index} iterations mismatch')
    if row['concurrency'] != expected_concurrency:
        raise SystemExit(f'result row {index} concurrency mismatch')
    if row['duration_ms'] != expected_duration_ms:
        raise SystemExit(f'result row {index} duration_ms mismatch')
    if row['verdict'] not in {'pass', 'fail'}:
        raise SystemExit(f'result row {index} verdict must be pass or fail')
    if not isinstance(row['completed_operations'], int) or row['completed_operations'] < 0:
        raise SystemExit(f'result row {index} completed_operations must be non-negative integer')
    if not isinstance(row['failed_operations'], int) or row['failed_operations'] < 0:
        raise SystemExit(f'result row {index} failed_operations must be non-negative integer')
    if not isinstance(row['elapsed_ns'], int) or row['elapsed_ns'] <= 0:
        raise SystemExit(f'result row {index} elapsed_ns must be positive integer')
    if row['verdict'] == 'pass':
        if row['completed_operations'] <= 0:
            raise SystemExit(f'result row {index} pass requires completed_operations > 0')
        if row['failed_operations'] != 0:
            raise SystemExit(f'result row {index} pass requires failed_operations=0')
        for nullable in ['failure_class', 'error_kind', 'direct_failure_kind', 'validation_phase', 'validation_error_kind', 'completion_status', 'completion_result', 'completion_bytes_completed', 'completion_fault_addr']:
            if row[nullable] is not None:
                raise SystemExit(f'result row {index} pass requires {nullable}=null')
    else:
        if row['failed_operations'] <= 0:
            raise SystemExit(f'result row {index} fail requires failed_operations > 0')
        if not isinstance(row['failure_class'], str) or not row['failure_class']:
            raise SystemExit(f'result row {index} fail requires failure_class')
        if not isinstance(row['error_kind'], str) or not row['error_kind']:
            raise SystemExit(f'result row {index} fail requires error_kind')
    for key in ['min_latency_ns', 'mean_latency_ns', 'max_latency_ns']:
        if row[key] is not None and (not isinstance(row[key], int) or row[key] <= 0):
            raise SystemExit(f'result row {index} {key} must be null or positive integer')
    for key in ['ops_per_sec', 'bytes_per_sec']:
        if row[key] is not None and (not isinstance(row[key], (int, float)) or not math.isfinite(float(row[key])) or float(row[key]) <= 0.0):
            raise SystemExit(f'result row {index} {key} must be null or positive number')
    if row['completion_status'] is not None and not re.fullmatch(r'0x[0-9a-f]{2}', row['completion_status']):
        raise SystemExit(f'result row {index} completion_status must be null or 0xNN')
    if row['completion_fault_addr'] is not None and not re.fullmatch(r'0x[0-9a-f]+', row['completion_fault_addr']):
        raise SystemExit(f'result row {index} completion_fault_addr must be null or hex')

    target = row['target']
    row_targets.append(target)
    completed_operations += row['completed_operations']
    failed_operations += row['failed_operations']
    if expected_backend == 'hardware':
        if target == 'direct_async':
            async_modes.add(row['mode'])
            if row['claim_eligible'] != (row['verdict'] == 'pass'):
                raise SystemExit('hardware direct_async row claim eligibility must match pass verdict')
            if row['mode'] == 'single_latency' and row['comparison_target'] != 'direct_sync':
                raise SystemExit('hardware single_latency async row must name direct_sync comparison_target')
        elif target == 'direct_sync':
            sync_comparison_seen = True
            if row['mode'] != 'single_latency':
                raise SystemExit('direct_sync comparison row must use single_latency mode')
            if row['comparison_target'] != 'direct_async':
                raise SystemExit('direct_sync comparison row must name direct_async comparison_target')
            if row['claim_eligible'] != (row['verdict'] == 'pass'):
                raise SystemExit('direct_sync row claim eligibility must match pass verdict')
        else:
            raise SystemExit(f'unexpected hardware row target {target!r}')

if report['ok']:
    if failed_operations != 0:
        raise SystemExit('successful artifact cannot contain failed operations')
    if expected_backend == 'hardware':
        if async_modes != expected_modes:
            raise SystemExit(f'hardware success missing direct async modes: saw {sorted(async_modes)} expected {sorted(expected_modes)}')
        if expected_suite in {'canonical', 'latency'} and not sync_comparison_seen:
            raise SystemExit('hardware success missing direct sync comparison row')
    elif expected_suite == 'canonical':
        modes = {row['mode'] for row in report['results']}
        if modes != canonical_modes:
            raise SystemExit(f'software canonical missing modes: saw {sorted(modes)}')
else:
    if not isinstance(report['failure_class'], str) or not report['failure_class']:
        raise SystemExit('failed artifact requires top-level failure_class')
    if not isinstance(report['error_kind'], str) or not report['error_kind']:
        raise SystemExit('failed artifact requires top-level error_kind')

print(f"ok={str(report['ok']).lower()}")
print(f"verdict={report['verdict']}")
print(f"backend={report['backend']}")
print(f"suite={report['suite']}")
print(f"claim_eligible={str(report['claim_eligible']).lower()}")
print(f"failure_class={report['failure_class'] if report['failure_class'] is not None else 'null'}")
print(f"error_kind={report['error_kind'] if report['error_kind'] is not None else 'null'}")
print(f"direct_failure_kind={report['direct_failure_kind'] if report['direct_failure_kind'] is not None else 'null'}")
print(f"validation_phase={report['validation_phase'] if report['validation_phase'] is not None else 'null'}")
print(f"validation_error_kind={report['validation_error_kind'] if report['validation_error_kind'] is not None else 'null'}")
print(f"completed_operations={completed_operations}")
print(f"failed_operations={failed_operations}")
print(f"targets={','.join(row_targets) if row_targets else 'none'}")
PY
) || fail_phase artifact_validation "backend=${BACKEND} suite=${SUITE} device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} stdout=${STDOUT_PATH} stderr=${STDERR_PATH} message=artifact validation failed"

ARTIFACT_OK=
ARTIFACT_VERDICT=
ARTIFACT_BACKEND=
ARTIFACT_SUITE=
ARTIFACT_CLAIM_ELIGIBLE=
ARTIFACT_FAILURE_CLASS=
ARTIFACT_ERROR_KIND=
ARTIFACT_DIRECT_FAILURE_KIND=
ARTIFACT_VALIDATION_PHASE=
ARTIFACT_VALIDATION_ERROR_KIND=
ARTIFACT_COMPLETED_OPERATIONS=
ARTIFACT_FAILED_OPERATIONS=
ARTIFACT_TARGETS=
while IFS='=' read -r key value; do
  case "${key}" in
    ok) ARTIFACT_OK=${value} ;;
    verdict) ARTIFACT_VERDICT=${value} ;;
    backend) ARTIFACT_BACKEND=${value} ;;
    suite) ARTIFACT_SUITE=${value} ;;
    claim_eligible) ARTIFACT_CLAIM_ELIGIBLE=${value} ;;
    failure_class) ARTIFACT_FAILURE_CLASS=${value} ;;
    error_kind) ARTIFACT_ERROR_KIND=${value} ;;
    direct_failure_kind) ARTIFACT_DIRECT_FAILURE_KIND=${value} ;;
    validation_phase) ARTIFACT_VALIDATION_PHASE=${value} ;;
    validation_error_kind) ARTIFACT_VALIDATION_ERROR_KIND=${value} ;;
    completed_operations) ARTIFACT_COMPLETED_OPERATIONS=${value} ;;
    failed_operations) ARTIFACT_FAILED_OPERATIONS=${value} ;;
    targets) ARTIFACT_TARGETS=${value} ;;
  esac
done <<< "${ARTIFACT_FIELDS}"

log_phase artifact_validation "backend=${ARTIFACT_BACKEND} suite=${ARTIFACT_SUITE} claim_eligible=${ARTIFACT_CLAIM_ELIGIBLE} failure_class=${ARTIFACT_FAILURE_CLASS} error_kind=${ARTIFACT_ERROR_KIND} direct_failure_kind=${ARTIFACT_DIRECT_FAILURE_KIND} validation_phase=${ARTIFACT_VALIDATION_PHASE} validation_error_kind=${ARTIFACT_VALIDATION_ERROR_KIND} completed_operations=${ARTIFACT_COMPLETED_OPERATIONS} failed_operations=${ARTIFACT_FAILED_OPERATIONS} targets=${ARTIFACT_TARGETS}"

if [[ "${ARTIFACT_OK}" != "true" ]]; then
  if [[ "${BACKEND}" == "hardware" ]]; then
    complete_with_explicit_failure runtime "backend=${ARTIFACT_BACKEND} suite=${ARTIFACT_SUITE} device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} claim_eligible=${ARTIFACT_CLAIM_ELIGIBLE} failure_class=${ARTIFACT_FAILURE_CLASS} error_kind=${ARTIFACT_ERROR_KIND} direct_failure_kind=${ARTIFACT_DIRECT_FAILURE_KIND} validation_phase=${ARTIFACT_VALIDATION_PHASE} validation_error_kind=${ARTIFACT_VALIDATION_ERROR_KIND} completed_operations=${ARTIFACT_COMPLETED_OPERATIONS} failed_operations=${ARTIFACT_FAILED_OPERATIONS} targets=${ARTIFACT_TARGETS} stdout=${STDOUT_PATH} stderr=${STDERR_PATH} message=hardware benchmark reported classified failure"
  fi
  fail_phase runtime "backend=${ARTIFACT_BACKEND} suite=${ARTIFACT_SUITE} device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} stdout=${STDOUT_PATH} stderr=${STDERR_PATH} failure_class=${ARTIFACT_FAILURE_CLASS} error_kind=${ARTIFACT_ERROR_KIND} message=software benchmark reported failure"
fi

if [[ "${RUN_EXIT}" -ne 0 ]]; then
  fail_phase runtime "backend=${ARTIFACT_BACKEND} suite=${ARTIFACT_SUITE} device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} stdout=${STDOUT_PATH} stderr=${STDERR_PATH} failure_class=${ARTIFACT_FAILURE_CLASS} error_kind=${ARTIFACT_ERROR_KIND} message=benchmark exited non-zero despite a success artifact"
fi

log_phase done "backend=${ARTIFACT_BACKEND} suite=${ARTIFACT_SUITE} device_path=${DEVICE_PATH} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} requested_bytes=${REQUEST_BYTES} iterations=${ITERATIONS} concurrency=${CONCURRENCY} duration_ms=${DURATION_MS} claim_eligible=${ARTIFACT_CLAIM_ELIGIBLE} failure_class=${ARTIFACT_FAILURE_CLASS} error_kind=${ARTIFACT_ERROR_KIND} direct_failure_kind=${ARTIFACT_DIRECT_FAILURE_KIND} validation_phase=${ARTIFACT_VALIDATION_PHASE} validation_error_kind=${ARTIFACT_VALIDATION_ERROR_KIND} completed_operations=${ARTIFACT_COMPLETED_OPERATIONS} failed_operations=${ARTIFACT_FAILED_OPERATIONS} targets=${ARTIFACT_TARGETS} stdout=${STDOUT_PATH} stderr=${STDERR_PATH} verdict=pass"
