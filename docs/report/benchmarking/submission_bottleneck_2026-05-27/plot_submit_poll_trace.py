#!/usr/bin/env python3
"""Plot one concrete Experiment 2 per-iteration sample trace.

The benchmark emits `sample_trace` as a list of real iteration traces. This plot
randomly chooses one iteration by default and draws submit latency, per-read
completion-status latency with within-poll read order, and the moving
next-unfinished completion frontier.
"""

from __future__ import annotations

import argparse
import csv
import json
import random
from pathlib import Path
from typing import Any

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt


DEFAULT_INPUT = Path(
    "docs/report/benchmarking/submission_bottleneck_2026-05-27/"
    "marker_trace_per_iteration_offset1_noop.json"
)
DEFAULT_OUTPUT = Path(
    "docs/report/benchmarking/submission_bottleneck_2026-05-27/"
    "submit_poll_trace_per_iteration_offset1_noop.png"
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


def select_trace_row(data: dict[str, Any], row_selector: str | None) -> dict[str, Any]:
    overlap_rows = data.get("submit_marker_overlap")
    if isinstance(overlap_rows, list) and overlap_rows and row_selector in (None, "overlap"):
        return overlap_rows[0]

    mechanism_rows = data.get("submit_marker_mechanism")
    if isinstance(mechanism_rows, list) and mechanism_rows:
        target = row_selector or "baseline/packed-32b"
        for row in mechanism_rows:
            labels = {
                f"{row.get('sub_experiment')}/{row.get('variant')}",
                str(row.get("variant")),
                str(row.get("sub_experiment")),
            }
            if target in labels:
                return row
        raise ValueError(f"input JSON does not contain mechanism row {target!r}")

    raise ValueError("input JSON does not contain trace-capable Experiment 2 rows")


def sample_trace(
    row: dict[str, Any], sample_iteration: int | None = None
) -> tuple[int, list[dict[str, Any]]]:
    traces = row.get("sample_trace")
    if not isinstance(traces, list) or not traces:
        raise ValueError("input row does not contain a non-empty sample_trace")

    first = traces[0]
    if not (isinstance(first, dict) and isinstance(first.get("points"), list)):
        if sample_iteration not in (None, 0):
            raise ValueError("legacy sample_trace has no per-iteration records")
        return 0, traces

    if sample_iteration is None:
        sample = random.choice(traces)
    else:
        sample = next(
            (
                trace
                for trace in traces
                if trace.get("iteration_index") == sample_iteration
            ),
            None,
        )
        if sample is None:
            raise ValueError(f"sample_trace does not contain iteration {sample_iteration}")

    points = sample["points"]
    if not points:
        raise ValueError("selected sample_trace iteration has no points")
    return int(sample["iteration_index"]), points

def write_csv(
    path: Path,
    iteration_index: int,
    trace: list[dict[str, Any]],
    ns_per_tick: float,
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fields = [
        "iteration_index",
        "submit_index",
        "submit_tsc_ticks",
        "submit_ns",
        "submit_start_from_marker_tsc",
        "submit_end_from_marker_tsc",
        "poll_performed",
        "poll_end_from_marker_tsc",
        "poll_window_tsc_ticks",
        "poll_count",
        "first_polled_request_index",
        "last_polled_request_index",
        "visible_prefix_len",
        "polled_statuses",
        "polled_status",
        "marker_status",
        "polled_request_indices",
        "poll_latency_tsc_ticks",
        "poll_latency_ns",
    ]
    with path.open("w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=fields)
        writer.writeheader()
        for point in trace:
            row = {field: point.get(field, "") for field in fields}
            row["iteration_index"] = iteration_index
            row["submit_ns"] = format_ns(point.get("submit_tsc_ticks"), ns_per_tick)
            row["polled_request_indices"] = ";".join(
                str(value) for value in point.get("polled_request_indices", [])
            )
            latencies = point.get("poll_latency_tsc_ticks", [])
            row["poll_latency_tsc_ticks"] = ";".join(str(value) for value in latencies)
            row["poll_latency_ns"] = ";".join(
                format_ns(value, ns_per_tick) for value in latencies
            )
            row["polled_statuses"] = ";".join(
                str(value) for value in point.get("polled_statuses", [])
            )
            writer.writerow(row)


def format_ns(value: Any, ns_per_tick: float) -> str:
    if value in (None, ""):
        return ""
    return f"{float(value) * ns_per_tick:.1f}"


def point_statuses(point: dict[str, Any]) -> list[int]:
    statuses = point.get("polled_statuses")
    if isinstance(statuses, list) and statuses:
        return [int(status) for status in statuses]
    return [int(point.get("polled_status", point.get("marker_status", 0)) or 0)]


def point_request_indices(point: dict[str, Any], read_count: int) -> list[int]:
    indices = point.get("polled_request_indices") or []
    if indices:
        return [int(value) for value in indices]

    first = point.get("first_polled_request_index")
    if first is None:
        return []
    return list(range(int(first), int(first) + read_count))


def visible_submit_indices(trace: list[dict[str, Any]]) -> list[int]:
    return [
        int(point["submit_index"])
        for point in trace
        if point.get("poll_performed") and any(status != 0 for status in point_statuses(point))
    ]


def plot(
    row: dict[str, Any],
    iteration_index: int,
    trace: list[dict[str, Any]],
    output: Path,
    title: str,
    ns_per_tick: float,
) -> None:
    x = [point["submit_index"] for point in trace]
    submit = [point["submit_tsc_ticks"] * ns_per_tick for point in trace]

    none_x: list[float] = []
    none_y: list[float] = []
    visible_x: list[float] = []
    visible_y: list[float] = []
    visible_order_labels: list[int] = []
    frontier_x: list[int] = []
    probed_request: list[int] = []
    visible_prefix: list[int] = []
    for point in trace:
        if not point.get("poll_performed"):
            continue
        submit_index = int(point["submit_index"])
        latencies = point.get("poll_latency_tsc_ticks", [])
        statuses = point_statuses(point)
        if not latencies:
            continue

        target = point.get("last_polled_request_index")
        if target is not None:
            frontier_x.append(submit_index)
            probed_request.append(int(target))
            visible_prefix.append(int(point.get("visible_prefix_len") or 0))

        latencies_ns = [latency * ns_per_tick for latency in latencies]
        for read_index, (latency_ns, status) in enumerate(zip(latencies_ns, statuses)):
            # Spread reads from the same polling event slightly to the right of
            # the submit index. Left-to-right position is the read order inside
            # that event: first read at i, second at i+0.08, etc.
            read_x = submit_index + 0.08 * read_index
            if status == 0:
                none_x.append(read_x)
                none_y.append(latency_ns)
            else:
                visible_x.append(read_x)
                visible_y.append(latency_ns)
                visible_order_labels.append(read_index + 1)
        if len(statuses) < len(latencies):
            for read_index, latency_ns in enumerate(latencies_ns[len(statuses) :]):
                none_x.append(submit_index + 0.08 * (len(statuses) + read_index))
                none_y.append(latency_ns)

    fig, axes = plt.subplots(3, 1, figsize=(13.5, 9.6), sharex=True, constrained_layout=True)
    fig.suptitle(title)

    axes[0].plot(x, submit, linewidth=1.8, color="#1f77b4")
    axes[0].set_ylabel("submit latency\n(ns)")

    if none_x:
        axes[1].scatter(none_x, none_y, s=18, color="#9ca3af", label="NONE")
    if visible_x:
        axes[1].scatter(visible_x, visible_y, s=20, color="#d62728", label="visible")
        for x_value, y_value, label in zip(visible_x, visible_y, visible_order_labels):
            axes[1].annotate(
                str(label),
                (x_value, y_value),
                textcoords="offset points",
                xytext=(0, 4),
                ha="center",
                fontsize=7,
                color="#7f1d1d",
            )
    axes[1].set_ylabel("status-read latency\n(ns)")
    axes[1].text(
        0.01,
        0.94,
        "within each submit index: left→right = poll read order",
        transform=axes[1].transAxes,
        fontsize=9,
        color="#4b5563",
        va="top",
    )
    axes[1].legend(loc="upper right")

    if frontier_x:
        axes[2].step(
            frontier_x,
            probed_request,
            where="post",
            linewidth=1.6,
            color="#1f77b4",
            label="last probed request",
        )
        axes[2].step(
            frontier_x,
            visible_prefix,
            where="post",
            linewidth=1.4,
            color="#2ca02c",
            label="visible prefix after read",
        )
    axes[2].set_ylabel("request frontier")
    axes[2].set_xlabel("submit index")
    axes[2].legend(loc="upper left")

    poll_offset = row["marker_poll_offset"]
    visible_indices = visible_submit_indices(trace)
    for ax in axes:
        ax.axvline(
            poll_offset,
            color="#16a34a",
            linewidth=1.0,
            linestyle="--",
            alpha=0.65,
            label="poll offset" if ax is axes[0] else None,
        )
        for offset, visible_index in enumerate(visible_indices):
            ax.axvline(
                visible_index,
                color="#7c3aed",
                linewidth=0.45,
                alpha=0.16,
                zorder=1,
                label="visible completion read"
                if ax is axes[0] and offset == 0
                else None,
            )
        ax.grid(True, color="#e5e7eb", linewidth=0.8)
        ax.set_xlim(min(x), max(x))
        ax.set_ylim(bottom=0)
    axes[0].legend(loc="upper left")

    visible_text = (
        f"{len(visible_indices)} submit indices with visible reads; "
        f"frontier {visible_prefix[0]}→{visible_prefix[-1]}"
        if visible_indices and probed_request and visible_prefix
        else "completion frontier not visible in selected trace"
    )
    subtitle = (
        f"N={row['n']}, poll offset={poll_offset}, "
        f"sample iteration={iteration_index}; {visible_text}; "
        f"1 TSC tick = {ns_per_tick:.3f} ns"
    )
    axes[0].text(
        0.0,
        1.02,
        subtitle,
        transform=axes[0].transAxes,
        fontsize=10,
        color="#4b5563",
        va="bottom",
    )

    output.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(output, dpi=180)
    plt.close(fig)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Plot one Experiment 2 per-iteration trace")
    parser.add_argument("input", nargs="?", type=Path, default=DEFAULT_INPUT)
    parser.add_argument("--output", "-o", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--csv", type=Path, default=None)
    parser.add_argument("--title", default="Experiment 2 rerun: random per-iteration trace")
    parser.add_argument(
        "--sample-iteration",
        type=int,
        default=None,
        help="Iteration index to draw; defaults to a random recorded iteration",
    )
    parser.add_argument(
        "--row",
        default=None,
        help=(
            "Row selector for mechanism JSON, e.g. baseline/packed-32b, "
            "prefetch/prefetch-1-lines, or layout/padded-64b. "
            "Defaults to overlap rows when present, otherwise baseline/packed-32b."
        ),
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    data = load_results(args.input)
    tsc_freq_hz = data.get("metadata", {}).get("tsc_freq_hz")
    if not tsc_freq_hz:
        raise ValueError("input JSON metadata does not contain tsc_freq_hz")
    ns_per_tick = 1_000_000_000.0 / float(tsc_freq_hz)
    row = select_trace_row(data, args.row)
    iteration_index, trace = sample_trace(row, args.sample_iteration)
    csv_path = args.csv if args.csv is not None else args.output.with_suffix(".csv")
    write_csv(csv_path, iteration_index, trace, ns_per_tick)
    plot(row, iteration_index, trace, args.output, args.title, ns_per_tick)
    print(f"selected iteration {iteration_index}")
    visible_indices = visible_submit_indices(trace)
    if visible_indices:
        print(
            "visible completion submits "
            f"{visible_indices[0]}..{visible_indices[-1]} ({len(visible_indices)} points)"
        )
    else:
        print("completion not visible in selected trace")
    print(f"wrote {args.output}")
    print(f"wrote {csv_path}")


if __name__ == "__main__":
    main()
