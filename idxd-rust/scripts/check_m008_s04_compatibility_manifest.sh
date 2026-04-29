#!/usr/bin/env bash
set -euo pipefail

MANIFEST_PATH=${1:-target/m008-s04-compatibility/latest/manifest.json}

python3 - "$MANIFEST_PATH" <<'PY'
import json
import re
import sys
from pathlib import Path

manifest_path = Path(sys.argv[1])
repo_root = Path.cwd()
FORBIDDEN_PAYLOAD_LABELS = {
    "payload",
    "payload_bytes",
    "payload_dump",
    "raw_payload",
    "dumped_payload",
    "source_bytes",
    "source_payload",
    "src_payload",
    "destination_bytes",
    "destination_payload",
    "dst_payload",
}
REQUIRED_RECORDS = {
    "idxd_cli_contracts",
    "idxd_verifier_contracts",
    "verify_live_memmove_hardware",
    "verify_async_memmove_hardware",
    "verify_tokio_memmove_bench_software",
    "verify_tokio_memmove_bench_hardware",
    "tonic_profile_check",
    "tonic_profile_contracts",
}
REQUIRED_VERIFIER_FIELDS = {
    "phase",
    "output_dir",
    "artifact",
    "verdict",
}
ASYNC_DIAGNOSTIC_KEYS = {
    "async_lifecycle_failure_kind",
    "async_worker_failure_kind",
    "async_direct_failure_kind",
    "direct_failure_kind",
}


def fail(reason: str):
    print(f"[check_m008_s04_compatibility_manifest] verdict=fail manifest={manifest_path} reason={reason}", file=sys.stderr)
    raise SystemExit(1)


def resolve_record_path(value, field: str, record_name: str):
    if not value:
        fail(f"record {record_name} missing {field}")
    path = Path(value)
    if not path.is_absolute():
        path = repo_root / path
    if not path.exists():
        fail(f"record {record_name} {field} does not exist: {value}")
    return path


def reject_payload_labels_in_text(text: str, where: str):
    for label in FORBIDDEN_PAYLOAD_LABELS:
        if re.search(rf"(?<![A-Za-z0-9_]){re.escape(label)}(?![A-Za-z0-9_])", text, re.IGNORECASE):
            fail(f"forbidden payload dump label {label!r} in {where}")


def reject_payload_labels_in_json(value, where: str):
    if isinstance(value, dict):
        for key, child in value.items():
            if key in FORBIDDEN_PAYLOAD_LABELS:
                fail(f"forbidden payload dump label {key!r} at {where}")
            reject_payload_labels_in_json(child, f"{where}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            reject_payload_labels_in_json(child, f"{where}[{index}]")
    elif isinstance(value, str):
        reject_payload_labels_in_text(value, where)


def read_json(path: Path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        fail(f"invalid JSON: {exc}")
    except OSError as exc:
        fail(f"cannot read manifest: {exc}")


if not manifest_path.is_file():
    fail("manifest file is missing")
if manifest_path.stat().st_size == 0:
    fail("manifest file is empty")

manifest = read_json(manifest_path)
reject_payload_labels_in_json(manifest, "manifest")

if manifest.get("schema_version") != 1:
    fail("schema_version must be 1")
if manifest.get("milestone") != "M008" or manifest.get("slice") != "S04":
    fail("manifest must identify milestone M008 slice S04")
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

hard_failures = []
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
    for field in ("stdout_log", "stderr_log"):
        path = resolve_record_path(record.get(field), field, name)
        reject_payload_labels_in_text(path.read_text(encoding="utf-8", errors="replace"), f"{name}.{field}")
    if record.get("payload_label_hits") not in ([], None):
        fail(f"record {name} reports payload label hits")

    interpretation = record.get("interpretation")
    if interpretation not in {"pass", "expected_failure", "hard_failure", "claim_eligible_hardware_pass", "non_claim_eligible_pass"}:
        fail(f"record {name} has invalid interpretation {interpretation!r}")
    if interpretation == "hard_failure":
        hard_failures.append(name)

    if record.get("type") == "cargo":
        if record["exit_code"] != 0 or interpretation != "pass":
            fail(f"cargo record {name} must pass")
        continue

    if record.get("type") != "verifier":
        fail(f"record {name} has invalid type {record.get('type')!r}")
    final_line = record.get("final_line")
    if not isinstance(final_line, str) or not final_line.startswith("[verify_"):
        fail(f"verifier record {name} missing final verifier line")
    fields = record.get("final_line_fields")
    if not isinstance(fields, dict):
        fail(f"verifier record {name} missing final_line_fields")
    missing_fields = sorted(REQUIRED_VERIFIER_FIELDS - fields.keys())
    if missing_fields:
        fail(f"verifier record {name} missing final fields: {', '.join(missing_fields)}")
    resolve_record_path(record.get("verifier_output_dir"), "verifier_output_dir", name)
    artifact_value = record.get("artifact")
    if fields.get("verdict") == "pass":
        resolve_record_path(artifact_value, "artifact", name)

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
    else:
        fail(f"verifier record {name} is a hard failure")

    if (
        name in {"verify_async_memmove_hardware", "verify_tokio_memmove_bench_software", "verify_tokio_memmove_bench_hardware"}
        and interpretation != "expected_failure"
    ):
        if not (ASYNC_DIAGNOSTIC_KEYS & fields.keys()):
            fail(f"async/benchmark verifier record {name} missing async failure-kind diagnostic fields")

if hard_failures:
    fail(f"hard failures present: {', '.join(sorted(hard_failures))}")

summary = manifest.get("summary")
if not isinstance(summary, dict):
    fail("summary must be an object")
if summary.get("total") != len(REQUIRED_RECORDS):
    fail("summary total does not match required record count")
computed = {
    "pass": sum(1 for r in records if r.get("interpretation") == "pass"),
    "expected_failure": sum(1 for r in records if r.get("interpretation") == "expected_failure"),
    "hard_failure": sum(1 for r in records if r.get("interpretation") == "hard_failure"),
    "claim_eligible_hardware_pass": sum(1 for r in records if r.get("interpretation") == "claim_eligible_hardware_pass"),
    "non_claim_eligible_pass": sum(1 for r in records if r.get("interpretation") == "non_claim_eligible_pass"),
}
for key, value in computed.items():
    if summary.get(key) != value:
        fail(f"summary {key}={summary.get(key)!r} expected {value}")
if computed["non_claim_eligible_pass"] != 1:
    fail("exactly one software non-claim-eligible pass is required")
if computed["hard_failure"] != 0:
    fail("hard_failure count must be zero")

print(f"[check_m008_s04_compatibility_manifest] verdict=pass manifest={manifest_path} records={len(records)} expected_failures={computed['expected_failure']}")
PY
