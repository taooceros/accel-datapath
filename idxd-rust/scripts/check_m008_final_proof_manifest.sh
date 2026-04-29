#!/usr/bin/env bash
set -euo pipefail

MANIFEST_PATH=${1:-target/m008-final-proof/latest/manifest.json}

python3 - "$MANIFEST_PATH" <<'PY'
import json
import sys
from pathlib import Path

manifest_path = Path(sys.argv[1])
if not manifest_path.is_absolute():
    manifest_path = Path.cwd() / manifest_path
manifest_path = manifest_path.resolve()
repo_root = None
for parent in [manifest_path.parent, *manifest_path.parents]:
    if (parent / "Cargo.toml").is_file() and (parent / "idxd-rust").is_dir():
        repo_root = parent
        break
if repo_root is None:
    repo_root = Path.cwd().resolve()
FORBIDDEN_TEXT = [
    "payload_bytes",
    "source_bytes",
    "destination_bytes",
    "payload_dump",
    "0xAB",
    "171, 171",
]
REQUIRED_RECORDS = {
    "final_report_guard",
    "idxd_rust_contracts",
    "idxd_rust_bins",
    "s04_compatibility_matrix",
    "hw_eval_contracts",
    "idxd_sys_raw_boundary",
    "hw_eval_software_json",
    "verify_live_memmove_hardware",
    "verify_async_memmove_hardware",
    "verify_tokio_memmove_bench_software",
    "verify_tokio_memmove_bench_hardware",
    "tonic_profile_check",
    "tonic_profile_contracts",
}
VERIFIER_RECORDS = {
    "verify_live_memmove_hardware",
    "verify_async_memmove_hardware",
    "verify_tokio_memmove_bench_software",
    "verify_tokio_memmove_bench_hardware",
}


def fail(reason: str):
    print(f"[check_m008_final_proof_manifest] verdict=fail manifest={manifest_path} reason={reason}", file=sys.stderr)
    raise SystemExit(1)


def resolve_path(value, field: str, record_name: str):
    if not value:
        fail(f"record {record_name} missing {field}")
    path = Path(value)
    if not path.is_absolute():
        path = repo_root / path
    if not path.exists():
        fail(f"record {record_name} {field} does not exist: {value}")
    return path


def reject_forbidden_text(text: str, where: str):
    for item in FORBIDDEN_TEXT:
        if item in text:
            fail(f"forbidden diagnostic text {item!r} in {where}")


def reject_forbidden_json(value, where: str):
    if isinstance(value, dict):
        for key, child in value.items():
            reject_forbidden_text(str(key), f"{where}.{key}")
            reject_forbidden_json(child, f"{where}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            reject_forbidden_json(child, f"{where}[{index}]")
    elif isinstance(value, str):
        reject_forbidden_text(value, where)


if not manifest_path.is_file():
    fail("manifest file is missing")
if manifest_path.stat().st_size == 0:
    fail("manifest file is empty")
try:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
except json.JSONDecodeError as exc:
    fail(f"invalid JSON: {exc}")
reject_forbidden_json(manifest, "manifest")

if manifest.get("schema_version") != 1:
    fail("schema_version must be 1")
if manifest.get("milestone") != "M008" or manifest.get("slice") != "S06":
    fail("manifest must identify milestone M008 slice S06")
records = manifest.get("records")
if not isinstance(records, list):
    fail("records must be an array")
if len(records) != len(REQUIRED_RECORDS):
    fail(f"records must contain exactly {len(REQUIRED_RECORDS)} entries")

by_name = {}
for record in records:
    if not isinstance(record, dict):
        fail("each record must be an object")
    name = record.get("name")
    if not isinstance(name, str) or not name:
        fail("record missing non-empty name")
    if name in by_name:
        fail(f"duplicate record {name}")
    by_name[name] = record

missing = sorted(REQUIRED_RECORDS - by_name.keys())
if missing:
    fail(f"missing required records: {', '.join(missing)}")
extra = sorted(by_name.keys() - REQUIRED_RECORDS)
if extra:
    fail(f"unexpected records: {', '.join(extra)}")

for name, record in by_name.items():
    command = record.get("command")
    if not isinstance(command, str) or not command:
        fail(f"record {name} missing command text")
    if record.get("type") == "cargo" and "--manifest-path ./Cargo.toml" not in command:
        fail(f"cargo record {name} must use --manifest-path ./Cargo.toml")
    if not isinstance(record.get("exit_code"), int):
        fail(f"record {name} missing integer exit_code")
    if not isinstance(record.get("duration_ms"), int) or record["duration_ms"] < 0:
        fail(f"record {name} missing non-negative duration_ms")
    if record.get("forbidden_text_hits") not in ([], None):
        fail(f"record {name} reports forbidden diagnostic text")
    for field in ("stdout_log", "stderr_log"):
        path = resolve_path(record.get(field), field, name)
        reject_forbidden_text(path.read_text(encoding="utf-8", errors="replace"), f"{name}.{field}")

    interpretation = record.get("interpretation")
    if interpretation not in {"pass", "expected_failure", "hard_failure", "claim_eligible_hardware_pass", "non_claim_eligible_pass"}:
        fail(f"record {name} has invalid interpretation {interpretation!r}")
    if interpretation == "hard_failure":
        fail(f"record {name} is a hard failure")

    if name == "s04_compatibility_matrix":
        artifact = resolve_path(record.get("artifact"), "artifact", name)
        try:
            s04_manifest = json.loads(artifact.read_text(encoding="utf-8"))
        except json.JSONDecodeError as exc:
            fail(f"record {name} artifact is invalid JSON: {exc}")
        if s04_manifest.get("milestone") != "M008" or s04_manifest.get("slice") != "S04":
            fail("S04 compatibility artifact must identify milestone M008 slice S04")
        s04_records = s04_manifest.get("records")
        if not isinstance(s04_records, list) or not s04_records:
            fail("S04 compatibility artifact missing records")
        for s04_record in s04_records:
            if s04_record.get("type") == "verifier":
                fields = s04_record.get("final_line_fields")
                if not isinstance(fields, dict) or "phase" not in fields or "verdict" not in fields:
                    fail(f"S04 verifier record {s04_record.get('name')} missing final-line diagnostics")
        reject_forbidden_json(s04_manifest, "s04_manifest")

    if name not in VERIFIER_RECORDS:
        if record.get("exit_code") != 0 or interpretation != "pass":
            fail(f"non-verifier record {name} must pass")
        continue

    if record.get("type") != "verifier":
        fail(f"verifier record {name} has invalid type")
    final_line = record.get("final_line")
    if not isinstance(final_line, str) or not final_line.startswith("[verify_"):
        fail(f"verifier record {name} missing final verifier line")
    fields = record.get("final_line_fields")
    if not isinstance(fields, dict):
        fail(f"verifier record {name} missing final_line_fields")
    for required in ("phase", "output_dir", "verdict"):
        if required not in fields:
            fail(f"verifier record {name} missing final field {required}")
    resolve_path(record.get("verifier_output_dir"), "verifier_output_dir", name)

    if interpretation == "expected_failure":
        if fields.get("verdict") != "expected_failure":
            fail(f"expected-failure record {name} must carry verdict=expected_failure")
        if not fields.get("failure_phase"):
            fail(f"expected-failure record {name} missing failure_phase")
        if not fields.get("launcher_status"):
            fail(f"expected-failure record {name} missing launcher_status")
        if record.get("claim_eligible") not in (None, "false"):
            fail(f"expected-failure record {name} must not be claim eligible")
    elif interpretation in {"pass", "claim_eligible_hardware_pass", "non_claim_eligible_pass"}:
        if fields.get("verdict") != "pass":
            fail(f"passing verifier record {name} must carry verdict=pass")
        if name == "verify_tokio_memmove_bench_software":
            if fields.get("backend") != "software" or fields.get("claim_eligible") != "false":
                fail("software benchmark pass requires backend=software and claim_eligible=false")
            if interpretation != "non_claim_eligible_pass":
                fail("software benchmark pass must be interpreted as non_claim_eligible_pass")
        if name == "verify_tokio_memmove_bench_hardware":
            if fields.get("backend") != "hardware" or fields.get("claim_eligible") != "true":
                fail("hardware benchmark pass requires backend=hardware and claim_eligible=true")
            if interpretation != "claim_eligible_hardware_pass":
                fail("hardware benchmark pass must be interpreted as claim_eligible_hardware_pass")
        artifact = record.get("artifact")
        if artifact:
            resolve_path(artifact, "artifact", name)
    else:
        fail(f"verifier record {name} has unsupported interpretation")

summary = manifest.get("summary")
if not isinstance(summary, dict):
    fail("summary must be an object")
computed = {
    "pass": sum(1 for r in records if r.get("interpretation") == "pass"),
    "expected_failure": sum(1 for r in records if r.get("interpretation") == "expected_failure"),
    "hard_failure": sum(1 for r in records if r.get("interpretation") == "hard_failure"),
    "claim_eligible_hardware_pass": sum(1 for r in records if r.get("interpretation") == "claim_eligible_hardware_pass"),
    "non_claim_eligible_pass": sum(1 for r in records if r.get("interpretation") == "non_claim_eligible_pass"),
}
if summary.get("total") != len(REQUIRED_RECORDS):
    fail("summary total does not match required record count")
for key, value in computed.items():
    if summary.get(key) != value:
        fail(f"summary {key}={summary.get(key)!r} expected {value}")
if computed["hard_failure"] != 0:
    fail("hard_failure count must be zero")
if computed["non_claim_eligible_pass"] != 1:
    fail("exactly one software non-claim-eligible pass is required")

print(f"[check_m008_final_proof_manifest] verdict=pass manifest={manifest_path} records={len(records)} expected_failures={computed['expected_failure']}")
PY
