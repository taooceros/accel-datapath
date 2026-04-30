#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
CRATE_DIR=$(cd -- "${SCRIPT_DIR}/.." && pwd)
REPO_ROOT=$(cd -- "${CRATE_DIR}/.." && pwd)

OUTPUT_DIR=${IDXD_RUST_BENCH_OUTPUT_DIR:-${REPO_ROOT}/target/async-throughput-matrix}
if [[ "${OUTPUT_DIR}" != /* ]]; then
  OUTPUT_DIR="${PWD}/${OUTPUT_DIR}"
fi
CSV_PATH=${IDXD_RUST_BENCH_CSV_PATH:-${OUTPUT_DIR}/async_throughput_matrix.csv}
if [[ "${CSV_PATH}" != /* ]]; then
  CSV_PATH="${PWD}/${CSV_PATH}"
fi

BACKEND=${IDXD_RUST_BENCH_BACKEND:-hardware}
BYTE_LIST=${IDXD_RUST_BENCH_BYTES:-64,4096}
CONCURRENCY_LIST=${IDXD_RUST_BENCH_CONCURRENCY:-1,4,16}
DURATION_MS=${IDXD_RUST_BENCH_DURATION_MS:-100}
MAX_PAGE_FAULT_RETRIES=${IDXD_RUST_BENCH_MAX_PAGE_FAULT_RETRIES:-1}
ITERATIONS=${IDXD_RUST_BENCH_ITERATIONS:-1}
BUILD_PROFILE=${IDXD_RUST_BENCH_PROFILE:-release}
SKIP_BUILD=${IDXD_RUST_BENCH_SKIP_BUILD:-0}
PREFLIGHT_TIMEOUT=${IDXD_RUST_BENCH_PREFLIGHT_TIMEOUT:-20s}
RUN_TIMEOUT=${IDXD_RUST_BENCH_RUN_TIMEOUT:-60s}
LAUNCHER_PATH=${IDXD_RUST_BENCH_LAUNCHER_PATH:-${REPO_ROOT}/tools/build/dsa_launcher}

if [[ "${BUILD_PROFILE}" == "dev" ]]; then
  TARGET_SUBDIR=debug
else
  TARGET_SUBDIR=${BUILD_PROFILE}
fi
BINARY_PATH=${IDXD_RUST_BENCH_BINARY:-${REPO_ROOT}/target/${TARGET_SUBDIR}/tokio_memmove_bench}

log_phase() {
  local phase=$1
  shift
  printf '[bench_async_throughput_matrix] phase=%s output_dir=%s csv=%s %s\n' "${phase}" "${OUTPUT_DIR}" "${CSV_PATH}" "$*"
}

fail_phase() {
  local phase=$1
  shift
  printf '[bench_async_throughput_matrix] phase=%s output_dir=%s csv=%s %s\n' "${phase}" "${OUTPUT_DIR}" "${CSV_PATH}" "$*" >&2
  exit 1
}

parse_positive_list() {
  local raw=$1
  local description=$2
  local -n out=$3

  out=()
  IFS=',' read -r -a parts <<< "${raw}"
  if [[ ${#parts[@]} -eq 0 ]]; then
    fail_phase preflight "message=${description} list must not be empty"
  fi

  local part trimmed
  for part in "${parts[@]}"; do
    trimmed=${part//[[:space:]]/}
    if [[ -z "${trimmed}" || ! "${trimmed}" =~ ^[1-9][0-9]*$ ]]; then
      fail_phase preflight "message=${description} entries must be positive integers value=${part}"
    fi
    out+=("${trimmed}")
  done
}

parse_nonnegative_scalar() {
  local raw=$1
  local description=$2
  local trimmed=${raw//[[:space:]]/}
  if [[ -z "${trimmed}" || ! "${trimmed}" =~ ^[0-9]+$ ]]; then
    fail_phase preflight "message=${description} must be a non-negative integer value=${raw}"
  fi
  printf '%s\n' "${trimmed}"
}

find_default_device() {
  if [[ -n "${IDXD_RUST_BENCH_DEVICE:-}" ]]; then
    printf '%s\n' "${IDXD_RUST_BENCH_DEVICE}"
    return 0
  fi
  if [[ -e /dev/dsa/wq0.1 ]]; then
    printf '/dev/dsa/wq0.1\n'
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

init_csv() {
  python3 - <<'PY' "${CSV_PATH}"
import csv
import sys
from pathlib import Path

path = Path(sys.argv[1])
path.parent.mkdir(parents=True, exist_ok=True)
with path.open('w', newline='', encoding='utf-8') as f:
    writer = csv.writer(f)
    writer.writerow([
        'backend',
        'device_path',
        'bytes',
        'concurrency',
        'duration_ms',
        'max_page_fault_retries',
        'completed_operations',
        'failed_operations',
        'elapsed_ns',
        'ops_per_sec',
        'bytes_per_sec',
        'gib_per_sec',
        'verdict',
        'claim_eligible',
        'artifact_path',
        'stdout_path',
        'stderr_path',
        'verifier_log_path',
    ])
PY
}

append_csv_row() {
  local artifact=$1
  local stdout_path=$2
  local stderr_path=$3
  local verifier_log_path=$4

  python3 - <<'PY' "${CSV_PATH}" "${artifact}" "${stdout_path}" "${stderr_path}" "${verifier_log_path}"
import csv
import json
import sys
from pathlib import Path

csv_path = Path(sys.argv[1])
artifact_path = Path(sys.argv[2])
stdout_path = Path(sys.argv[3])
stderr_path = Path(sys.argv[4])
verifier_log_path = Path(sys.argv[5])

if not artifact_path.is_file():
    raise SystemExit(f'missing artifact: {artifact_path}')
report = json.loads(artifact_path.read_text(encoding='utf-8'))
rows = [row for row in report.get('results', []) if row.get('mode') == 'fixed_duration_throughput']
if len(rows) != 1:
    raise SystemExit(f'expected exactly one fixed_duration_throughput row, found {len(rows)}')
row = rows[0]
bytes_per_sec = row.get('bytes_per_sec')
gib_per_sec = None if bytes_per_sec is None else float(bytes_per_sec) / (1024.0 ** 3)

with csv_path.open('a', newline='', encoding='utf-8') as f:
    writer = csv.writer(f)
    writer.writerow([
        report['backend'],
        report['device_path'],
        report['requested_bytes'],
        report['concurrency'],
        report['duration_ms'],
        report['max_page_fault_retries'],
        row['completed_operations'],
        row['failed_operations'],
        row['elapsed_ns'],
        row['ops_per_sec'],
        row['bytes_per_sec'],
        gib_per_sec,
        row['verdict'],
        str(row['claim_eligible']).lower(),
        str(artifact_path),
        str(stdout_path),
        str(stderr_path),
        str(verifier_log_path),
    ])

print(f"ok={str(report['ok']).lower()}")
print(f"claim_eligible={str(report['claim_eligible']).lower()}")
print(f"verdict={report['verdict']}")
print(f"completed_operations={row['completed_operations']}")
print(f"failed_operations={row['failed_operations']}")
print(f"ops_per_sec={row['ops_per_sec']}")
print(f"bytes_per_sec={row['bytes_per_sec']}")
print(f"gib_per_sec={gib_per_sec}")
PY
}

case "${BACKEND}" in
  hardware|software) ;;
  *) fail_phase preflight "message=IDXD_RUST_BENCH_BACKEND must be hardware or software backend=${BACKEND}" ;;
esac

parse_positive_list "${BYTE_LIST}" "IDXD_RUST_BENCH_BYTES" BYTE_VALUES
parse_positive_list "${CONCURRENCY_LIST}" "IDXD_RUST_BENCH_CONCURRENCY" CONCURRENCY_VALUES
parse_positive_list "${DURATION_MS}" "IDXD_RUST_BENCH_DURATION_MS" DURATION_VALUES
parse_positive_list "${ITERATIONS}" "IDXD_RUST_BENCH_ITERATIONS" ITERATION_VALUES
MAX_PAGE_FAULT_RETRIES=$(parse_nonnegative_scalar "${MAX_PAGE_FAULT_RETRIES}" "IDXD_RUST_BENCH_MAX_PAGE_FAULT_RETRIES")
if [[ ${#DURATION_VALUES[@]} -ne 1 ]]; then
  fail_phase preflight "message=IDXD_RUST_BENCH_DURATION_MS expects one positive integer"
fi
if [[ ${#ITERATION_VALUES[@]} -ne 1 ]]; then
  fail_phase preflight "message=IDXD_RUST_BENCH_ITERATIONS expects one positive integer"
fi
DURATION_MS=${DURATION_VALUES[0]}
ITERATIONS=${ITERATION_VALUES[0]}

mkdir -p "${OUTPUT_DIR}" 2>/dev/null || fail_phase preflight "message=failed to create output directory"
touch "${OUTPUT_DIR}/.write-test" 2>/dev/null || fail_phase preflight "message=failed to write output directory"
rm -f "${OUTPUT_DIR}/.write-test"
command -v python3 >/dev/null 2>&1 || fail_phase preflight "message=python3 command not found"

DEVICE_PATH=${IDXD_RUST_BENCH_DEVICE:-}
if [[ "${BACKEND}" == "hardware" ]]; then
  DEVICE_PATH=$(find_default_device) || fail_phase preflight "backend=hardware device_path=<none> message=no /dev/dsa/wq* device found; set IDXD_RUST_BENCH_DEVICE explicitly"
fi
if [[ "${BACKEND}" == "software" && -z "${DEVICE_PATH}" ]]; then
  DEVICE_PATH=/dev/dsa/wq0.0
fi

if [[ -n "${IDXD_RUST_BENCH_BINARY:-}" && "${SKIP_BUILD}" != "1" ]]; then
  fail_phase preflight "message=IDXD_RUST_BENCH_BINARY requires IDXD_RUST_BENCH_SKIP_BUILD=1 so the script does not build one binary and execute another"
fi

if [[ "${SKIP_BUILD}" != "1" ]]; then
  log_phase build "workspace=${REPO_ROOT} binary=${BINARY_PATH} profile=${BUILD_PROFILE}"
  (
    cd "${REPO_ROOT}"
    cargo build --profile "${BUILD_PROFILE}" -p idxd-rust --bin tokio_memmove_bench
  )
fi
[[ -x "${BINARY_PATH}" ]] || fail_phase preflight "message=tokio_memmove_bench binary is not executable binary=${BINARY_PATH}"

init_csv

matrix_failed=0
points=0
for bytes in "${BYTE_VALUES[@]}"; do
  for concurrency in "${CONCURRENCY_VALUES[@]}"; do
    points=$((points + 1))
    point_dir="${OUTPUT_DIR}/bytes-${bytes}/concurrency-${concurrency}"
    mkdir -p "${point_dir}" 2>/dev/null || fail_phase preflight "message=failed to create point output directory point_dir=${point_dir}"
    verifier_log="${point_dir}/verify_tokio_memmove_bench.log"
    verifier_stderr="${point_dir}/verify_tokio_memmove_bench.stderr"
    artifact="${point_dir}/tokio_memmove_bench.json"
    stdout_path="${point_dir}/tokio_memmove_bench.stdout"
    stderr_path="${point_dir}/tokio_memmove_bench.stderr"

    log_phase runtime "backend=${BACKEND} device_path=${DEVICE_PATH} bytes=${bytes} concurrency=${concurrency} duration_ms=${DURATION_MS} max_page_fault_retries=${MAX_PAGE_FAULT_RETRIES} artifact=${artifact}"

    set +e
    IDXD_RUST_VERIFY_BACKEND="${BACKEND}" \
    IDXD_RUST_VERIFY_SUITE=throughput \
    IDXD_RUST_VERIFY_DEVICE="${DEVICE_PATH}" \
    IDXD_RUST_VERIFY_BYTES="${bytes}" \
    IDXD_RUST_VERIFY_ITERATIONS="${ITERATIONS}" \
    IDXD_RUST_VERIFY_CONCURRENCY="${concurrency}" \
    IDXD_RUST_VERIFY_DURATION_MS="${DURATION_MS}" \
    IDXD_RUST_VERIFY_MAX_PAGE_FAULT_RETRIES="${MAX_PAGE_FAULT_RETRIES}" \
    IDXD_RUST_VERIFY_OUTPUT_DIR="${point_dir}" \
    IDXD_RUST_VERIFY_PROFILE="${BUILD_PROFILE}" \
    IDXD_RUST_VERIFY_SKIP_BUILD=1 \
    IDXD_RUST_VERIFY_BINARY="${BINARY_PATH}" \
    IDXD_RUST_VERIFY_LAUNCHER_PATH="${LAUNCHER_PATH}" \
    IDXD_RUST_VERIFY_PREFLIGHT_TIMEOUT="${PREFLIGHT_TIMEOUT}" \
    IDXD_RUST_VERIFY_RUN_TIMEOUT="${RUN_TIMEOUT}" \
      bash "${SCRIPT_DIR}/verify_tokio_memmove_bench.sh" >"${verifier_log}" 2>"${verifier_stderr}"
    verifier_exit=$?
    set -e

    if [[ "${verifier_exit}" -ne 0 ]]; then
      fail_phase runtime "backend=${BACKEND} device_path=${DEVICE_PATH} bytes=${bytes} concurrency=${concurrency} verifier_exit=${verifier_exit} verifier_log=${verifier_log} verifier_stderr=${verifier_stderr} message=throughput point verifier failed"
    fi

    row_summary=$(append_csv_row "${artifact}" "${stdout_path}" "${stderr_path}" "${verifier_log}") \
      || fail_phase artifact_validation "backend=${BACKEND} device_path=${DEVICE_PATH} bytes=${bytes} concurrency=${concurrency} artifact=${artifact} verifier_log=${verifier_log} message=failed to append CSV row"

    log_phase point_done "backend=${BACKEND} device_path=${DEVICE_PATH} bytes=${bytes} concurrency=${concurrency} max_page_fault_retries=${MAX_PAGE_FAULT_RETRIES} ${row_summary//$'\n'/ } artifact=${artifact}"

    if [[ "${row_summary}" != *"ok=true"* ]]; then
      matrix_failed=1
    fi
    if [[ "${BACKEND}" == "hardware" && "${row_summary}" != *"claim_eligible=true"* ]]; then
      matrix_failed=1
    fi
    if [[ "${row_summary}" != *"verdict=pass"* ]]; then
      matrix_failed=1
    fi
  done
done

if [[ "${matrix_failed}" -ne 0 ]]; then
  fail_phase done "verdict=fail backend=${BACKEND} device_path=${DEVICE_PATH} points=${points} message=one or more throughput points were not claim-eligible passes"
fi

log_phase done "verdict=pass backend=${BACKEND} device_path=${DEVICE_PATH} points=${points} bytes=${BYTE_LIST} concurrency=${CONCURRENCY_LIST} duration_ms=${DURATION_MS} max_page_fault_retries=${MAX_PAGE_FAULT_RETRIES} csv=${CSV_PATH}"
