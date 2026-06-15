#!/usr/bin/env python3
"""Plot Experiment 1 submit-occupancy spin sweep results."""

from __future__ import annotations

import csv
import json
from pathlib import Path
from statistics import mean, median

import matplotlib.pyplot as plt

REPO = Path(__file__).resolve().parents[3]
RESULTS = REPO / "hw-eval" / "results" / "2026-06-02-spin-sweep"
OUT_STEM = Path(__file__).resolve().parent / "041.submit_occupancy_spin_sweep_2026-06-02"
SPINS = [0, 1, 2, 5, 10, 20, 50, 100, 1000, 10000]
WORKLOADS = [
    ("noop", "NOOP"),
    ("memmove1m", "1 MiB memmove"),
]
K_VALUES = [96, 108, 112, 114, 115, 116, 120, 124, 128, 136, 144, 160]


def load_json_with_launcher_prefix(path: Path) -> dict:
    text = path.read_text()
    start = text.find("{")
    if start < 0:
        raise ValueError(f"missing JSON object in {path}")
    return json.loads(text[start:])


def percentile(values: list[int], fraction: float) -> int:
    values = sorted(values)
    index = min(round((len(values) - 1) * fraction), len(values) - 1)
    return values[index]


def collect_rows() -> list[dict[str, int | str | float]]:
    rows: list[dict[str, int | str | float]] = []
    for workload, label in WORKLOADS:
        for spin in SPINS:
            data = load_json_with_launcher_prefix(RESULTS / f"{workload}_spin{spin}.json")
            for result in data["submit_occupancy"]:
                submit_values = [
                    point["submit_ns"]
                    for point in result["extra_submit_trace"]
                    if point["submit_index"] == result["k_prefill"]
                ]
                if not submit_values:
                    raise ValueError(f"missing measured submit trace for {workload} spin={spin} K={result['k_prefill']}")

                prefill_submit_ticks = [
                    point["prefill_submit_tsc_ticks"]
                    for point in result["prefill_completion_trace"]
                ]
                prefill_submit = [
                    point["prefill_submit_ns"]
                    for point in result["prefill_completion_trace"]
                ]
                post_submit_completion = [
                    point["post_submit_completion_ns"]
                    for point in result["prefill_completion_trace"]
                ]
                completed = [
                    point["completed"]
                    for point in result["prefill_completion_trace"]
                ]
                rows.append(
                    {
                        "workload": workload,
                        "label": label,
                        "spin": spin,
                        "k_prefill": result["k_prefill"],
                        "median_submit_ns": median(submit_values),
                        "p90_submit_ns": percentile(submit_values, 0.90),
                        "p99_submit_ns": percentile(submit_values, 0.99),
                        "avg_prefill_submit_tsc_ticks": mean(prefill_submit_ticks),
                        "median_prefill_submit_ns": median(prefill_submit),
                        "median_post_submit_completion_ns": median(post_submit_completion),
                        "min_completed": min(completed),
                        "max_completed": max(completed),
                    }
                )
    return rows


def write_summary_csv(rows: list[dict[str, int | str | float]]) -> Path:
    path = OUT_STEM.parent / f"{OUT_STEM.name}.summary.csv"
    fields = [
        "workload",
        "spin",
        "k_prefill",
        "median_submit_ns",
        "p90_submit_ns",
        "p99_submit_ns",
        "avg_prefill_submit_tsc_ticks",
        "median_prefill_submit_ns",
        "median_post_submit_completion_ns",
        "min_completed",
        "max_completed",
    ]
    with path.open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        for row in rows:
            writer.writerow({field: row[field] for field in fields})
    return path


def plot(rows: list[dict[str, int | str | float]]) -> tuple[Path, Path]:
    fig, axes = plt.subplots(1, 2, figsize=(15, 4.8))
    colors = plt.get_cmap("viridis")([index / (len(SPINS) - 1) for index in range(len(SPINS))])

    for col, (workload, label) in enumerate(WORKLOADS):
        submit_ax = axes[col]
        workload_rows = [row for row in rows if row["workload"] == workload]

        for spin, color in zip(SPINS, colors, strict=True):
            series = sorted(
                [row for row in workload_rows if row["spin"] == spin],
                key=lambda row: int(row["k_prefill"]),
            )
            xs = [int(row["k_prefill"]) for row in series]
            medians = [float(row["median_submit_ns"]) for row in series]
            submit_ax.plot(xs, medians, marker="o", linewidth=1.8, markersize=3, color=color, label=f"spin={spin}")

        submit_ax.axhline(100, color="0.35", linestyle="--", linewidth=1, label="100 ns knee")
        submit_ax.set_title(f"{label}: measured extra-submit latency")
        submit_ax.set_ylabel("submit latency (ns)")
        submit_ax.set_xticks(K_VALUES)
        submit_ax.tick_params(axis="x", rotation=45)
        submit_ax.grid(True, alpha=0.25)
        submit_ax.set_ylim(bottom=0)

        submit_ax.set_xlabel("K prefilled descriptors")

    axes[0].legend(frameon=False, fontsize=8, ncols=2, loc="upper left")
    fig.suptitle(
        "Experiment 1 submit-occupancy spin sweep on /dev/dsa/wq0.0\n"
        "median measured submit; black-box loop is outside measured submit call",
        fontsize=14,
    )
    fig.tight_layout(rect=(0, 0.02, 1, 0.88))
    png = OUT_STEM.parent / f"{OUT_STEM.name}.png"
    pdf = OUT_STEM.parent / f"{OUT_STEM.name}.pdf"
    fig.savefig(png, dpi=180)
    fig.savefig(pdf)
    return png, pdf


def print_prefill_tick_table(rows: list[dict[str, int | str | float]]) -> None:
    for workload, label in WORKLOADS:
        print(f"\n{label}: average prefill submit ticks")
        print("spin " + " ".join(f"K{k:>6}" for k in K_VALUES))
        for spin in SPINS:
            series = {
                int(row["k_prefill"]): float(row["avg_prefill_submit_tsc_ticks"])
                for row in rows
                if row["workload"] == workload and row["spin"] == spin
            }
            print(f"{spin:>5} " + " ".join(f"{series[k]:7.0f}" for k in K_VALUES))


def main() -> None:
    rows = collect_rows()
    csv_path = write_summary_csv(rows)
    print_prefill_tick_table(rows)
    png, pdf = plot(rows)
    print(png)
    print(pdf)
    print(csv_path)


if __name__ == "__main__":
    main()
