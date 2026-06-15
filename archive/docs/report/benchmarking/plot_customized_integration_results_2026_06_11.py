#!/usr/bin/env python3
"""Plot customized integration Criterion benchmark results."""

from __future__ import annotations

import csv
import json
import re
from dataclasses import dataclass
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt

REPO_ROOT = Path(__file__).resolve().parents[3]
BATCH_TARGET_BYTES = 16 * 1024 * 1024
CRITERION_ROOT = REPO_ROOT / "accel-rpc" / "tonic" / "target" / "criterion"
REPORT_ROOT = REPO_ROOT / "docs" / "report" / "benchmarking"
OUTPUT_DIR = REPORT_ROOT / "055.customized_integration_results_2026-06-11"
SUMMARY_CSV = OUTPUT_DIR / "criterion_summary.csv"

CURRENT_GROUPS = {
    "prost_async_payload_encode",
    "prost_async_payload_encode_cpu_thread_matrix",
    "prost_async_payload_encode_dsa",
    "prost_async_payload_encode_dsa_concurrency_matrix",
    "prost_async_payload_encode_dsa_thread_matrix",
}

PATH_LABELS = {
    "CPU Prost Message::encode": "CPU sync",
    "async CPU Prost encode_async_ref": "CPU async",
    "DSA Prost encode_async_ref submit+poll+complete": "DSA sequential",
}
PATH_COLORS = {
    "CPU Prost Message::encode": "#4C78A8",
    "async CPU Prost encode_async_ref": "#72B7B2",
    "DSA Prost encode_async_ref submit+poll+complete": "#F58518",
}
CASE_LABELS = {
    "tiny-bytes-128": "128 B",
    "tiny-bytes-512": "512 B",
    "tiny-bytes-1024": "1 KiB",
    "small-bytes-4k": "4 KiB",
    "large-bytes": "1 MiB",
}
CASE_ORDER = [
    "tiny-bytes-128",
    "tiny-bytes-512",
    "tiny-bytes-1024",
    "small-bytes-4k",
    "large-bytes",
]


@dataclass(frozen=True)
class Row:
    group: str
    case: str
    path: str
    threads: int | None
    concurrency: int | None
    requests: int
    bytes_per_request: int
    bytes_per_iter: int
    mean_ns: float
    ci_low_ns: float
    ci_high_ns: float
    mtime: float

    @property
    def gbps(self) -> float:
        return self.bytes_per_iter / self.mean_ns

    @property
    def mrps(self) -> float:
        return self.requests / (self.mean_ns * 1e-9) / 1_000_000.0

    @property
    def mean_ms(self) -> float:
        return self.mean_ns / 1_000_000.0


def metadata_int(function_id: str, key: str) -> int | None:
    match = re.search(rf"(?<![A-Za-z_]){key}=([0-9]+)", function_id)
    return int(match.group(1)) if match else None


def requests_per_iteration(bytes_per_request: int) -> int:
    return (BATCH_TARGET_BYTES + bytes_per_request - 1) // bytes_per_request


def matches_current_benchmark_metadata(row: Row) -> bool:
    base_requests = requests_per_iteration(row.bytes_per_request)
    if row.group == "prost_async_payload_encode_dsa_concurrency_matrix":
        if row.concurrency is None:
            return False
        expected_requests = ((base_requests + row.concurrency - 1) // row.concurrency) * row.concurrency
    else:
        expected_requests = base_requests
    return row.requests == expected_requests and row.bytes_per_iter == expected_requests * row.bytes_per_request


def load_rows() -> list[Row]:
    rows: list[Row] = []
    for benchmark_json in CRITERION_ROOT.rglob("new/benchmark.json"):
        estimates_json = benchmark_json.with_name("estimates.json")
        if not estimates_json.exists():
            continue
        benchmark = json.loads(benchmark_json.read_text())
        group = benchmark.get("group_id", "")
        if group not in CURRENT_GROUPS:
            continue
        function_id = benchmark.get("function_id", "")
        bytes_per_iter = metadata_int(function_id, "bytes_per_iter")
        bytes_per_request = metadata_int(function_id, "bytes_per_request")
        requests = metadata_int(function_id, "requests_per_iter") or metadata_int(function_id, "requests")
        if not (bytes_per_iter and bytes_per_request and requests):
            continue
        estimates = json.loads(estimates_json.read_text())
        mean = estimates["mean"]
        parts = function_id.split(" / ")
        row = Row(
            group=group,
            case=parts[0],
            path=parts[1] if len(parts) > 1 else "",
            threads=metadata_int(function_id, "threads"),
            concurrency=metadata_int(function_id, "concurrent_requests"),
            requests=requests,
            bytes_per_request=bytes_per_request,
            bytes_per_iter=bytes_per_iter,
            mean_ns=mean["point_estimate"],
            ci_low_ns=mean["confidence_interval"]["lower_bound"],
            ci_high_ns=mean["confidence_interval"]["upper_bound"],
            mtime=max(benchmark_json.stat().st_mtime, estimates_json.stat().st_mtime),
        )
        if matches_current_benchmark_metadata(row):
            rows.append(row)
    return rows


def dedupe_latest(rows: list[Row], key) -> list[Row]:
    deduped = {}
    for row in rows:
        row_key = key(row)
        if row_key not in deduped or row.mtime > deduped[row_key].mtime:
            deduped[row_key] = row
    return list(deduped.values())


def write_summary(rows: list[Row]) -> None:
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    with SUMMARY_CSV.open("w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(
            [
                "group",
                "case",
                "path",
                "threads",
                "concurrent_requests",
                "requests",
                "bytes_per_request",
                "bytes_per_iter",
                "mean_ms",
                "gbps",
                "mreq_s",
            ]
        )
        for row in sorted(rows, key=lambda r: (r.group, CASE_ORDER.index(r.case) if r.case in CASE_ORDER else 99, r.path, r.threads or 0, r.concurrency or 0)):
            writer.writerow(
                [
                    row.group,
                    row.case,
                    row.path,
                    row.threads or "",
                    row.concurrency or "",
                    row.requests,
                    row.bytes_per_request,
                    row.bytes_per_iter,
                    f"{row.mean_ms:.6f}",
                    f"{row.gbps:.6f}",
                    f"{row.mrps:.6f}",
                ]
            )


def plot_single_worker(rows: list[Row]) -> None:
    base = dedupe_latest(
        [row for row in rows if row.group == "prost_async_payload_encode"],
        lambda row: (row.case, row.path),
    )
    dsa = dedupe_latest(
        [row for row in rows if row.group == "prost_async_payload_encode_dsa"],
        lambda row: (row.case, row.path),
    )
    indexed = {(row.case, row.path): row for row in base + dsa}
    cases = ["small-bytes-4k", "large-bytes"]
    paths = list(PATH_LABELS)

    fig, ax = plt.subplots(figsize=(9.6, 5.4), constrained_layout=True)
    x_positions = range(len(cases))
    width = 0.22
    offsets = [-width, 0.0, width]
    for offset, path in zip(offsets, paths):
        values = [indexed[(case, path)].gbps for case in cases]
        bars = ax.bar(
            [x + offset for x in x_positions],
            values,
            width=width,
            label=PATH_LABELS[path],
            color=PATH_COLORS[path],
            edgecolor="white",
            linewidth=0.8,
        )
        ax.bar_label(bars, labels=[f"{value:.2f}" for value in values], padding=3, fontsize=9)

    ax.set_title("Single-worker encode throughput", fontweight="bold")
    ax.set_ylabel("Throughput (GB/s)")
    ax.set_xticks(list(x_positions))
    ax.set_xticklabels([CASE_LABELS[case] for case in cases])
    ax.grid(axis="y", alpha=0.28)
    ax.set_ylim(0, max(row.gbps for row in indexed.values()) * 1.25)
    ax.legend(frameon=False, ncol=3, loc="upper center", bbox_to_anchor=(0.5, -0.08))
    fig.savefig(OUTPUT_DIR / "single_worker_encode.png", dpi=220, bbox_inches="tight")
    plt.close(fig)


def plot_dsa_concurrency(rows: list[Row]) -> None:
    base = dedupe_latest(
        [row for row in rows if row.group == "prost_async_payload_encode"],
        lambda row: (row.case, row.path),
    )
    baseline = {(row.case, row.path): row for row in base}
    dsa_rows = [row for row in rows if row.group == "prost_async_payload_encode_dsa_concurrency_matrix"]
    cases = ["small-bytes-4k", "large-bytes"]

    fig, axes = plt.subplots(1, 2, figsize=(12.5, 4.8), constrained_layout=True)
    for ax, case in zip(axes, cases):
        case_rows = sorted([row for row in dsa_rows if row.case == case], key=lambda row: row.concurrency or 0)
        xs = [row.concurrency for row in case_rows]
        ys = [row.gbps for row in case_rows]
        ax.plot(xs, ys, marker="o", linewidth=2.0, color="#F58518", label="DSA concurrent")
        for row in case_rows:
            ax.annotate(f"{row.gbps:.1f}", (row.concurrency, row.gbps), textcoords="offset points", xytext=(0, 6), ha="center", fontsize=8)
        for path, color in [
            ("CPU Prost Message::encode", "#4C78A8"),
            ("async CPU Prost encode_async_ref", "#72B7B2"),
        ]:
            row = baseline[(case, path)]
            ax.axhline(row.gbps, linestyle="--", linewidth=1.3, color=color, label=PATH_LABELS[path])
        ax.set_title(CASE_LABELS[case], fontweight="bold")
        ax.set_xlabel("Concurrent requests")
        ax.set_xscale("log", base=2)
        ax.set_xticks(xs)
        ax.set_xticklabels([str(x) for x in xs])
        ax.set_ylim(bottom=0)
        ax.set_ylabel("Throughput (GB/s)")
        ax.grid(True, which="major", alpha=0.28)
    handles, labels = axes[0].get_legend_handles_labels()
    fig.legend(handles, labels, loc="lower center", frameon=False, ncol=3, bbox_to_anchor=(0.5, -0.05))
    fig.suptitle("DSA throughput vs per-batch concurrency", fontweight="bold", fontsize=14)
    fig.savefig(OUTPUT_DIR / "dsa_concurrency_matrix.png", dpi=220, bbox_inches="tight")
    plt.close(fig)


def plot_cpu_thread_matrix(rows: list[Row]) -> None:
    cpu_rows = [row for row in rows if row.group == "prost_async_payload_encode_cpu_thread_matrix"]
    cases = [case for case in CASE_ORDER if any(row.case == case for row in cpu_rows)]
    thread_counts = sorted({row.threads for row in cpu_rows if row.threads is not None})

    fig, ax = plt.subplots(figsize=(10.8, 5.8), constrained_layout=True)
    for case in cases:
        case_rows = {row.threads: row for row in cpu_rows if row.case == case}
        xs = [thread_count for thread_count in thread_counts if thread_count in case_rows]
        ys = [case_rows[thread_count].gbps for thread_count in xs]
        ax.plot(xs, ys, marker="o", linewidth=2.0, label=CASE_LABELS[case])
        for x, y in zip(xs, ys):
            ax.annotate(f"{y:.1f}", (x, y), textcoords="offset points", xytext=(0, 6), ha="center", fontsize=8)

    ax.set_title("CPU thread matrix: all current rows", fontweight="bold")
    ax.set_xlabel("Threads")
    ax.set_ylabel("Throughput (GB/s)")
    ax.set_xscale("log", base=2)
    ax.set_xticks(thread_counts)
    ax.set_xticklabels([str(thread_count) for thread_count in thread_counts])
    ax.set_ylim(bottom=0)
    ax.grid(True, which="major", alpha=0.28)
    ax.legend(frameon=False, ncol=3, loc="upper center", bbox_to_anchor=(0.5, -0.10))
    fig.savefig(OUTPUT_DIR / "cpu_thread_matrix_all.png", dpi=220, bbox_inches="tight")
    plt.close(fig)


def plot_dsa_thread_matrix(rows: list[Row]) -> None:
    dsa_rows = [row for row in rows if row.group == "prost_async_payload_encode_dsa_thread_matrix"]
    cases = [case for case in CASE_ORDER if any(row.case == case for row in dsa_rows)]
    thread_counts = sorted({row.threads for row in dsa_rows if row.threads is not None})
    concurrency_counts = sorted({row.concurrency for row in dsa_rows if row.concurrency is not None})

    fig, axes = plt.subplots(2, 3, figsize=(14.5, 8.6), constrained_layout=True)
    axes_flat = list(axes.flat)
    for ax, case in zip(axes_flat, cases):
        case_rows = [row for row in dsa_rows if row.case == case]
        by_config = {(row.threads, row.concurrency): row for row in case_rows}
        for thread_count in thread_counts:
            xs = [concurrency for concurrency in concurrency_counts if (thread_count, concurrency) in by_config]
            if not xs:
                continue
            ys = [by_config[(thread_count, concurrency)].gbps for concurrency in xs]
            ax.plot(xs, ys, marker="o", linewidth=1.8, label=f"{thread_count} threads")
        ax.set_title(CASE_LABELS[case], fontweight="bold")
        ax.set_xlabel("Concurrent requests")
        ax.set_ylabel("Throughput (GB/s)")
        ax.set_xscale("log", base=2)
        ax.set_xticks(concurrency_counts)
        ax.set_xticklabels([str(concurrency) for concurrency in concurrency_counts])
        ax.set_ylim(bottom=0)
        ax.grid(True, which="major", alpha=0.28)

    handles, labels = axes_flat[0].get_legend_handles_labels()
    unused_axes = axes_flat[len(cases) :]
    for ax in unused_axes:
        ax.axis("off")
    if unused_axes:
        unused_axes[0].legend(handles, labels, loc="center", frameon=False, title="Thread count")

    fig.suptitle("DSA thread/concurrency matrix: all current rows", fontweight="bold", fontsize=14)
    fig.savefig(OUTPUT_DIR / "dsa_thread_matrix_all.png", dpi=220, bbox_inches="tight")
    plt.close(fig)



def analysis_rows(rows: list[Row]) -> list[Row]:
    return (
        dedupe_latest(
            [row for row in rows if row.group == "prost_async_payload_encode"],
            lambda row: (row.case, row.path),
        )
        + [row for row in rows if row.group == "prost_async_payload_encode_cpu_thread_matrix"]
        + dedupe_latest(
            [row for row in rows if row.group == "prost_async_payload_encode_dsa"],
            lambda row: (row.case, row.path),
        )
        + [row for row in rows if row.group == "prost_async_payload_encode_dsa_concurrency_matrix"]
        + [row for row in rows if row.group == "prost_async_payload_encode_dsa_thread_matrix"]
    )


def main() -> None:
    rows = dedupe_latest(
        load_rows(),
        lambda row: (row.group, row.case, row.path, row.threads or 0, row.concurrency or 0),
    )
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    write_summary(analysis_rows(rows))
    plot_single_worker(rows)
    plot_dsa_concurrency(rows)
    plot_cpu_thread_matrix(rows)
    plot_dsa_thread_matrix(rows)
    print(OUTPUT_DIR)


if __name__ == "__main__":
    main()
