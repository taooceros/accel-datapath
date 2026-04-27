#!/usr/bin/env python3
"""Export a stable async control-floor summary from Criterion output."""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

SCHEMA_VERSION = 1
DEFAULT_SUITE = "async_control_floor"


class ExportError(RuntimeError):
    pass


@dataclass
class BenchmarkSummary:
    name: str
    benchmark_id: str
    criterion_directory: str
    benchmark_source: str
    estimates_source: str
    sample_source: str
    mean_ns: float
    median_ns: float
    slope_ns: float
    std_dev_ns: float
    sample_count: int
    sampling_mode: str

    def to_json(self) -> dict[str, Any]:
        return {
            "benchmark_name": self.name,
            "benchmark_id": self.benchmark_id,
            "criterion_directory": self.criterion_directory,
            "benchmark_source": self.benchmark_source,
            "estimates_source": self.estimates_source,
            "sample_source": self.sample_source,
            "mean_ns": self.mean_ns,
            "median_ns": self.median_ns,
            "slope_ns": self.slope_ns,
            "std_dev_ns": self.std_dev_ns,
            "sample_count": self.sample_count,
            "sampling_mode": self.sampling_mode,
        }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--criterion-root",
        type=Path,
        help="Criterion output root (required for export mode)",
    )
    parser.add_argument(
        "--out",
        type=Path,
        help="Path to write the exported JSON summary (required for export mode)",
    )
    parser.add_argument(
        "--suite-name",
        default=DEFAULT_SUITE,
        help=f"Expected Criterion group/suite name (default: {DEFAULT_SUITE})",
    )
    parser.add_argument(
        "--expected-benchmark",
        action="append",
        default=[],
        help="Benchmark short name expected in the suite; repeat for multiple entries",
    )
    parser.add_argument(
        "--validate-existing",
        type=Path,
        help="Validate an existing exported summary instead of reading Criterion output",
    )
    return parser.parse_args()


def load_json(path: Path) -> dict[str, Any]:
    try:
        with path.open("r", encoding="utf-8") as handle:
            return json.load(handle)
    except FileNotFoundError as exc:
        raise ExportError(f"missing JSON input: {path}") from exc
    except json.JSONDecodeError as exc:
        raise ExportError(f"failed to parse JSON from {path}: {exc}") from exc


def require_mapping(value: Any, description: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ExportError(f"{description} must be a JSON object")
    return value


def require_non_empty_string(value: Any, description: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ExportError(f"{description} must be a non-empty string")
    return value


def require_number(value: Any, description: str) -> float:
    if not isinstance(value, (int, float)):
        raise ExportError(f"{description} must be numeric")
    return float(value)


def benchmark_name_from_metadata(metadata: dict[str, Any], suite_name: str) -> tuple[str, str]:
    group_id = require_non_empty_string(metadata.get("group_id"), "benchmark group_id")
    if group_id != suite_name:
        raise ExportError(
            f"benchmark group_id mismatch: expected {suite_name}, found {group_id}"
        )
    function_id = metadata.get("function_id")
    if function_id is None:
        name = group_id
    else:
        name = require_non_empty_string(function_id, "benchmark function_id")
    benchmark_id = require_non_empty_string(
        metadata.get("full_id") or metadata.get("title"), "benchmark id"
    )
    return name, benchmark_id


def summarize_benchmark(directory: Path, suite_name: str) -> BenchmarkSummary:
    benchmark_json = load_json(directory / "new" / "benchmark.json")
    estimates_json = load_json(directory / "new" / "estimates.json")
    sample_json = load_json(directory / "new" / "sample.json")

    benchmark_meta = require_mapping(benchmark_json, f"benchmark metadata in {directory}")
    estimates = require_mapping(estimates_json, f"estimates in {directory}")
    sample = require_mapping(sample_json, f"sample metadata in {directory}")

    name, benchmark_id = benchmark_name_from_metadata(benchmark_meta, suite_name)
    mean = require_mapping(estimates.get("mean"), f"mean estimate for {name}")
    median = require_mapping(estimates.get("median"), f"median estimate for {name}")
    slope = require_mapping(estimates.get("slope"), f"slope estimate for {name}")
    std_dev = require_mapping(estimates.get("std_dev"), f"std_dev estimate for {name}")
    sampling_mode = require_non_empty_string(sample.get("sampling_mode"), f"sampling mode for {name}")
    times = sample.get("times")
    if not isinstance(times, list) or not times:
        raise ExportError(f"sample times for {name} must be a non-empty list")

    return BenchmarkSummary(
        name=name,
        benchmark_id=benchmark_id,
        criterion_directory=str(directory),
        benchmark_source=str(directory / "new" / "benchmark.json"),
        estimates_source=str(directory / "new" / "estimates.json"),
        sample_source=str(directory / "new" / "sample.json"),
        mean_ns=require_number(mean.get("point_estimate"), f"mean point_estimate for {name}"),
        median_ns=require_number(median.get("point_estimate"), f"median point_estimate for {name}"),
        slope_ns=require_number(slope.get("point_estimate"), f"slope point_estimate for {name}"),
        std_dev_ns=require_number(std_dev.get("point_estimate"), f"std_dev point_estimate for {name}"),
        sample_count=len(times),
        sampling_mode=sampling_mode,
    )


def export_summary(
    criterion_root: Path, out_path: Path, suite_name: str, expected_benchmarks: list[str]
) -> dict[str, Any]:
    suite_root = criterion_root / suite_name
    if not suite_root.exists():
        raise ExportError(f"missing Criterion suite directory: {suite_root}")

    benchmark_directories = sorted(
        entry for entry in suite_root.iterdir() if entry.is_dir() and (entry / "new" / "benchmark.json").exists()
    )
    if not benchmark_directories:
        raise ExportError(f"no benchmark directories found under {suite_root}")

    summaries = [summarize_benchmark(directory, suite_name) for directory in benchmark_directories]
    benchmark_map = {summary.name: summary.to_json() for summary in summaries}

    missing = [name for name in expected_benchmarks if name not in benchmark_map]
    if missing:
        raise ExportError(f"missing expected benchmark ids: {', '.join(missing)}")

    payload = {
        "schema_version": SCHEMA_VERSION,
        "suite_name": suite_name,
        "criterion_root": str(criterion_root),
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "benchmarks": benchmark_map,
    }

    out_path.parent.mkdir(parents=True, exist_ok=True)
    with out_path.open("w", encoding="utf-8") as handle:
        json.dump(payload, handle, indent=2, sort_keys=True)
        handle.write("\n")

    return payload


def validate_summary(summary_path: Path, suite_name: str, expected_benchmarks: list[str]) -> dict[str, Any]:
    payload = require_mapping(load_json(summary_path), f"exported summary {summary_path}")
    schema_version = payload.get("schema_version")
    if schema_version != SCHEMA_VERSION:
        raise ExportError(
            f"summary schema_version mismatch: expected {SCHEMA_VERSION}, found {schema_version}"
        )
    if require_non_empty_string(payload.get("suite_name"), "suite_name") != suite_name:
        raise ExportError(
            f"suite_name mismatch: expected {suite_name}, found {payload.get('suite_name')}"
        )

    benchmarks = require_mapping(payload.get("benchmarks"), "benchmarks")
    missing = [name for name in expected_benchmarks if name not in benchmarks]
    if missing:
        raise ExportError(f"missing expected benchmark ids: {', '.join(missing)}")

    for name, entry in benchmarks.items():
        benchmark = require_mapping(entry, f"benchmark entry for {name}")
        require_non_empty_string(benchmark.get("benchmark_name"), f"benchmark_name for {name}")
        require_non_empty_string(benchmark.get("benchmark_id"), f"benchmark_id for {name}")
        require_non_empty_string(benchmark.get("criterion_directory"), f"criterion_directory for {name}")
        require_non_empty_string(benchmark.get("benchmark_source"), f"benchmark_source for {name}")
        require_non_empty_string(benchmark.get("estimates_source"), f"estimates_source for {name}")
        require_non_empty_string(benchmark.get("sample_source"), f"sample_source for {name}")
        require_non_empty_string(benchmark.get("sampling_mode"), f"sampling_mode for {name}")
        require_number(benchmark.get("mean_ns"), f"mean_ns for {name}")
        require_number(benchmark.get("median_ns"), f"median_ns for {name}")
        require_number(benchmark.get("slope_ns"), f"slope_ns for {name}")
        require_number(benchmark.get("std_dev_ns"), f"std_dev_ns for {name}")
        sample_count = benchmark.get("sample_count")
        if not isinstance(sample_count, int) or sample_count <= 0:
            raise ExportError(f"sample_count for {name} must be a positive integer")

    return payload


def main() -> int:
    args = parse_args()
    expected = args.expected_benchmark

    try:
        if args.validate_existing:
            payload = validate_summary(args.validate_existing, args.suite_name, expected)
            print(
                f"validated control-floor summary: suite={payload['suite_name']} benchmarks={len(payload['benchmarks'])} path={args.validate_existing}"
            )
            return 0

        if args.criterion_root is None or args.out is None:
            raise ExportError("--criterion-root and --out are required in export mode")

        payload = export_summary(args.criterion_root, args.out, args.suite_name, expected)
        print(
            f"exported control-floor summary: suite={payload['suite_name']} benchmarks={len(payload['benchmarks'])} path={args.out}"
        )
        return 0
    except ExportError as exc:
        print(f"export_control_floor.py: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
