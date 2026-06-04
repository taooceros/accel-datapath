#!/usr/bin/env python3
"""Plot one representative trace for each separately-run Experiment 2 variant.

The case map below names the variable under test for each family. Overlap rows use
`submit_marker_overlap`; the poll-submit-batch family uses the mechanism row whose
`poll_submit_batch_n` is the variable under test.
"""

from __future__ import annotations

import argparse
import csv
import importlib.util
import math
from pathlib import Path
from types import ModuleType
from typing import Any

import matplotlib

matplotlib.use("Agg")
import matplotlib.image as mpimg
import matplotlib.pyplot as plt

DEFAULT_INPUT_DIR = Path(
    "docs/report/benchmarking/submission_bottleneck_2026-06-02/experiment2_separate_variants"
)
TRACE_PLOTTER = Path(
    "docs/report/benchmarking/submission_bottleneck_2026-05-27/plot_submit_poll_trace.py"
)


def load_trace_plotter() -> ModuleType:
    spec = importlib.util.spec_from_file_location("experiment2_trace_plotter", TRACE_PLOTTER)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"could not import {TRACE_PLOTTER}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def cases() -> list[dict[str, str]]:
    result: list[dict[str, str]] = []
    for offset in [1, 8, 16, 32, 64, 96, 112]:
        result.append(
            {
                "family": "poll_offset",
                "case": f"poll_offset_{offset:03d}",
                "selector": "overlap",
                "variable": "marker_poll_offset",
                "value": str(offset),
                "fixed": "N=160, marker=first, op=noop",
                "title": f"Experiment 2 trace: poll offset {offset} (N=160, marker=first, noop)",
            }
        )
    for n in [96, 112, 114, 116, 120, 128, 160]:
        result.append(
            {
                "family": "burst_depth",
                "case": f"burst_n{n:03d}",
                "selector": "overlap",
                "variable": "marker_bursts / N",
                "value": str(n),
                "fixed": "poll_offset=1, marker=first, op=noop",
                "title": f"Experiment 2 trace: burst N={n} (poll offset=1, marker=first, noop)",
            }
        )
    for case, label in [
        ("payload_noop", "noop"),
        ("payload_memmove64", "64B memmove"),
        ("payload_memmove4k", "4KiB memmove"),
        ("payload_memmove1m", "1MiB memmove"),
    ]:
        result.append(
            {
                "family": "payload",
                "case": case,
                "selector": "overlap",
                "variable": "dsa operation / payload bytes",
                "value": label,
                "fixed": "N=160, poll_offset=1, marker=first",
                "title": f"Experiment 2 trace: payload {label} (N=160, poll offset=1, marker=first)",
            }
        )
    for batch_n in [1, 2, 4, 8, 16]:
        selector = "baseline/packed-32b" if batch_n == 1 else "poll-submit-batch/configured"
        result.append(
            {
                "family": "poll_submit_batch",
                "case": f"poll_submit_batch_{batch_n:02d}",
                "selector": selector,
                "variable": "poll_submit_batch_n",
                "value": str(batch_n),
                "fixed": "N=160, poll_offset=1, op=noop; mechanism row",
                "title": f"Experiment 2 mechanism trace: poll-submit interval {batch_n} (N=160, noop)",
            }
        )
    for position in ["first", "half", "last"]:
        result.append(
            {
                "family": "marker_position",
                "case": f"marker_position_{position}",
                "selector": "overlap",
                "variable": "marker_positions",
                "value": position,
                "fixed": "N=160, poll_offset=1, op=noop",
                "title": f"Experiment 2 trace: marker position {position} (N=160, poll offset=1, noop)",
            }
        )
    return result


def trace_records(row: dict[str, Any]) -> list[dict[str, Any]]:
    traces = row.get("sample_trace")
    if not isinstance(traces, list) or not traces:
        raise ValueError("row has no sample_trace records")
    first = traces[0]
    if isinstance(first, dict) and isinstance(first.get("points"), list):
        return traces
    return [{"iteration_index": 0, "points": traces}]


def choose_representative_iteration(trace_plotter: ModuleType, row: dict[str, Any]) -> tuple[int, list[dict[str, Any]], int]:
    records = trace_records(row)
    scored: list[tuple[int, int, list[dict[str, Any]]]] = []
    for record in records:
        points = record.get("points")
        if not points:
            continue
        iteration_index = int(record["iteration_index"])
        visible_count = len(trace_plotter.visible_submit_indices(points))
        scored.append((visible_count, iteration_index, points))
    if not scored:
        raise ValueError("sample_trace contains no non-empty trace records")

    visible_counts = sorted(item[0] for item in scored)
    median_visible = visible_counts[len(visible_counts) // 2]
    middle_iteration = scored[len(scored) // 2][1]
    visible_count, iteration_index, points = min(
        scored,
        key=lambda item: (
            abs(item[0] - median_visible),
            abs(item[1] - middle_iteration),
            item[1],
        ),
    )
    return iteration_index, points, visible_count


def safe_name(value: str) -> str:
    return value.replace("/", "_").replace(" ", "_").replace("=", "")


def plot_case(
    trace_plotter: ModuleType,
    input_dir: Path,
    output_root: Path,
    case: dict[str, str],
) -> dict[str, str | int]:
    json_path = input_dir / f"{case['case']}.json"
    data = trace_plotter.load_results(json_path)
    tsc_freq_hz = data.get("metadata", {}).get("tsc_freq_hz")
    if not tsc_freq_hz:
        raise ValueError(f"{json_path} metadata does not contain tsc_freq_hz")
    ns_per_tick = 1_000_000_000.0 / float(tsc_freq_hz)
    row = trace_plotter.select_trace_row(data, case["selector"])
    iteration_index, trace, visible_count = choose_representative_iteration(trace_plotter, row)

    family_dir = output_root / case["family"]
    family_dir.mkdir(parents=True, exist_ok=True)
    stem = safe_name(case["case"])
    png_path = family_dir / f"{stem}.png"
    csv_path = family_dir / f"{stem}.csv"
    trace_plotter.write_csv(csv_path, iteration_index, trace, ns_per_tick)
    trace_plotter.plot(row, iteration_index, trace, png_path, case["title"], ns_per_tick)

    visible_indices = trace_plotter.visible_submit_indices(trace)
    return {
        "family": case["family"],
        "case": case["case"],
        "variable": case["variable"],
        "value": case["value"],
        "fixed": case["fixed"],
        "selector": case["selector"],
        "sample_iteration": iteration_index,
        "visible_submit_count": visible_count,
        "first_visible_submit_index": visible_indices[0] if visible_indices else "",
        "last_visible_submit_index": visible_indices[-1] if visible_indices else "",
        "plot": str(png_path),
        "csv": str(csv_path),
    }


def write_manifest(path: Path, rows: list[dict[str, str | int]]) -> None:
    if not rows:
        return
    with path.open("w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        writer.writeheader()
        writer.writerows(rows)


def make_contact_sheet(plot_paths: list[Path], output: Path) -> None:
    columns = 4
    rows = math.ceil(len(plot_paths) / columns)
    fig, axes = plt.subplots(rows, columns, figsize=(columns * 5.0, rows * 3.8), constrained_layout=True)
    axes_list = list(axes.flat) if hasattr(axes, "flat") else [axes]
    for ax, path in zip(axes_list, plot_paths):
        image = mpimg.imread(path)
        ax.imshow(image)
        ax.set_title(path.parent.name + "/" + path.stem, fontsize=8)
        ax.axis("off")
    for ax in axes_list[len(plot_paths) :]:
        ax.axis("off")
    output.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(output, dpi=120)
    plt.close(fig)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input-dir", type=Path, default=DEFAULT_INPUT_DIR)
    parser.add_argument("--output-dir", type=Path, default=None)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    output_root = args.output_dir or args.input_dir / "trace_plots"
    trace_plotter = load_trace_plotter()
    manifest_rows: list[dict[str, str | int]] = []
    for case in cases():
        row = plot_case(trace_plotter, args.input_dir, output_root, case)
        manifest_rows.append(row)
        print(f"wrote {row['plot']} sample_iteration={row['sample_iteration']}")
    manifest = output_root / "manifest.csv"
    write_manifest(manifest, manifest_rows)
    make_contact_sheet([Path(str(row["plot"])) for row in manifest_rows], output_root / "contact_sheet.png")
    print(f"wrote {manifest}")
    print(f"wrote {output_root / 'contact_sheet.png'}")


if __name__ == "__main__":
    main()
