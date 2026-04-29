#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "${SCRIPT_DIR}/../.." && pwd)

python3 - "$REPO_ROOT" "$@" <<'PY'
import json
import os
import re
import shutil
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

repo_root = Path(sys.argv[1]).resolve()
base_dir = repo_root / "target" / "m008-s04-compatibility"
timestamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
run_dir = base_dir / timestamp
logs_dir = run_dir / "logs"
artifacts_dir = run_dir / "artifacts"
logs_dir.mkdir(parents=True, exist_ok=False)
artifacts_dir.mkdir(parents=True, exist_ok=True)

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
KEY_PATTERN = re.compile(r"(?:^|\s)([A-Za-z_][A-Za-z0-9_]*)=")


def rel(path: Path | None):
    if path is None:
        return None
    try:
        return str(path.resolve().relative_to(repo_root))
    except ValueError:
        return str(path)


def parse_fields(line: str):
    matches = list(KEY_PATTERN.finditer(line))
    fields = {}
    for index, match in enumerate(matches):
        key = match.group(1)
        start = match.end()
        end = matches[index + 1].start() if index + 1 < len(matches) else len(line)
        fields[key] = line[start:end].strip()
    return fields


def final_verifier_line(stdout_path: Path, stderr_path: Path):
    candidates = []
    for path in (stdout_path, stderr_path):
        if path.is_file():
            for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
                if line.startswith("[verify_") and " phase=" in line:
                    candidates.append(line)
    return candidates[-1] if candidates else None


def scan_forbidden_payload_labels(path: Path):
    hits = []
    if not path.exists():
        return hits
    paths = [path]
    if path.is_dir():
        paths = [p for p in path.rglob("*") if p.is_file()]
    for file_path in paths:
        try:
            text = file_path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        for label in FORBIDDEN_PAYLOAD_LABELS:
            if re.search(rf"(?<![A-Za-z0-9_]){re.escape(label)}(?![A-Za-z0-9_])", text, re.IGNORECASE):
                hits.append({"path": rel(file_path), "label": label})
    return hits


def interpretation_for(command_type: str, exit_code: int, fields: dict, payload_hits: list):
    if payload_hits:
        return "hard_failure"
    if command_type == "cargo":
        return "pass" if exit_code == 0 else "hard_failure"

    verdict = fields.get("verdict")
    backend = fields.get("backend")
    claim_eligible = fields.get("claim_eligible")
    if exit_code != 0:
        return "hard_failure"
    if verdict == "expected_failure":
        return "expected_failure"
    if verdict == "pass":
        if backend == "software" and claim_eligible == "false":
            return "non_claim_eligible_pass"
        if backend == "hardware" and claim_eligible == "true":
            return "claim_eligible_hardware_pass"
        return "pass"
    return "hard_failure"


def run_record(spec: dict):
    name = spec["name"]
    stdout_path = logs_dir / f"{name}.stdout.log"
    stderr_path = logs_dir / f"{name}.stderr.log"
    env = os.environ.copy()
    env.update(spec.get("env", {}))
    command = spec["command"]
    print(f"[collect_m008_s04_compatibility] running {name}: {command}", flush=True)
    started = time.monotonic()
    with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
        proc = subprocess.run(
            ["bash", "-lc", command],
            cwd=repo_root,
            env=env,
            stdout=stdout,
            stderr=stderr,
            check=False,
        )
    duration_ms = int((time.monotonic() - started) * 1000)

    final_line = None
    fields = {}
    verifier_output_dir = None
    artifact_path = None
    payload_scan_roots = [stdout_path, stderr_path]
    if spec["type"] == "verifier":
        final_line = final_verifier_line(stdout_path, stderr_path)
        if final_line:
            fields = parse_fields(final_line)
        if fields.get("output_dir"):
            verifier_output_dir = Path(fields["output_dir"])
            payload_scan_roots.append(verifier_output_dir)
        if fields.get("artifact"):
            artifact_path = Path(fields["artifact"])
            payload_scan_roots.append(artifact_path)

    payload_hits = []
    for root in payload_scan_roots:
        payload_hits.extend(scan_forbidden_payload_labels(root))

    interpretation = interpretation_for(spec["type"], proc.returncode, fields, payload_hits)
    record = {
        "name": name,
        "type": spec["type"],
        "command": command,
        "exit_code": proc.returncode,
        "duration_ms": duration_ms,
        "stdout_log": rel(stdout_path),
        "stderr_log": rel(stderr_path),
        "interpretation": interpretation,
        "required": True,
        "payload_label_hits": payload_hits,
    }
    if spec["type"] == "verifier":
        record.update({
            "final_line": final_line,
            "final_line_fields": fields,
            "verifier_output_dir": rel(verifier_output_dir),
            "artifact": rel(artifact_path),
            "launcher_status": fields.get("launcher_status"),
            "failure_phase": fields.get("failure_phase"),
            "validation_phase": fields.get("validation_phase"),
            "validation_error_kind": fields.get("validation_error_kind"),
            "async_lifecycle_failure_kind": fields.get("async_lifecycle_failure_kind"),
            "async_worker_failure_kind": fields.get("async_worker_failure_kind"),
            "async_direct_failure_kind": fields.get("async_direct_failure_kind"),
            "direct_failure_kind": fields.get("direct_failure_kind"),
            "claim_eligible": fields.get("claim_eligible"),
        })
    print(
        f"[collect_m008_s04_compatibility] {name} exit={proc.returncode} interpretation={interpretation} stdout={record['stdout_log']} stderr={record['stderr_log']}",
        flush=True,
    )
    return record


commands = [
    {
        "name": "idxd_cli_contracts",
        "type": "cargo",
        "command": "cargo test --manifest-path ./Cargo.toml -p idxd-rust --test validation_cli_contract --test async_validation_cli_contract --test async_benchmark_cli_contract",
    },
    {
        "name": "idxd_verifier_contracts",
        "type": "cargo",
        "command": "cargo test --manifest-path ./Cargo.toml -p idxd-rust --test verifier_contract --test async_verifier_contract --test async_benchmark_verifier_contract",
    },
    {
        "name": "verify_live_memmove_hardware",
        "type": "verifier",
        "command": "bash idxd-rust/scripts/verify_live_memmove.sh",
        "env": {"IDXD_RUST_VERIFY_OUTPUT_DIR": str(artifacts_dir / "verify_live_memmove_hardware")},
    },
    {
        "name": "verify_async_memmove_hardware",
        "type": "verifier",
        "command": "bash idxd-rust/scripts/verify_async_memmove.sh",
        "env": {"IDXD_RUST_VERIFY_OUTPUT_DIR": str(artifacts_dir / "verify_async_memmove_hardware")},
    },
    {
        "name": "verify_tokio_memmove_bench_software",
        "type": "verifier",
        "command": "IDXD_RUST_VERIFY_BACKEND=software bash idxd-rust/scripts/verify_tokio_memmove_bench.sh",
        "env": {"IDXD_RUST_VERIFY_OUTPUT_DIR": str(artifacts_dir / "verify_tokio_memmove_bench_software")},
    },
    {
        "name": "verify_tokio_memmove_bench_hardware",
        "type": "verifier",
        "command": "IDXD_RUST_VERIFY_BACKEND=hardware bash idxd-rust/scripts/verify_tokio_memmove_bench.sh",
        "env": {"IDXD_RUST_VERIFY_OUTPUT_DIR": str(artifacts_dir / "verify_tokio_memmove_bench_hardware")},
    },
    {
        "name": "tonic_profile_check",
        "type": "cargo",
        "command": "cargo check --manifest-path ./Cargo.toml -p tonic-profile",
    },
    {
        "name": "tonic_profile_contracts",
        "type": "cargo",
        "command": "cargo test --manifest-path ./Cargo.toml -p tonic-profile --test downstream_async_handle_contract --test accelerated_mode_contract",
    },
]

records = [run_record(spec) for spec in commands]
manifest = {
    "schema_version": 1,
    "milestone": "M008",
    "slice": "S04",
    "generated_at": datetime.now(timezone.utc).isoformat(),
    "run_dir": rel(run_dir),
    "logs_dir": rel(logs_dir),
    "artifacts_dir": rel(artifacts_dir),
    "records": records,
    "summary": {
        "total": len(records),
        "pass": sum(1 for r in records if r["interpretation"] == "pass"),
        "expected_failure": sum(1 for r in records if r["interpretation"] == "expected_failure"),
        "hard_failure": sum(1 for r in records if r["interpretation"] == "hard_failure"),
        "claim_eligible_hardware_pass": sum(1 for r in records if r["interpretation"] == "claim_eligible_hardware_pass"),
        "non_claim_eligible_pass": sum(1 for r in records if r["interpretation"] == "non_claim_eligible_pass"),
    },
}
manifest_path = run_dir / "manifest.json"
manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")

latest = base_dir / "latest"
if latest.is_symlink() or latest.exists():
    if latest.is_dir() and not latest.is_symlink():
        shutil.rmtree(latest)
    else:
        latest.unlink()
try:
    latest.symlink_to(run_dir.name, target_is_directory=True)
except OSError:
    shutil.copytree(run_dir, latest)

print(f"[collect_m008_s04_compatibility] manifest={rel(manifest_path)} latest={rel(latest / 'manifest.json')}")
if manifest["summary"]["hard_failure"]:
    print(
        f"[collect_m008_s04_compatibility] verdict=fail hard_failures={manifest['summary']['hard_failure']}",
        file=sys.stderr,
    )
    sys.exit(1)
print("[collect_m008_s04_compatibility] verdict=pass")
PY
