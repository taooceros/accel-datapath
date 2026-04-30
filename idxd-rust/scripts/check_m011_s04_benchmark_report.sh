#!/usr/bin/env bash
set -euo pipefail

REPORT_PATH=${1:-docs/report/benchmarking/015.m011_representative_idxd_numbers_2026-04-30.md}

python3 - <<'PY' "${REPORT_PATH}"
import re
import sys
from pathlib import Path

report_path = Path(sys.argv[1])
if not report_path.is_file():
    raise SystemExit(f"missing report: {report_path}")

text = report_path.read_text(encoding="utf-8")

required_snippets = [
    "R023",
    "IdxdSession<Accel>",
    "idxd_representative_bench",
    "verify_idxd_representative_bench.sh",
    "dsa-memmove",
    "iax-crc64",
    "verdict=pass",
    "launcher_status=ready",
    "profile=release",
    "claim_eligible=true",
    "artifact=target/m011-s04-representative-bench/idxd_representative_bench.json",
    "stdout=target/m011-s04-representative-bench/idxd_representative_bench.stdout",
    "stderr=target/m011-s04-representative-bench/idxd_representative_bench.stderr",
    "target/m011-s04-representative-bench/verify.log",
    "target/m011-s04-representative-bench/idxd_representative_bench.stdout.raw",
    "requested_bytes=4096",
    "iterations=1000",
    "warmup_iterations=1",
    "elapsed_ns",
    "min_latency_ns",
    "mean_latency_ns",
    "max_latency_ns",
    "ops_per_sec",
    "bytes_per_sec",
    "completed_operations == iterations",
    "failed_operations == 0",
    "crc64_verified=true",
    "no-payload contract",
    "raw buffer bytes",
    "not a benchmark matrix",
    "not a full performance characterization",
    "not a final performance comparison",
    "does not claim optional shared DSA",
    "docs/report/hw_eval/011.m011_s03_representative_ops_2026-04-30.md",
    "S05",
]
missing = [snippet for snippet in required_snippets if snippet not in text]
if missing:
    raise SystemExit("report missing required S04 benchmark terms: " + ", ".join(missing))

final_lines = [
    line for line in text.splitlines()
    if line.startswith("[verify_idxd_representative_bench] phase=done ")
]
if len(final_lines) != 1:
    raise SystemExit(f"expected exactly one final verifier line, found {len(final_lines)}")
final_line = final_lines[0]
for token in [
    "output_dir=target/m011-s04-representative-bench",
    "verdict=pass",
    "launcher_status=ready",
    "profile=release",
    "requested_bytes=4096",
    "iterations=1000",
    "claim_eligible=true",
    "completed_operations=2000",
    "failed_operations=0",
    "targets=dsa-memmove:/dev/dsa/wq0.0,iax-crc64:/dev/iax/wq1.0",
    "artifact_targets=dsa-memmove,iax-crc64",
    "artifact=target/m011-s04-representative-bench/idxd_representative_bench.json",
    "stdout=target/m011-s04-representative-bench/idxd_representative_bench.stdout",
    "stderr=target/m011-s04-representative-bench/idxd_representative_bench.stderr",
]:
    if token not in final_line:
        raise SystemExit(f"final verifier line missing {token}")

row_pattern = re.compile(r"^\| `(?P<target>dsa-memmove|iax-crc64)` \|(?P<rest>.+)\|$", re.MULTILINE)
rows = {}
for match in row_pattern.finditer(text):
    cells = [cell.strip() for cell in match.group(0).strip().strip("|").split("|")]
    rows[match.group("target")] = cells

if set(rows) != {"dsa-memmove", "iax-crc64"}:
    raise SystemExit(f"expected exactly dsa-memmove and iax-crc64 measured rows, found {sorted(rows)}")

# Columns: target, family, device, WQ mode, operation, bytes, iterations,
# completed, failed, elapsed, min, mean, max, ops/s, bytes/s, status, retries,
# CRC verified, claim eligible.
def parse_int(cell: str) -> int:
    return int(cell.replace(",", ""))

def parse_float(cell: str) -> float:
    return float(cell.replace(",", ""))

expected = {
    "dsa-memmove": {
        "family": "`dsa`",
        "device": "`/dev/dsa/wq0.0`",
        "operation": "`memmove`",
        "crc": "n/a",
    },
    "iax-crc64": {
        "family": "`iax`",
        "device": "`/dev/iax/wq1.0`",
        "operation": "`crc64`",
        "crc": "`true`",
    },
}
for target, cells in rows.items():
    if len(cells) != 19:
        raise SystemExit(f"row {target} has {len(cells)} cells, expected 19")
    _, family, device, mode, operation, bytes_cell, iterations, completed, failed, elapsed, min_latency, mean_latency, max_latency, ops_sec, bytes_sec, status, retries, crc_verified, claim = cells
    spec = expected[target]
    if family != spec["family"] or device != spec["device"] or operation != spec["operation"]:
        raise SystemExit(f"row {target} has unexpected family/device/operation")
    if mode != "`dedicated`":
        raise SystemExit(f"row {target} must record dedicated WQ mode")
    if parse_int(bytes_cell) != 4096:
        raise SystemExit(f"row {target} requested bytes mismatch")
    if parse_int(iterations) != 1000 or parse_int(completed) != 1000:
        raise SystemExit(f"row {target} must complete all 1000 iterations")
    if parse_int(failed) != 0:
        raise SystemExit(f"row {target} must have zero failed operations")
    for label, cell in [
        ("elapsed_ns", elapsed),
        ("min_latency_ns", min_latency),
        ("mean_latency_ns", mean_latency),
        ("max_latency_ns", max_latency),
    ]:
        if parse_int(cell) <= 0:
            raise SystemExit(f"row {target} {label} must be positive")
    for label, cell in [("ops_per_sec", ops_sec), ("bytes_per_sec", bytes_sec)]:
        if parse_float(cell) <= 0.0:
            raise SystemExit(f"row {target} {label} must be positive")
    if status != "`0x01`" or parse_int(retries) != 0:
        raise SystemExit(f"row {target} must preserve success status and retry count")
    if crc_verified != spec["crc"]:
        raise SystemExit(f"row {target} CRC verification field mismatch")
    if claim != "`true`":
        raise SystemExit(f"row {target} must be claim eligible")

if "dsa-shared-memmove` |" in text:
    raise SystemExit("report must not include an unclaimed optional shared DSA measured row")

print(f"report_guard=pass path={report_path}")
PY
