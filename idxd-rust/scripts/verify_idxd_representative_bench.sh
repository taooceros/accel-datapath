#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
CRATE_DIR=$(cd -- "${SCRIPT_DIR}/.." && pwd)
REPO_ROOT=$(cd -- "${CRATE_DIR}/.." && pwd)

OUTPUT_DIR=${IDXD_RUST_VERIFY_OUTPUT_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/idxd-rust-representative-bench.XXXXXX")}
REQUEST_BYTES=${IDXD_RUST_VERIFY_BYTES:-4096}
ITERATIONS=${IDXD_RUST_VERIFY_ITERATIONS:-1000}
BUILD_PROFILE=${IDXD_RUST_VERIFY_PROFILE:-release}
TARGET_SUBDIR=${BUILD_PROFILE}
PREFLIGHT_TIMEOUT=${IDXD_RUST_VERIFY_PREFLIGHT_TIMEOUT:-20s}
BUILD_TIMEOUT=${IDXD_RUST_VERIFY_BUILD_TIMEOUT:-120s}
RUN_TIMEOUT=${IDXD_RUST_VERIFY_RUN_TIMEOUT:-60s}
SKIP_BUILD=${IDXD_RUST_VERIFY_SKIP_BUILD:-0}
LAUNCHER_PATH=${IDXD_RUST_VERIFY_LAUNCHER_PATH:-${REPO_ROOT}/tools/build/dsa_launcher}
BINARY_PATH=${IDXD_RUST_VERIFY_BINARY:-${REPO_ROOT}/target/${TARGET_SUBDIR}/idxd_representative_bench}
ARTIFACT_PATH="${OUTPUT_DIR}/idxd_representative_bench.json"
STDOUT_PATH="${OUTPUT_DIR}/idxd_representative_bench.stdout"
RAW_STDOUT_PATH="${STDOUT_PATH}.raw"
STDERR_PATH="${OUTPUT_DIR}/idxd_representative_bench.stderr"
PREFLIGHT_STDOUT_PATH="${OUTPUT_DIR}/preflight.stdout"
PREFLIGHT_STDERR_PATH="${OUTPUT_DIR}/preflight.stderr"

DSA_DEVICE=
IAX_DEVICE=
SHARED_DSA_DEVICE=
LAUNCHER_STATUS=unknown

join_csv() {
  local IFS=,
  if [[ $# -eq 0 ]]; then
    printf '<none>'
  else
    printf '%s' "$*"
  fi
}

target_list() {
  local -a entries=("dsa-memmove:${DSA_DEVICE:-<missing>}" "iax-crc64:${IAX_DEVICE:-<missing>}")
  if [[ -n "${SHARED_DSA_DEVICE:-}" ]]; then
    entries+=("dsa-shared-memmove:${SHARED_DSA_DEVICE}")
  fi
  join_csv "${entries[@]}"
}

log_phase() {
  local phase=$1
  shift
  printf '[verify_idxd_representative_bench] phase=%s output_dir=%s artifact=%s stdout=%s stderr=%s raw_stdout=%s %s\n' \
    "${phase}" \
    "${OUTPUT_DIR}" \
    "${ARTIFACT_PATH}" \
    "${STDOUT_PATH}" \
    "${STDERR_PATH}" \
    "${RAW_STDOUT_PATH}" \
    "$*"
}

fail_phase() {
  local phase=$1
  shift
  printf '[verify_idxd_representative_bench] phase=%s output_dir=%s artifact=%s stdout=%s stderr=%s raw_stdout=%s targets=%s %s\n' \
    "${phase}" \
    "${OUTPUT_DIR}" \
    "${ARTIFACT_PATH}" \
    "${STDOUT_PATH}" \
    "${STDERR_PATH}" \
    "${RAW_STDOUT_PATH}" \
    "$(target_list)" \
    "$*" >&2
  exit 1
}

complete_with_explicit_failure() {
  local phase=$1
  shift
  log_phase done "verdict=expected_failure failure_phase=${phase} launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} profile=${BUILD_PROFILE} requested_bytes=${REQUEST_BYTES} iterations=${ITERATIONS} claim_eligible=false targets=$(target_list) $*"
  exit 0
}

is_positive_integer() {
  [[ ${1:-} =~ ^[1-9][0-9]*$ ]]
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

normalize_launch_stdout() {
  local raw_stdout_path=$1
  local normalized_stdout_path=$2

  if [[ ! -f "${raw_stdout_path}" ]]; then
    return 0
  fi

  # The repo's `launch` wrapper prints its own "Running: .../dsa_launcher ..."
  # banner to stdout before execing the proof binary. Keep stdout as the
  # benchmark JSON contract and preserve the raw launcher stream beside it.
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

validate_artifact_contract() {
  python3 - <<'PY' "${ARTIFACT_PATH}" "${STDOUT_PATH}" "${BUILD_PROFILE}" "${REQUEST_BYTES}" "${ITERATIONS}" "${DSA_DEVICE}" "${IAX_DEVICE}" "${SHARED_DSA_DEVICE:-}"
import json
import math
import re
import sys
from pathlib import Path

artifact_path = Path(sys.argv[1])
stdout_path = Path(sys.argv[2])
expected_profile = sys.argv[3]
expected_bytes = int(sys.argv[4])
expected_iterations = int(sys.argv[5])
expected_dsa_device = sys.argv[6]
expected_iax_device = sys.argv[7]
expected_shared_device = sys.argv[8]

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

if not isinstance(report, dict):
    raise SystemExit('artifact top-level must be a JSON object')

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

required_top = {
    'schema_version',
    'ok',
    'verdict',
    'claim_eligible',
    'suite',
    'profile',
    'requested_bytes',
    'iterations',
    'warmup_iterations',
    'clock',
    'failure_phase',
    'error_kind',
    'failure_target',
    'failure_accelerator',
    'targets',
}
missing_top = sorted(required_top - report.keys())
if missing_top:
    raise SystemExit(f"artifact missing required top-level fields: {', '.join(missing_top)}")

if report['schema_version'] != 1:
    raise SystemExit('schema_version must be 1')
if report['suite'] != 'idxd_representative_bench':
    raise SystemExit('suite must be idxd_representative_bench')
if expected_profile != 'release':
    raise SystemExit('verifier expected profile must be release')
if report['profile'] != expected_profile or report['profile'] != 'release':
    raise SystemExit(f"artifact profile={report['profile']!r} expected release")
if report['requested_bytes'] != expected_bytes:
    raise SystemExit(f"requested_bytes={report['requested_bytes']!r} expected {expected_bytes!r}")
if report['iterations'] != expected_iterations:
    raise SystemExit(f"iterations={report['iterations']!r} expected {expected_iterations!r}")
if not isinstance(report['warmup_iterations'], int) or report['warmup_iterations'] <= 0:
    raise SystemExit('warmup_iterations must be a positive integer')
if not isinstance(report['clock'], str) or not report['clock']:
    raise SystemExit('clock must be a non-empty string')
if not isinstance(report['ok'], bool):
    raise SystemExit('ok must be boolean')
if report['verdict'] not in {'pass', 'fail', 'expected_failure'}:
    raise SystemExit('verdict must be pass, fail, or expected_failure')
if not isinstance(report['claim_eligible'], bool):
    raise SystemExit('claim_eligible must be boolean')
if not isinstance(report['targets'], list):
    raise SystemExit('targets must be an array')

expected_rows = {
    'dsa-memmove': {
        'operation': 'memmove',
        'family': 'dsa',
        'device_path': expected_dsa_device,
        'target_role': 'required',
        'crc_required': False,
    },
    'iax-crc64': {
        'operation': 'crc64',
        'family': 'iax',
        'device_path': expected_iax_device,
        'target_role': 'required',
        'crc_required': True,
    },
}
if expected_shared_device:
    expected_rows['dsa-shared-memmove'] = {
        'operation': 'memmove',
        'family': 'dsa',
        'device_path': expected_shared_device,
        'target_role': 'optional-shared',
        'crc_required': False,
    }

row_required = {
    'target',
    'operation',
    'family',
    'device_path',
    'work_queue_mode',
    'target_role',
    'requested_bytes',
    'iterations',
    'warmup_iterations',
    'ok',
    'verdict',
    'claim_eligible',
    'completed_operations',
    'failed_operations',
    'elapsed_ns',
    'min_latency_ns',
    'mean_latency_ns',
    'max_latency_ns',
    'ops_per_sec',
    'bytes_per_sec',
    'total_page_fault_retries',
    'last_page_fault_retries',
    'final_status',
    'completion_error_code',
    'invalid_flags',
    'fault_addr',
    'crc64',
    'expected_crc64',
    'crc64_verified',
    'failure_phase',
    'error_kind',
    'message',
}


def is_non_bool_int(value):
    return isinstance(value, int) and not isinstance(value, bool)


def is_positive_int(value):
    return is_non_bool_int(value) and value > 0


def is_non_negative_int(value):
    return is_non_bool_int(value) and value >= 0


def is_positive_number(value):
    return (
        isinstance(value, (int, float))
        and not isinstance(value, bool)
        and math.isfinite(float(value))
        and float(value) > 0.0
    )


def is_hex(value, digits=None):
    if not isinstance(value, str):
        return False
    pattern = r'0x[0-9a-f]+'
    if digits is not None:
        pattern = rf'0x[0-9a-f]{{{digits}}}'
    return re.fullmatch(pattern, value) is not None


seen = {}
completed_operations = 0
failed_operations = 0
row_targets = []

for index, row in enumerate(report['targets']):
    if not isinstance(row, dict):
        raise SystemExit(f'target row {index} must be object')
    missing_row = sorted(row_required - row.keys())
    if missing_row:
        raise SystemExit(f"target row {index} missing required fields: {', '.join(missing_row)}")

    target = row['target']
    if target not in expected_rows:
        raise SystemExit(f'unexpected benchmark target {target!r}')
    if target in seen:
        raise SystemExit(f'duplicate benchmark target {target!r}')
    expected = expected_rows[target]
    seen[target] = row
    row_targets.append(target)

    if row['operation'] != expected['operation']:
        raise SystemExit(f"target {target} operation={row['operation']!r} expected {expected['operation']!r}")
    if row['family'] != expected['family']:
        raise SystemExit(f"target {target} family={row['family']!r} expected {expected['family']!r}")
    if row['device_path'] != expected['device_path']:
        raise SystemExit(f"target {target} device_path={row['device_path']!r} expected {expected['device_path']!r}")
    if row['target_role'] != expected['target_role']:
        raise SystemExit(f"target {target} target_role={row['target_role']!r} expected {expected['target_role']!r}")
    if row['requested_bytes'] != expected_bytes:
        raise SystemExit(f'target {target} requested_bytes mismatch')
    if row['iterations'] != expected_iterations:
        raise SystemExit(f'target {target} iterations mismatch')
    if row['warmup_iterations'] != report['warmup_iterations']:
        raise SystemExit(f'target {target} warmup_iterations mismatch')
    if not isinstance(row['ok'], bool):
        raise SystemExit(f'target {target} ok must be boolean')
    if row['verdict'] not in {'pass', 'fail', 'expected_failure'}:
        raise SystemExit(f'target {target} verdict must be pass, fail, or expected_failure')
    if not isinstance(row['claim_eligible'], bool):
        raise SystemExit(f'target {target} claim_eligible must be boolean')
    if not is_non_negative_int(row['completed_operations']):
        raise SystemExit(f'target {target} completed_operations must be a non-negative integer')
    if not is_non_negative_int(row['failed_operations']):
        raise SystemExit(f'target {target} failed_operations must be a non-negative integer')
    if not isinstance(row['message'], str) or not row['message']:
        raise SystemExit(f'target {target} message must be a non-empty string')

    completed_operations += row['completed_operations']
    failed_operations += row['failed_operations']

    if row['ok']:
        if row['verdict'] != 'pass':
            raise SystemExit(f'target {target} ok=true requires verdict=pass')
        if row['claim_eligible'] is not True:
            raise SystemExit(f'target {target} pass row must be claim eligible in release profile')
        if row['completed_operations'] != expected_iterations:
            raise SystemExit(f'target {target} pass requires completed_operations == iterations')
        if row['failed_operations'] != 0:
            raise SystemExit(f'target {target} pass requires failed_operations=0')
        for key in ['elapsed_ns', 'min_latency_ns', 'mean_latency_ns', 'max_latency_ns']:
            if not is_positive_int(row[key]):
                raise SystemExit(f'target {target} {key} must be a positive integer on pass')
        for key in ['ops_per_sec', 'bytes_per_sec']:
            if not is_positive_number(row[key]):
                raise SystemExit(f'target {target} {key} must be a positive finite number on pass')
        if not is_non_negative_int(row['total_page_fault_retries']):
            raise SystemExit(f'target {target} total_page_fault_retries must be non-negative on pass')
        if not is_non_negative_int(row['last_page_fault_retries']):
            raise SystemExit(f'target {target} last_page_fault_retries must be non-negative on pass')
        if not is_hex(row['final_status'], 2):
            raise SystemExit(f'target {target} final_status must be a 0xNN string on pass')
        for nullable in ['completion_error_code', 'invalid_flags', 'fault_addr', 'failure_phase', 'error_kind']:
            if row[nullable] is not None:
                raise SystemExit(f'target {target} pass requires {nullable}=null')
        if row['work_queue_mode'] not in {'dedicated', 'shared'}:
            raise SystemExit(f'target {target} pass requires dedicated/shared work_queue_mode')
        if expected['crc_required']:
            if not is_hex(row['crc64']) or not is_hex(row['expected_crc64']):
                raise SystemExit(f'target {target} pass requires hex crc64 and expected_crc64')
            if row['crc64_verified'] is not True:
                raise SystemExit(f'target {target} pass requires crc64_verified=true')
        else:
            if row['crc64'] is not None or row['expected_crc64'] is not None or row['crc64_verified'] is not None:
                raise SystemExit(f'target {target} DSA pass must not carry CRC fields')
    else:
        if row['verdict'] == 'pass':
            raise SystemExit(f'target {target} failed row must not use verdict=pass')
        if row['claim_eligible'] is not False:
            raise SystemExit(f'target {target} failed row must not be claim eligible')
        if row['failed_operations'] <= 0:
            raise SystemExit(f'target {target} failed row requires failed_operations > 0')
        if not isinstance(row['failure_phase'], str) or not row['failure_phase']:
            raise SystemExit(f'target {target} failed row requires failure_phase')
        if not isinstance(row['error_kind'], str) or not row['error_kind']:
            raise SystemExit(f'target {target} failed row requires error_kind')
        for key in ['elapsed_ns', 'min_latency_ns', 'mean_latency_ns', 'max_latency_ns', 'ops_per_sec', 'bytes_per_sec']:
            if row[key] is not None:
                raise SystemExit(f'target {target} failed row requires {key}=null')
        if row['total_page_fault_retries'] is not None and not is_non_negative_int(row['total_page_fault_retries']):
            raise SystemExit(f'target {target} total_page_fault_retries must be null or non-negative')
        if row['last_page_fault_retries'] is not None and not is_non_negative_int(row['last_page_fault_retries']):
            raise SystemExit(f'target {target} last_page_fault_retries must be null or non-negative')
        if row['final_status'] is not None and not is_hex(row['final_status'], 2):
            raise SystemExit(f'target {target} final_status must be null or a 0xNN string')
        if row['completion_error_code'] is not None and not is_hex(row['completion_error_code'], 2):
            raise SystemExit(f'target {target} completion_error_code must be null or a 0xNN string')
        if row['invalid_flags'] is not None and not is_hex(row['invalid_flags'], 8):
            raise SystemExit(f'target {target} invalid_flags must be null or a 0xNNNNNNNN string')
        if row['fault_addr'] is not None and not is_hex(row['fault_addr']):
            raise SystemExit(f'target {target} fault_addr must be null or a hex string')
        if row['crc64_verified'] is not None and not isinstance(row['crc64_verified'], bool):
            raise SystemExit(f'target {target} crc64_verified must be null or boolean')

missing_targets = sorted(set(expected_rows) - set(seen))
if missing_targets:
    raise SystemExit(f"artifact missing required benchmark targets: {', '.join(missing_targets)}")

all_expected_rows_passed = all(row['ok'] for row in seen.values())
if report['ok'] != all_expected_rows_passed:
    raise SystemExit('top-level ok must match configured target pass state')
expected_top_claim = all_expected_rows_passed and report['profile'] == 'release'
if report['claim_eligible'] is not expected_top_claim:
    raise SystemExit('top-level claim_eligible must require release profile and all configured targets passing')

if report['ok']:
    if report['verdict'] != 'pass':
        raise SystemExit('ok=true artifact requires verdict=pass')
    if failed_operations != 0:
        raise SystemExit('successful artifact cannot contain failed operations')
    for nullable in ['failure_phase', 'error_kind', 'failure_target', 'failure_accelerator']:
        if report[nullable] is not None:
            raise SystemExit(f'successful artifact requires {nullable}=null')
else:
    if report['verdict'] == 'pass':
        raise SystemExit('failed artifact must not use verdict=pass')
    if report['claim_eligible'] is not False:
        raise SystemExit('failed artifact must not be claim eligible')
    for key in ['failure_phase', 'error_kind', 'failure_target', 'failure_accelerator']:
        if not isinstance(report[key], str) or not report[key]:
            raise SystemExit(f'failed artifact requires top-level {key}')

print(f"ok={str(report['ok']).lower()}")
print(f"verdict={report['verdict']}")
print(f"profile={report['profile']}")
print(f"requested_bytes={report['requested_bytes']}")
print(f"iterations={report['iterations']}")
print(f"claim_eligible={str(report['claim_eligible']).lower()}")
print(f"failure_phase={report['failure_phase'] if report['failure_phase'] is not None else 'null'}")
print(f"error_kind={report['error_kind'] if report['error_kind'] is not None else 'null'}")
print(f"completed_operations={completed_operations}")
print(f"failed_operations={failed_operations}")
print(f"targets={','.join(row_targets) if row_targets else 'none'}")
PY
}

mkdir -p "${OUTPUT_DIR}" 2>/dev/null || fail_phase preflight 'launcher_status=output_dir_unwritable message=failed to create output directory'
touch "${OUTPUT_DIR}/.write-test" 2>/dev/null || fail_phase preflight 'launcher_status=output_dir_unwritable message=failed to write into output directory'
rm -f "${OUTPUT_DIR}/.write-test"

is_positive_integer "${REQUEST_BYTES}" || fail_phase preflight "launcher_status=invalid_bytes message=IDXD_RUST_VERIFY_BYTES must be a positive integer"
is_positive_integer "${ITERATIONS}" || fail_phase preflight "launcher_status=invalid_iterations message=IDXD_RUST_VERIFY_ITERATIONS must be a positive integer"

if [[ "${BUILD_PROFILE}" != "release" ]]; then
  fail_phase preflight "launcher_status=invalid_profile profile=${BUILD_PROFILE} message=IDXD_RUST_VERIFY_PROFILE must be release for representative benchmark closure evidence"
fi

DSA_DEVICE=$(find_dsa_device) || complete_with_explicit_failure preflight 'launcher_status=missing_work_queue missing_target=dsa-memmove message=no /dev/dsa/wq* device found; set IDXD_RUST_VERIFY_DSA_DEVICE explicitly'
IAX_DEVICE=$(find_iax_device) || complete_with_explicit_failure preflight 'launcher_status=missing_work_queue missing_target=iax-crc64 message=no /dev/iax/wq* device found; set IDXD_RUST_VERIFY_IAX_DEVICE explicitly'
SHARED_DSA_DEVICE=${IDXD_RUST_VERIFY_DSA_SHARED_DEVICE:-}

command -v python3 >/dev/null 2>&1 || complete_with_explicit_failure preflight 'launcher_status=missing_python3 message=python3 command not found'
command -v timeout >/dev/null 2>&1 || complete_with_explicit_failure preflight 'launcher_status=missing_timeout message=timeout command not found'
command -v devenv >/dev/null 2>&1 || complete_with_explicit_failure preflight 'launcher_status=missing_devenv message=devenv command not found'

if [[ -n "${IDXD_RUST_VERIFY_BINARY:-}" && "${SKIP_BUILD}" != "1" ]]; then
  fail_phase preflight 'launcher_status=contradictory_overrides message=IDXD_RUST_VERIFY_BINARY requires IDXD_RUST_VERIFY_SKIP_BUILD=1 so the verifier does not build one binary and execute another'
fi

if [[ "${SKIP_BUILD}" != "1" ]]; then
  log_phase build "launcher_status=building workspace=${REPO_ROOT} binary=${BINARY_PATH} profile=${BUILD_PROFILE} build_timeout=${BUILD_TIMEOUT} requested_bytes=${REQUEST_BYTES} iterations=${ITERATIONS} targets=$(target_list)"
  BUILD_EXIT=0
  if timeout "${BUILD_TIMEOUT}" cargo build --manifest-path "${REPO_ROOT}/Cargo.toml" --profile "${BUILD_PROFILE}" -p idxd-rust --bin idxd_representative_bench; then
    BUILD_EXIT=0
  else
    BUILD_EXIT=$?
  fi
  if [[ "${BUILD_EXIT}" -eq 124 ]]; then
    LAUNCHER_STATUS=build_timeout
    complete_with_explicit_failure build_timeout "binary=${BINARY_PATH} message=idxd_representative_bench build exceeded timeout"
  fi
  if [[ "${BUILD_EXIT}" -ne 0 ]]; then
    fail_phase build "launcher_status=build_failed binary=${BINARY_PATH} profile=${BUILD_PROFILE} exit_code=${BUILD_EXIT} message=failed to build idxd_representative_bench"
  fi
fi

[[ -x "${BINARY_PATH}" ]] || complete_with_explicit_failure preflight "launcher_status=missing_binary binary=${BINARY_PATH} message=idxd_representative_bench binary is not executable"
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

log_phase preflight "launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} binary=${BINARY_PATH} timeout=${PREFLIGHT_TIMEOUT} profile=${BUILD_PROFILE} requested_bytes=${REQUEST_BYTES} iterations=${ITERATIONS} targets=$(target_list)"

PREFLIGHT_EXIT=0
if timeout "${PREFLIGHT_TIMEOUT}" \
  devenv shell -- launch "${BINARY_PATH}" --bytes abc \
  >"${PREFLIGHT_STDOUT_PATH}" 2>"${PREFLIGHT_STDERR_PATH}"; then
  PREFLIGHT_EXIT=0
else
  PREFLIGHT_EXIT=$?
fi

if [[ "${PREFLIGHT_EXIT}" -eq 124 ]]; then
  complete_with_explicit_failure preflight_timeout "launcher_path=${LAUNCHER_PATH} stdout=${PREFLIGHT_STDOUT_PATH} stderr=${PREFLIGHT_STDERR_PATH} message=launch-wrapped preflight exceeded timeout"
fi

if [[ "${PREFLIGHT_EXIT}" -ne 2 ]] || ! grep -q 'invalid value `abc` for `--bytes`' "${PREFLIGHT_STDERR_PATH}"; then
  fail_phase preflight "launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} stdout=${PREFLIGHT_STDOUT_PATH} stderr=${PREFLIGHT_STDERR_PATH} message=launch-wrapped invalid-argument preflight failed"
fi

BENCH_ARGS=(
  --dsa-device "${DSA_DEVICE}"
  --iax-device "${IAX_DEVICE}"
  --bytes "${REQUEST_BYTES}"
  --iterations "${ITERATIONS}"
  --format json
  --artifact "${ARTIFACT_PATH}"
)
if [[ -n "${SHARED_DSA_DEVICE}" ]]; then
  BENCH_ARGS+=(--dsa-shared-device "${SHARED_DSA_DEVICE}")
fi

log_phase runtime "launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} binary=${BINARY_PATH} profile=${BUILD_PROFILE} requested_bytes=${REQUEST_BYTES} iterations=${ITERATIONS} targets=$(target_list) timeout=${RUN_TIMEOUT}"

RUN_EXIT=0
if timeout "${RUN_TIMEOUT}" \
  devenv shell -- launch "${BINARY_PATH}" \
    "${BENCH_ARGS[@]}" \
    >"${RAW_STDOUT_PATH}" 2>"${STDERR_PATH}"; then
  RUN_EXIT=0
else
  RUN_EXIT=$?
fi
normalize_launch_stdout "${RAW_STDOUT_PATH}" "${STDOUT_PATH}"

if [[ "${RUN_EXIT}" -eq 124 ]]; then
  complete_with_explicit_failure runtime_timeout "launcher_path=${LAUNCHER_PATH} dsa_device=${DSA_DEVICE} iax_device=${IAX_DEVICE} shared_dsa_device=${SHARED_DSA_DEVICE:-none} artifact=${ARTIFACT_PATH} stdout=${STDOUT_PATH} stderr=${STDERR_PATH} raw_stdout=${RAW_STDOUT_PATH} message=launch-wrapped representative benchmark exceeded timeout"
fi

ARTIFACT_FIELDS=$(validate_artifact_contract) || fail_phase artifact_validation "launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} profile=${BUILD_PROFILE} artifact=${ARTIFACT_PATH} stdout=${STDOUT_PATH} stderr=${STDERR_PATH} raw_stdout=${RAW_STDOUT_PATH} message=artifact validation failed"

ARTIFACT_OK=
ARTIFACT_VERDICT=
ARTIFACT_PROFILE=
ARTIFACT_REQUESTED_BYTES=
ARTIFACT_ITERATIONS=
ARTIFACT_CLAIM_ELIGIBLE=
ARTIFACT_FAILURE_PHASE=
ARTIFACT_ERROR_KIND=
ARTIFACT_COMPLETED_OPERATIONS=
ARTIFACT_FAILED_OPERATIONS=
ARTIFACT_TARGETS=
while IFS='=' read -r key value; do
  case "${key}" in
    ok) ARTIFACT_OK=${value} ;;
    verdict) ARTIFACT_VERDICT=${value} ;;
    profile) ARTIFACT_PROFILE=${value} ;;
    requested_bytes) ARTIFACT_REQUESTED_BYTES=${value} ;;
    iterations) ARTIFACT_ITERATIONS=${value} ;;
    claim_eligible) ARTIFACT_CLAIM_ELIGIBLE=${value} ;;
    failure_phase) ARTIFACT_FAILURE_PHASE=${value} ;;
    error_kind) ARTIFACT_ERROR_KIND=${value} ;;
    completed_operations) ARTIFACT_COMPLETED_OPERATIONS=${value} ;;
    failed_operations) ARTIFACT_FAILED_OPERATIONS=${value} ;;
    targets) ARTIFACT_TARGETS=${value} ;;
  esac
done <<< "${ARTIFACT_FIELDS}"

log_phase artifact_validation "launcher_status=${LAUNCHER_STATUS} profile=${ARTIFACT_PROFILE} requested_bytes=${ARTIFACT_REQUESTED_BYTES} iterations=${ARTIFACT_ITERATIONS} claim_eligible=${ARTIFACT_CLAIM_ELIGIBLE} failure_phase=${ARTIFACT_FAILURE_PHASE} error_kind=${ARTIFACT_ERROR_KIND} completed_operations=${ARTIFACT_COMPLETED_OPERATIONS} failed_operations=${ARTIFACT_FAILED_OPERATIONS} targets=${ARTIFACT_TARGETS}"

if [[ "${ARTIFACT_OK}" != "true" ]]; then
  if [[ "${RUN_EXIT}" -eq 0 ]]; then
    fail_phase runtime "launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} artifact_failure_phase=${ARTIFACT_FAILURE_PHASE} artifact_error_kind=${ARTIFACT_ERROR_KIND} artifact=${ARTIFACT_PATH} stdout=${STDOUT_PATH} stderr=${STDERR_PATH} message=idxd_representative_bench exited zero despite a failure artifact"
  fi
  complete_with_explicit_failure runtime "artifact_verdict=${ARTIFACT_VERDICT} artifact_failure_phase=${ARTIFACT_FAILURE_PHASE} artifact_error_kind=${ARTIFACT_ERROR_KIND} completed_operations=${ARTIFACT_COMPLETED_OPERATIONS} failed_operations=${ARTIFACT_FAILED_OPERATIONS} artifact_targets=${ARTIFACT_TARGETS} artifact=${ARTIFACT_PATH} stdout=${STDOUT_PATH} stderr=${STDERR_PATH} raw_stdout=${RAW_STDOUT_PATH} message=idxd_representative_bench reported benchmark failure"
fi

if [[ "${RUN_EXIT}" -ne 0 ]]; then
  fail_phase runtime "launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} artifact_verdict=${ARTIFACT_VERDICT} artifact_failure_phase=${ARTIFACT_FAILURE_PHASE} artifact_error_kind=${ARTIFACT_ERROR_KIND} artifact=${ARTIFACT_PATH} stdout=${STDOUT_PATH} stderr=${STDERR_PATH} message=idxd_representative_bench exited non-zero despite a success artifact"
fi

log_phase done "verdict=pass launcher_status=${LAUNCHER_STATUS} launcher_path=${LAUNCHER_PATH} profile=${ARTIFACT_PROFILE} requested_bytes=${ARTIFACT_REQUESTED_BYTES} iterations=${ARTIFACT_ITERATIONS} claim_eligible=${ARTIFACT_CLAIM_ELIGIBLE} completed_operations=${ARTIFACT_COMPLETED_OPERATIONS} failed_operations=${ARTIFACT_FAILED_OPERATIONS} targets=$(target_list) artifact_targets=${ARTIFACT_TARGETS} artifact=${ARTIFACT_PATH} stdout=${STDOUT_PATH} stderr=${STDERR_PATH} raw_stdout=${RAW_STDOUT_PATH}"
