#!/usr/bin/env python3
"""Plot per-request completion timing from hw-eval Experiment 2 JSON."""

from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path
from typing import Any

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt


DEFAULT_INPUT = Path(
    "docs/report/benchmarking/submission_bottleneck_2026-05-27/"
    "marker_trace_all_request_completions_offset1_noop_rerun.json"
)
DEFAULT_OUTPUT = Path(
    "docs/report/benchmarking/submission_bottleneck_2026-05-27/"
    "request_completion_times_offset1_noop_rerun.png"
)


def load_results(path: Path) -> dict[str, Any]:
    text = path.read_text()
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        start = text.find("{")
        if start < 0:
            raise
        return json.loads(text[start:])


def stat(row: dict[str, Any], key: str, field: str = "median") -> float:
    return float(row[key][field])


def first_overlap_row(data: dict[str, Any]) -> dict[str, Any]:
    rows = data.get("submit_marker_overlap")
    if not isinstance(rows, list) or not rows:
        raise ValueError("input JSON does not contain submit_marker_overlap rows")
    return rows[0]


def write_csv(path: Path, request_rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow([
            "request_index",
            "observed_count",
            "success_count",
            "error_count",
            "completion_tsc_median",
            "completion_tsc_p99",
            "completion_ns_median",
            "observed_after_submit_index_median",
            "observed_from_marker_tsc_median",
        ])
        for row in request_rows:
            writer.writerow([
                row["request_index"],
                row["observed_count"],
                row["success_count"],
                row["error_count"],
                stat(row, "completion_tsc_ticks"),
                stat(row, "completion_tsc_ticks", "p99"),
                stat(row, "completion_ns"),
                stat(row, "observed_after_submit_index"),
                stat(row, "observed_from_marker_tsc"),
            ])


def plot(row: dict[str, Any], output: Path, title: str) -> None:
    requests = row["request_completions"]
    x = [int(r["request_index"]) for r in requests]
    completion_median = [stat(r, "completion_tsc_ticks") for r in requests]
    completion_p99 = [stat(r, "completion_tsc_ticks", "p99") for r in requests]
    observed_after = [stat(r, "observed_after_submit_index") for r in requests]
    from_marker = [stat(r, "observed_from_marker_tsc") for r in requests]

    fig, axes = plt.subplots(3, 1, figsize=(13.5, 10), sharex=True, constrained_layout=True)
    fig.suptitle(title)

    axes[0].plot(x, completion_median, label="median", linewidth=1.8)
    axes[0].plot(x, completion_p99, label="p99", linewidth=1.1, alpha=0.65)
    axes[0].set_ylabel("completion from own submit\n(TSC ticks)")
    axes[0].legend(loc="upper right")

    axes[1].plot(x, observed_after, linewidth=1.8, color="#2ca02c")
    axes[1].plot(x, x, linewidth=1.0, linestyle="--", color="#6b7280", label="observed at submit=request")
    axes[1].set_ylabel("first observed after\nsubmit index")
    axes[1].legend(loc="upper left")

    axes[2].plot(x, from_marker, linewidth=1.8, color="#d62728")
    axes[2].set_ylabel("observed from marker\n(TSC ticks)")
    axes[2].set_xlabel("request index")

    for ax in axes:
        ax.grid(True, color="#e5e7eb", linewidth=0.8)
        ax.set_xlim(min(x), max(x))

    marker_stats = row["marker_visible_tsc_ticks"]
    subtitle = (
        f"N={row['n']}, poll offset={row['marker_poll_offset']}, "
        f"iterations={requests[0]['observed_count']}; "
        f"request 1 median={int(marker_stats['median'])} TSC ticks"
    )
    axes[0].text(0.0, 1.02, subtitle, transform=axes[0].transAxes, fontsize=10, color="#4b5563", va="bottom")

    output.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(output, dpi=180)
    plt.close(fig)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Plot Experiment 2 per-request completion timing")
    parser.add_argument("input", nargs="?", type=Path, default=DEFAULT_INPUT)
    parser.add_argument("--output", "-o", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--csv", type=Path, default=None)
    parser.add_argument("--title", default="Experiment 2 rerun: all request completion observations")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    row = first_overlap_row(load_results(args.input))
    csv_path = args.csv if args.csv is not None else args.output.with_suffix(".csv")
    write_csv(csv_path, row["request_completions"])
    plot(row, args.output, args.title)
    print(f"wrote {args.output}")
    print(f"wrote {csv_path}")


if __name__ == "__main__":
    main()
