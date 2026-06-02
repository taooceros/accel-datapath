#!/usr/bin/env python3
"""Plot Tokio async DSA memmove against hw-eval shared-WQ batch-size-1 data."""

from __future__ import annotations

import csv
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.ticker import FuncFormatter

ROOT = Path(__file__).resolve().parent
INPUTS = {
    "dedicated": ROOT / "tokio_async_dedicated_vs_hw_eval_shared_batch1.csv",
    "shared_default": ROOT / "tokio_async_vs_hw_eval_shared_batch1_wq0_1.csv",
    "shared_threads1": ROOT / "tokio_async_threads1_vs_hw_eval_shared_batch1_wq0_1.csv",
}
OUTPUT = ROOT / "tokio_async_shared_batch1_comparison.png"

SERIES = (
    ("raw_shared", "hw-eval shared raw", "shared_raw_mops", "#222222", "o"),
    ("async_dedicated", "Tokio async dedicated WQ", "async_dedicated_mops", "#1f77b4", "s"),
    ("async_shared", "Tokio async shared WQ, default workers", "async_shared_mops", "#d62728", "^"),
    ("async_shared_threads1", "Tokio async shared WQ, 1 worker", "async_shared_threads1_mops", "#2ca02c", "D"),
)


def read_csv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="") as f:
        return list(csv.DictReader(f))


def index_rows(rows: list[dict[str, str]]) -> dict[tuple[int, int], dict[str, str]]:
    return {
        (int(row["message_bytes"]), int(row["concurrency"])): row
        for row in rows
    }


def main() -> None:
    dedicated_rows = index_rows(read_csv(INPUTS["dedicated"]))
    shared_default_rows = index_rows(read_csv(INPUTS["shared_default"]))
    shared_threads1_rows = index_rows(read_csv(INPUTS["shared_threads1"]))

    merged: dict[tuple[int, int], dict[str, float]] = {}
    for key, row in dedicated_rows.items():
        merged.setdefault(key, {})["shared_raw_mops"] = float(row["shared_raw_mops"])
        merged[key]["async_dedicated_mops"] = float(row["async_dedicated_mops"])
    for key, row in shared_default_rows.items():
        merged.setdefault(key, {})["async_shared_mops"] = float(row["async_shared_mops"])
    for key, row in shared_threads1_rows.items():
        merged.setdefault(key, {})["async_shared_threads1_mops"] = float(row["async_shared_threads1_mops"])

    sizes = sorted({size for size, _ in merged})
    concurrencies = sorted({concurrency for _, concurrency in merged})

    fig, axes = plt.subplots(
        2,
        len(sizes),
        figsize=(17.5, 7.8),
        sharex=True,
        constrained_layout=True,
    )
    fig.suptitle(
        "Tokio async DSA memmove vs hw-eval shared-WQ batch-size-1 baseline",
        fontsize=15,
        fontweight="bold",
    )

    for col, size in enumerate(sizes):
        ax_throughput = axes[0][col]
        ax_ratio = axes[1][col]
        for _, label, column, color, marker in SERIES:
            y_values = []
            x_values = []
            ratio_values = []
            for concurrency in concurrencies:
                row = merged.get((size, concurrency), {})
                if column not in row:
                    continue
                x_values.append(concurrency)
                y = row[column]
                y_values.append(y)
                baseline = row.get("shared_raw_mops")
                ratio_values.append(y / baseline if baseline else float("nan"))
            ax_throughput.plot(
                x_values,
                y_values,
                label=label,
                color=color,
                marker=marker,
                linewidth=1.8,
                markersize=4.5,
            )
            if column != "shared_raw_mops":
                ax_ratio.plot(
                    x_values,
                    ratio_values,
                    label=label,
                    color=color,
                    marker=marker,
                    linewidth=1.8,
                    markersize=4.5,
                )

        ax_throughput.set_title(f"{size} B")
        ax_throughput.set_xscale("log", base=2)
        ax_throughput.grid(True, which="major", alpha=0.28)
        ax_throughput.set_ylim(bottom=0)
        ax_throughput.yaxis.set_major_formatter(FuncFormatter(lambda value, _: f"{value:g}"))

        ax_ratio.axhline(1.0, color="#777777", linewidth=1.0, linestyle="--")
        ax_ratio.set_xscale("log", base=2)
        ax_ratio.set_xticks(concurrencies)
        ax_ratio.set_xticklabels([str(c) for c in concurrencies])
        ax_ratio.grid(True, which="major", alpha=0.28)
        ax_ratio.set_ylim(bottom=0)
        ax_ratio.yaxis.set_major_formatter(FuncFormatter(lambda value, _: f"{value:.1f}×"))

    axes[0][0].set_ylabel("Throughput (Mops/s)")
    axes[1][0].set_ylabel("Async / raw shared")
    for ax in axes[1]:
        ax.set_xlabel("Concurrency")

    handles, labels = axes[0][0].get_legend_handles_labels()
    fig.legend(handles, labels, loc="lower center", ncol=4, frameon=False, bbox_to_anchor=(0.5, -0.01))
    fig.text(
        0.5,
        0.035,
        "Note: dedicated-WQ Tokio rows are plotted against the shared-WQ raw baseline because that is the target CSV comparison; "
        "shared-WQ default workers are multi-submitter and are not a single-thread overhead measurement.",
        ha="center",
        va="bottom",
        fontsize=9,
        color="#444444",
    )
    fig.savefig(OUTPUT, dpi=220, bbox_inches="tight")
    print(OUTPUT)


if __name__ == "__main__":
    main()
