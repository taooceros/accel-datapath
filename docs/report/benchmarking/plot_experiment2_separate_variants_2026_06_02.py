#!/usr/bin/env python3
"""Plot Experiment 2 separate variant summaries from CSV artifacts."""

from __future__ import annotations

import argparse
import csv
from pathlib import Path
from typing import Iterable

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt

DEFAULT_INPUT_DIR = Path(
    "docs/report/benchmarking/submission_bottleneck_2026-06-02/experiment2_separate_variants"
)


def read_rows(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as f:
        return list(csv.DictReader(f))


def number(row: dict[str, str], key: str) -> float:
    value = row.get(key, "")
    if value == "" or value is None:
        return float("nan")
    return float(value)


def int_label(row: dict[str, str]) -> int:
    return int(float(row["label"]))


def ordered(rows: Iterable[dict[str, str]], order: list[str]) -> list[dict[str, str]]:
    by_label = {row["label"]: row for row in rows}
    return [by_label[label] for label in order if label in by_label]


def draw_lines(ax, xs, series, ylabel: str, title: str, xlabel: str) -> None:
    for label, ys, style in series:
        ax.plot(xs, ys, marker="o", linewidth=1.9, label=label, **style)
    ax.set_title(title)
    ax.set_xlabel(xlabel)
    ax.set_ylabel(ylabel)
    ax.grid(True, color="#e5e7eb", linewidth=0.8)
    ax.legend(frameon=True)


def plot_combined(overlap: list[dict[str, str]], mechanism: list[dict[str, str]], output: Path) -> None:
    fig, axes = plt.subplots(3, 2, figsize=(14.5, 13.5), constrained_layout=True)
    fig.suptitle("Experiment 2 separate variant runs, 2026-06-02", fontsize=15)

    poll = sorted(
        [row for row in overlap if row["family"] == "poll_offset"], key=int_label
    )
    xs = [int_label(row) for row in poll]
    draw_lines(
        axes[0, 0],
        xs,
        [
            (
                "marker visible median",
                [number(row, "marker_visible_ns_median") for row in poll],
                {"color": "#d62728"},
            ),
            (
                "submit tail median",
                [number(row, "submit_tail_ns_median") for row in poll],
                {"color": "#1f77b4"},
            ),
        ],
        "latency (ns)",
        "Poll-offset sweep: later first poll delays observation",
        "first poll submit index",
    )

    burst = sorted(
        [row for row in overlap if row["family"] == "burst_depth"], key=int_label
    )
    xs = [int_label(row) for row in burst]
    draw_lines(
        axes[0, 1],
        xs,
        [
            (
                "marker visible median",
                [number(row, "marker_visible_ns_median") for row in burst],
                {"color": "#d62728"},
            ),
            (
                "submit tail median",
                [number(row, "submit_tail_ns_median") for row in burst],
                {"color": "#1f77b4"},
            ),
        ],
        "latency (ns)",
        "Burst-depth sweep: first marker visible across the knee",
        "burst N",
    )

    payload = ordered(
        [row for row in overlap if row["family"] == "payload"],
        ["noop", "memmove64", "memmove4k", "memmove1m"],
    )
    payload_labels = [
        {
            "noop": "noop",
            "memmove64": "64B",
            "memmove4k": "4KiB",
            "memmove1m": "1MiB",
        }[row["label"]]
        for row in payload
    ]
    ax = axes[1, 0]
    ax.bar(payload_labels, [number(row, "marker_visible_ns_median") for row in payload], color="#d62728")
    ax.set_yscale("log")
    ax.set_title("Payload sweep: 1 MiB changes the regime")
    ax.set_ylabel("marker visible median (ns, log)")
    ax.grid(True, axis="y", color="#e5e7eb", linewidth=0.8)
    ax2 = ax.twinx()
    ax2.plot(
        payload_labels,
        [number(row, "completed_median") for row in payload],
        marker="o",
        color="#2ca02c",
        linewidth=1.8,
        label="completed median",
    )
    ax2.set_ylabel("completed median")
    ax2.set_ylim(0, 170)

    batch_rows = []
    for row in mechanism:
        if row["family"] != "poll_submit_batch":
            continue
        if row["label"] == "1" and row["sub_experiment"] == "baseline":
            batch_rows.append(row)
        elif row["sub_experiment"] == "poll-submit-batch":
            batch_rows.append(row)
    batch_rows.sort(key=int_label)
    xs = [int_label(row) for row in batch_rows]
    draw_lines(
        axes[1, 1],
        xs,
        [
            (
                "visible poll median",
                [number(row, "visible_poll_ns_median") for row in batch_rows],
                {"color": "#d62728"},
            ),
            (
                "visible poll p99",
                [number(row, "visible_poll_ns_p99") for row in batch_rows],
                {"color": "#ff7f0e", "linestyle": "--"},
            ),
            (
                "NONE poll median",
                [number(row, "none_poll_ns_median") for row in batch_rows],
                {"color": "#6b7280"},
            ),
        ],
        "status-read latency (ns)",
        "Poll-submit interval: less frequent polling reduces median visible-read cost",
        "submits between polls",
    )
    axes[1, 1].set_xticks(xs)

    marker = ordered(
        [row for row in overlap if row["family"] == "marker_position"],
        ["first", "half", "last"],
    )
    ax = axes[2, 0]
    marker_labels = [row["label"] for row in marker]
    ax.bar(marker_labels, [number(row, "marker_visible_ns_median") for row in marker], color="#9467bd")
    ax.set_title("Marker-position sweep")
    ax.set_ylabel("marker visible median (ns)")
    ax.grid(True, axis="y", color="#e5e7eb", linewidth=0.8)
    ax2 = ax.twinx()
    ax2.plot(
        marker_labels,
        [100.0 * number(row, "marker_observed_before_final_submit_fraction") for row in marker],
        marker="o",
        color="#2ca02c",
        linewidth=1.8,
    )
    ax2.set_ylabel("observed before final submit (%)")
    ax2.set_ylim(-5, 105)

    ax = axes[2, 1]
    ax.bar(
        payload_labels,
        [number(row, "completed_median") for row in payload],
        color="#2ca02c",
        label="completed",
    )
    ax.bar(
        payload_labels,
        [number(row, "missing_median") for row in payload],
        bottom=[number(row, "completed_median") for row in payload],
        color="#d62728",
        label="missing",
    )
    ax.set_title("Payload completion visibility at overlap measurement point")
    ax.set_ylabel("median requests")
    ax.set_ylim(0, 170)
    ax.grid(True, axis="y", color="#e5e7eb", linewidth=0.8)
    ax.legend()

    output.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(output, dpi=180)
    plt.close(fig)


def plot_family_panels(overlap: list[dict[str, str]], mechanism: list[dict[str, str]], output_dir: Path) -> None:
    combined_path = output_dir / "experiment2_separate_variants_summary.png"
    plot_combined(overlap, mechanism, combined_path)

    # Also save a PDF companion for vector-friendly report use.
    pdf_path = combined_path.with_suffix(".pdf")
    plot_combined(overlap, mechanism, pdf_path)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input-dir", type=Path, default=DEFAULT_INPUT_DIR)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    overlap = read_rows(args.input_dir / "summary_overlap.csv")
    mechanism = read_rows(args.input_dir / "summary_mechanism.csv")
    plot_family_panels(overlap, mechanism, args.input_dir)
    print(args.input_dir / "experiment2_separate_variants_summary.png")
    print(args.input_dir / "experiment2_separate_variants_summary.pdf")


if __name__ == "__main__":
    main()
