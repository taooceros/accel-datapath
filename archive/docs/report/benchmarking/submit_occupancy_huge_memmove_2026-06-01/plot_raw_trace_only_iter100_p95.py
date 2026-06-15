#!/usr/bin/env python3
"""Post-process raw submit-occupancy trace JSON and plot p95 statistics for 100 iterations."""

from __future__ import annotations

import json
from pathlib import Path
from statistics import mean

import matplotlib.pyplot as plt

BASE = Path(__file__).resolve().parent
SERIES = [
    ("NOOP", "raw_trace_only_k112_iter100_noop.json"),
    ("64 KiB", "raw_trace_only_k112_iter100_memmove_65536.json"),
    ("256 KiB", "raw_trace_only_k112_iter100_memmove_262144.json"),
    ("1 MiB", "raw_trace_only_k112_iter100_memmove_1048576.json"),
]
SLOW_NS = 50


def load_json_with_launcher_prefix(path: Path) -> dict:
    lines = path.read_text().splitlines()
    if lines and lines[0].startswith("Running:"):
        lines = lines[1:]
    return json.loads("\n".join(lines))


def percentile(sorted_values: list[int], fraction: float) -> int:
    index = min(int(len(sorted_values) * fraction), len(sorted_values) - 1)
    return sorted_values[index]


def raw_trace_stats(filename: str):
    data = load_json_with_launcher_prefix(BASE / filename)
    trace_rows = [
        row
        for row in data["submit_occupancy"]
        if row.get("benchmark") == "submit_occupancy_trace"
    ]
    if len(trace_rows) != 1:
        raise ValueError(f"expected one trace row in {filename}, got {len(trace_rows)}")
    row = trace_rows[0]

    bad_outcomes = [
        outcome
        for outcome in row["trace_outcomes"]
        if outcome["missing"] != 0 or outcome["errors"] != 0
    ]
    if bad_outcomes:
        print(
            f"warning: {filename} has {len(bad_outcomes)} non-clean outcomes; "
            "plotting submit trace points anyway"
        )

    by_submit: dict[int, list[int]] = {}
    for point in row["extra_submit_trace"]:
        by_submit.setdefault(point["submit_index"], []).append(point["submit_ns"])

    xs = sorted(by_submit)
    mins: list[int] = []
    medians: list[int] = []
    means: list[float] = []
    p95s: list[int] = []
    slow_counts: list[int] = []
    sample_counts: list[int] = []
    for submit_index in xs:
        values = sorted(by_submit[submit_index])
        sample_counts.append(len(values))
        mins.append(values[0])
        medians.append(percentile(values, 0.50))
        means.append(mean(values))
        p95s.append(percentile(values, 0.95))
        slow_counts.append(sum(value >= SLOW_NS for value in values))

    return xs, mins, medians, means, p95s, slow_counts, sample_counts


def main() -> None:
    fig, axes = plt.subplots(2, 2, figsize=(14, 8.5), sharex=True, sharey=True)
    axes = axes.ravel()

    for ax, (label, filename) in zip(axes, SERIES, strict=True):
        xs, mins, medians, means, p95s, slow_counts, sample_counts = raw_trace_stats(filename)
        ax.fill_between(xs, mins, p95s, color="#4c78a8", alpha=0.20, label="min..p95")
        ax.plot(xs, medians, color="black", marker="o", linewidth=2, markersize=4, label="median")
        ax.plot(xs, means, color="#f58518", marker="x", linewidth=1.6, markersize=4, label="mean")
        ax.plot(xs, p95s, color="#4c78a8", linestyle="--", linewidth=1.4, label="p95")
        ax.axhline(SLOW_NS, color="0.45", linestyle=":", linewidth=1)

        for x, y, slow_count, sample_count in zip(
            xs, medians, slow_counts, sample_counts, strict=True
        ):
            ax.annotate(
                f"{slow_count}/{sample_count}",
                (x, y),
                textcoords="offset points",
                xytext=(0, 7),
                ha="center",
                fontsize=6,
            )

        ax.set_title(label)
        ax.grid(True, alpha=0.25)
        ax.set_xticks(xs)
        ax.tick_params(axis="x", rotation=45)

    axes[0].legend(frameon=False, loc="upper left", fontsize=9)
    fig.supxlabel("submit_index")
    fig.supylabel("extra submit latency (ns)")
    fig.suptitle(
        "Raw trace post-processing: K=112, trace until 128, 100 iterations",
        fontsize=14,
    )
    fig.tight_layout(rect=(0, 0, 1, 0.95))

    out_png = BASE / "raw_trace_only_k112_iter100_submit_ns_p95.png"
    out_pdf = BASE / "raw_trace_only_k112_iter100_submit_ns_p95.pdf"
    fig.savefig(out_png, dpi=180)
    fig.savefig(out_pdf)
    print(out_png)
    print(out_pdf)

    for label, filename in SERIES:
        xs, _mins, medians, _means, p95s, slow_counts, sample_counts = raw_trace_stats(filename)
        print(f"\n{label}")
        print("| submit_index | median ns | p95 ns | slow >=50ns |")
        print("|---:|---:|---:|---:|")
        for x, median_ns, p95_ns, slow_count, sample_count in zip(
            xs, medians, p95s, slow_counts, sample_counts, strict=True
        ):
            print(f"| {x} | {median_ns} | {p95_ns} | {slow_count}/{sample_count} |")


if __name__ == "__main__":
    main()
