#!/usr/bin/env python3
"""Verify the IDXD Tokio results report against tracked benchmark artifacts.

Checks:
- Typst hard-coded `threads` and `scenarios` data match the NUMA0 Tokio JSONL
  and refreshed raw hw-eval JSON artifacts at c=128.
- The Typst deck compiles.
- The rendered PDF has one overview page plus one page per scenario.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

SCRIPT = Path(__file__).resolve()
REPORT_DIR = SCRIPT.parent
REPO_ROOT = SCRIPT.parents[2]
REPORT = REPORT_DIR / "idxd_tokio_results_report.typ"
PDF = REPORT_DIR / "idxd_tokio_results_report.pdf"
DATA_DIR = REPO_ROOT / "docs/report/benchmarking/shared_thread_sweep_numa0"

EXPECTED_SIZES = (128, 256, 1024, 4096)
CONCURRENCY = 128
TOL = 0.0005


def parse_number_list(text: str, kind: type[float] | type[int] = float):
    parts = [part.strip() for part in text.split(",") if part.strip()]
    return [kind(part) for part in parts]


def parse_typst_report():
    text = REPORT.read_text()

    threads_match = re.search(r"#let\s+threads\s*=\s*\(([^)]*)\)", text)
    if not threads_match:
        raise RuntimeError("could not find `#let threads = (...)` in report")
    threads = parse_number_list(threads_match.group(1), int)

    scenario_pattern = re.compile(
        r"\(\s*"
        r"title:\s*\[[^\]]+\],\s*"
        r"note:\s*\[[^\]]+\],\s*"
        r"bytes:\s*(\d+),\s*"
        r"tokio:\s*\(([^)]*)\),\s*"
        r"raw:\s*\(([^)]*)\),",
        re.DOTALL,
    )
    scenarios = []
    for match in scenario_pattern.finditer(text):
        scenarios.append(
            {
                "bytes": int(match.group(1)),
                "tokio": parse_number_list(match.group(2), float),
                "raw": parse_number_list(match.group(3), float),
            }
        )

    if not scenarios:
        raise RuntimeError("could not find scenario data in report")
    return threads, scenarios


def tokio_mops(thread: int, size: int) -> float:
    path = DATA_DIR / f"tokio_wq0_1_numa0_threads{thread}.jsonl"
    if not path.exists():
        raise RuntimeError(f"missing Tokio artifact: {path}")
    for line in path.read_text().splitlines():
        if not line.strip():
            continue
        row = json.loads(line)
        if row["message_bytes"] == size and row["concurrency"] == CONCURRENCY:
            return row["ops_per_second"] / 1_000_000
    raise RuntimeError(f"missing Tokio row: thread={thread}, size={size}, c={CONCURRENCY}")


def raw_mops(thread: int, size: int) -> float:
    path = DATA_DIR / f"hw_eval_wq0_1_numa0_threads{thread}.json"
    if not path.exists():
        raise RuntimeError(f"missing raw hw-eval artifact: {path}")
    data = json.loads(path.read_text())
    benchmark = "memmove" if thread == 1 else f"memmove_mt_t{thread}"
    for row in data["throughput"]:
        if (
            row["benchmark"] == benchmark
            and row["size"] == size
            and row["concurrency"] == CONCURRENCY
        ):
            return row["ops_per_sec"] / 1_000_000
    raise RuntimeError(
        f"missing raw row: thread={thread}, size={size}, c={CONCURRENCY}, benchmark={benchmark}"
    )


def assert_rounded_equal(label: str, report_value: float, source_value: float):
    rounded = round(source_value, 3)
    if abs(report_value - rounded) > TOL:
        raise AssertionError(
            f"{label}: report has {report_value:.3f}, source rounds to {rounded:.3f} "
            f"(raw source {source_value:.9f})"
        )


def verify_data():
    threads, scenarios = parse_typst_report()
    if tuple(s["bytes"] for s in scenarios) != EXPECTED_SIZES:
        raise AssertionError(
            f"scenario sizes are {[s['bytes'] for s in scenarios]}, expected {list(EXPECTED_SIZES)}"
        )

    for scenario in scenarios:
        size = scenario["bytes"]
        if len(scenario["tokio"]) != len(threads):
            raise AssertionError(f"Tokio series for size {size} does not match thread count")
        if len(scenario["raw"]) != len(threads):
            raise AssertionError(f"raw series for size {size} does not match thread count")

        for thread, value in zip(threads, scenario["tokio"]):
            assert_rounded_equal(f"Tokio size={size} thread={thread}", value, tokio_mops(thread, size))
        for thread, value in zip(threads, scenario["raw"]):
            assert_rounded_equal(f"raw size={size} thread={thread}", value, raw_mops(thread, size))

    print(f"data check passed: {len(scenarios)} scenarios x {len(threads)} thread counts")
    return threads, scenarios


def run_command(args: list[str], cwd: Path = REPO_ROOT) -> str:
    proc = subprocess.run(args, cwd=cwd, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if proc.returncode != 0:
        raise RuntimeError(
            f"command failed ({proc.returncode}): {' '.join(args)}\nSTDOUT:\n{proc.stdout}\nSTDERR:\n{proc.stderr}"
        )
    return proc.stdout


def verify_render(expected_pages: int):
    run_command(
        [
            "typst",
            "compile",
            "--root",
            "presentation",
            str(REPORT.relative_to(REPO_ROOT)),
            str(PDF.relative_to(REPO_ROOT)),
        ]
    )
    info = run_command(["pdfinfo", str(PDF.relative_to(REPO_ROOT))])
    pages_match = re.search(r"^Pages:\s*(\d+)$", info, re.MULTILINE)
    if not pages_match:
        raise RuntimeError("pdfinfo output did not include a Pages line")
    pages = int(pages_match.group(1))
    if pages != expected_pages:
        raise AssertionError(f"PDF has {pages} pages, expected {expected_pages}")
    print(f"render check passed: {PDF.relative_to(REPO_ROOT)} has {pages} pages")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--data-only", action="store_true", help="skip Typst/PDF render checks")
    args = parser.parse_args()

    _threads, scenarios = verify_data()
    if not args.data_only:
        verify_render(expected_pages=1 + len(scenarios))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:  # noqa: BLE001 - script should report concise failure.
        print(f"verification failed: {exc}", file=sys.stderr)
        raise SystemExit(1)
