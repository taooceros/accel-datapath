#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
CRATE_DIR=$(cd -- "${SCRIPT_DIR}/.." && pwd)
REPO_ROOT=$(cd -- "${CRATE_DIR}/.." && pwd)
TIMESTAMP=$(date -u +%Y%m%dT%H%M%SZ)
OUTPUT_DIR=${M007_S04_EVIDENCE_OUTPUT_DIR:-${REPO_ROOT}/target/m007-s04-evidence/${TIMESTAMP}}
MANIFEST_PATH="${OUTPUT_DIR}/manifest.json"
COMMANDS_DIR="${OUTPUT_DIR}/logs"
VERIFIER_ROOT="${OUTPUT_DIR}/verifiers"
SOFTWARE_OUTPUT_DIR="${VERIFIER_ROOT}/software"
HARDWARE_OUTPUT_DIR="${VERIFIER_ROOT}/hardware"

mkdir -p "${COMMANDS_DIR}" "${SOFTWARE_OUTPUT_DIR}" "${HARDWARE_OUTPUT_DIR}"

COMMAND_RECORDS_FILE=$(mktemp "${OUTPUT_DIR}/manifest-commands.XXXXXX")
trap 'rm -f "${COMMAND_RECORDS_FILE}"' EXIT

json_quote() {
  python3 -c 'import json, sys; print(json.dumps(sys.argv[1]))' "$1"
}

append_command_record() {
  local name=$1
  local command_text=$2
  local exit_code=$3
  local log_path=$4
  local verifier_output_dir=${5:-}

  python3 - "$COMMAND_RECORDS_FILE" "$name" "$command_text" "$exit_code" "$log_path" "$verifier_output_dir" <<'PY'
import json
import sys
from pathlib import Path

records_path = Path(sys.argv[1])
record = {
    "name": sys.argv[2],
    "command": sys.argv[3],
    "exit_code": int(sys.argv[4]),
    "log_path": sys.argv[5],
}
if sys.argv[6]:
    record["verifier_output_dir"] = sys.argv[6]

with records_path.open("a", encoding="utf-8") as handle:
    handle.write(json.dumps(record, separators=(",", ":")) + "\n")
PY
}

run_logged() {
  local name=$1
  local log_file=$2
  shift 2
  local command_text
  printf -v command_text '%q ' "$@"
  command_text=${command_text% }

  printf '[collect_tokio_memmove_evidence] command=%s log=%s\n' "${name}" "${log_file}"
  local exit_code=0
  if (cd "${REPO_ROOT}" && "$@") >"${log_file}" 2>&1; then
    exit_code=0
  else
    exit_code=$?
  fi
  append_command_record "${name}" "${command_text}" "${exit_code}" "${log_file}"
  return "${exit_code}"
}

run_verifier_logged() {
  local name=$1
  local backend=$2
  local output_dir=$3
  local log_file=$4
  shift 4
  local command_text
  printf -v command_text 'IDXD_RUST_VERIFY_OUTPUT_DIR=%q IDXD_RUST_VERIFY_BACKEND=%q ' "${output_dir}" "${backend}"
  local arg
  for arg in "$@"; do
    printf -v command_text '%s%q ' "${command_text}" "${arg}"
  done
  command_text=${command_text% }

  printf '[collect_tokio_memmove_evidence] command=%s backend=%s log=%s verifier_output_dir=%s\n' "${name}" "${backend}" "${log_file}" "${output_dir}"
  local exit_code=0
  if IDXD_RUST_VERIFY_OUTPUT_DIR="${output_dir}" IDXD_RUST_VERIFY_BACKEND="${backend}" "$@" >"${log_file}" 2>&1; then
    exit_code=0
  else
    exit_code=$?
  fi
  append_command_record "${name}" "${command_text}" "${exit_code}" "${log_file}" "${output_dir}"
  return "${exit_code}"
}

FAILURE=0

run_logged "cargo_test_cli_contract" "${COMMANDS_DIR}/cargo_test_cli_contract.log" \
  cargo test -p idxd-rust --test async_benchmark_cli_contract || FAILURE=1

run_logged "cargo_test_verifier_contract" "${COMMANDS_DIR}/cargo_test_verifier_contract.log" \
  cargo test -p idxd-rust --test async_benchmark_verifier_contract || FAILURE=1

run_logged "cargo_check_bins" "${COMMANDS_DIR}/cargo_check_bins.log" \
  cargo check -p idxd-rust --bins || FAILURE=1

run_verifier_logged "software_verifier" "software" "${SOFTWARE_OUTPUT_DIR}" "${COMMANDS_DIR}/software_verifier.log" \
  bash "${SCRIPT_DIR}/verify_tokio_memmove_bench.sh" || FAILURE=1

HARDWARE_FAILURE=0
run_verifier_logged "hardware_verifier" "hardware" "${HARDWARE_OUTPUT_DIR}" "${COMMANDS_DIR}/hardware_verifier.log" \
  bash "${SCRIPT_DIR}/verify_tokio_memmove_bench.sh" || HARDWARE_FAILURE=$?
if [[ "${HARDWARE_FAILURE}" -ne 0 ]]; then
  FAILURE=1
fi

python3 - "${MANIFEST_PATH}" "${COMMAND_RECORDS_FILE}" "${OUTPUT_DIR}" "${SOFTWARE_OUTPUT_DIR}" "${HARDWARE_OUTPUT_DIR}" <<'PY'
import json
import re
import sys
from pathlib import Path

manifest_path = Path(sys.argv[1])
records_path = Path(sys.argv[2])
output_dir = Path(sys.argv[3])
software_output_dir = Path(sys.argv[4])
hardware_output_dir = Path(sys.argv[5])

DONE_RE = re.compile(r"\[verify_tokio_memmove_bench\] phase=done\b.*")
KEY_RE = re.compile(r"(?:^|\s)([A-Za-z_][A-Za-z0-9_]*)=([^\s]+)")


def read_records():
    if not records_path.exists():
        return []
    return [json.loads(line) for line in records_path.read_text(encoding="utf-8").splitlines() if line.strip()]


def rel(path):
    try:
        return str(Path(path).resolve().relative_to(output_dir.resolve()))
    except ValueError:
        return str(path)


def key_values(line):
    return {key: value for key, value in KEY_RE.findall(line)}


def verifier_record(command, verifier_output_dir):
    log_path = Path(command["log_path"])
    log_text = log_path.read_text(encoding="utf-8", errors="replace") if log_path.exists() else ""
    final_lines = DONE_RE.findall(log_text)
    final_line = final_lines[-1] if final_lines else None
    fields = key_values(final_line) if final_line else {}
    artifact_path = verifier_output_dir / "tokio_memmove_bench.json"
    stdout_path = verifier_output_dir / "tokio_memmove_bench.stdout"
    stderr_path = verifier_output_dir / "tokio_memmove_bench.stderr"
    preflight_stdout_path = verifier_output_dir / "preflight.stdout"
    preflight_stderr_path = verifier_output_dir / "preflight.stderr"

    verdict = fields.get("verdict")
    claim_eligible = fields.get("claim_eligible")
    failure_phase = fields.get("failure_phase")
    launcher_status = fields.get("launcher_status")
    backend = fields.get("backend")
    exit_code = command["exit_code"]

    if exit_code == 0 and verdict == "pass":
        interpretation = "claim_eligible_pass" if claim_eligible == "true" else "non_claim_eligible_pass"
    elif exit_code == 0 and verdict == "expected_failure" and failure_phase:
        interpretation = "classified_expected_failure"
    elif exit_code != 0:
        interpretation = "hard_failure"
    else:
        interpretation = "malformed_final_line"

    claim_eligible_value = True if claim_eligible == "true" else False if claim_eligible == "false" else None
    if interpretation == "classified_expected_failure" and claim_eligible_value is None:
        claim_eligible_value = False

    out = {
        "backend": backend,
        "output_dir": str(verifier_output_dir),
        "final_line": final_line,
        "verdict": verdict,
        "claim_eligible": claim_eligible_value,
        "failure_phase": failure_phase,
        "launcher_status": launcher_status,
        "interpretation": interpretation,
        "artifact_path": str(artifact_path) if artifact_path.exists() else None,
        "stdout_path": str(stdout_path) if stdout_path.exists() else None,
        "stderr_path": str(stderr_path) if stderr_path.exists() else None,
        "preflight_stdout_path": str(preflight_stdout_path) if preflight_stdout_path.exists() else None,
        "preflight_stderr_path": str(preflight_stderr_path) if preflight_stderr_path.exists() else None,
    }
    return out


commands = read_records()
for command in commands:
    command["log_path"] = str(Path(command["log_path"]))

software_command = next((command for command in commands if command["name"] == "software_verifier"), None)
hardware_command = next((command for command in commands if command["name"] == "hardware_verifier"), None)
software = verifier_record(software_command, software_output_dir) if software_command else None
hardware = verifier_record(hardware_command, hardware_output_dir) if hardware_command else None

collection_success = all(command["exit_code"] == 0 for command in commands)
if hardware and hardware["interpretation"] == "classified_expected_failure":
    collection_success = all(command["exit_code"] == 0 for command in commands if command["name"] != "hardware_verifier")

manifest_errors = []
if software is None:
    manifest_errors.append("missing software verifier record")
elif software["interpretation"] != "non_claim_eligible_pass":
    manifest_errors.append(f"software verifier did not produce non-claim-eligible pass: {software['interpretation']}")

if hardware is None:
    manifest_errors.append("missing hardware verifier record")
elif hardware["interpretation"] not in {"claim_eligible_pass", "classified_expected_failure"}:
    manifest_errors.append(f"hardware verifier did not produce pass or classified expected-failure: {hardware['interpretation']}")

if manifest_errors:
    collection_success = False

manifest = {
    "schema_version": 1,
    "milestone": "M007",
    "slice": "S04",
    "task": "T01",
    "output_dir": str(output_dir),
    "collection_success": collection_success,
    "commands": commands,
    "manifest_errors": manifest_errors,
    "verifiers": {
        "software": software,
        "hardware": hardware,
    },
}
manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY

python3 -m json.tool "${MANIFEST_PATH}" >/dev/null

if [[ "${FAILURE}" -ne 0 ]] || ! python3 - "${MANIFEST_PATH}" <<'PY'
import json
import sys
from pathlib import Path
manifest = json.loads(Path(sys.argv[1]).read_text(encoding='utf-8'))
if manifest.get('manifest_errors'):
    for error in manifest['manifest_errors']:
        print(error, file=sys.stderr)
    raise SystemExit(1)
if not manifest.get('collection_success'):
    raise SystemExit(1)
PY
then
  printf '[collect_tokio_memmove_evidence] status=failed manifest=%s\n' "${MANIFEST_PATH}" >&2
  exit 1
fi

printf '[collect_tokio_memmove_evidence] status=ok manifest=%s\n' "${MANIFEST_PATH}"
