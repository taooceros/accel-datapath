#!/usr/bin/env python3
"""
Interactive visualization for hw-eval benchmark results using Plotly.
Generates self-contained HTML dashboards from JSON output.

Usage:
    hw-eval --json > results.json
    python3 visualize_interactive.py results.json [-o graphs/]
"""

import argparse
import json
import re
from pathlib import Path


def human_size(n):
    if n >= 1048576:
        return f"{n // 1048576}MB"
    if n >= 1024:
        return f"{n // 1024}KB"
    return f"{n}B"


COLORS = ["#1f77b4", "#ff7f0e", "#2ca02c", "#d62728", "#9467bd", "#8c564b", "#e377c2"]
LINE_STYLES = ["solid", "dash", "dot", "dashdot", "longdash", "longdashdot"]
MARKERS = ["circle", "square", "diamond", "triangle-up", "cross", "x", "star"]

# Canonical strategy order and display names
STRATEGY_INFO = {
    "sliding_window": {"label": "Sliding Window", "color": "#1f77b4", "dash": "solid"},
    "burst": {"label": "Burst", "color": "#ff7f0e", "dash": "dash"},
    "pipelined_batch": {"label": "Pipelined Batch", "color": "#2ca02c", "dash": "dot"},
    "burst_batch": {"label": "Burst-Batch", "color": "#d62728", "dash": "dashdot"},
}


def load_results(path):
    with open(path) as f:
        return json.load(f)


def queue_type_label(metadata):
    return "dedicated" if metadata.get("wq_dedicated") else "shared"


def load_result_sets(paths):
    result_sets = []
    for path_str in paths:
        path = Path(path_str)
        data = load_results(path)
        metadata = data["metadata"]
        result_sets.append(
            {
                "path": path,
                "data": data,
                "metadata": metadata,
                "queue_type": queue_type_label(metadata),
            }
        )
    return result_sets


def build_meta_text(result_sets):
    queue_types = ", ".join(sorted({rs["queue_type"] for rs in result_sets}))
    devices = ", ".join(sorted({rs["metadata"]["device"] for rs in result_sets}))
    iterations = ", ".join(
        str(v) for v in sorted({rs["metadata"]["iterations"] for rs in result_sets})
    )
    pinned_cores = ", ".join(
        str(v) for v in sorted({rs["metadata"]["pinned_core"] for rs in result_sets})
    )
    tsc_ghz = ", ".join(
        f"{v / 1e9:.3f}"
        for v in sorted({rs["metadata"]["tsc_freq_hz"] for rs in result_sets})
    )
    cold_cache = ", ".join(
        str(v) for v in sorted({rs["metadata"]["cold_cache"] for rs in result_sets})
    )
    return (
        f"Queue Type: {queue_types}"
        f" | Device: {devices}"
        f" | Core: {pinned_cores}"
        f" | TSC: {tsc_ghz} GHz"
        f" | Iterations: {iterations}"
        f" | Cold cache: {cold_cache}"
    )


def classify_benchmark(name):
    """Classify a benchmark name into (strategy, batch_size).

    Returns (strategy_key, batch_n) where batch_n=0 for non-batch strategies.
    """
    m = re.match(r"pipelined_batch_b(\d+)", name)
    if m:
        return "pipelined_batch", int(m.group(1))
    m = re.match(r"burst_batch_b(\d+)", name)
    if m:
        return "burst_batch", int(m.group(1))
    if name == "memmove":
        return "sliding_window", 0
    if name == "burst_memmove":
        return "burst", 0
    return None, None


def create_dashboard(result_sets, output_path: Path):
    """Create interactive HTML dashboard with checkbox filters."""

    # Parse throughput data into records
    records = []
    for result_set in result_sets:
        queue_type = result_set["queue_type"]
        for r in result_set["data"]["throughput"]:
            strategy, batch_n = classify_benchmark(r["benchmark"])
            if strategy is None:
                continue
            records.append(
                {
                    "queue_type": queue_type,
                    "strategy": strategy,
                    "batch_size": batch_n,
                    "concurrency": r["concurrency"],
                    "size": r["size"],
                    "ops_per_sec": r["ops_per_sec"],
                    "bandwidth_mb_s": r["bandwidth_mb_s"],
                }
            )

    # Parse latency data
    latency_records = []
    for result_set in result_sets:
        queue_type = result_set["queue_type"]
        for r in result_set["data"]["latency"]:
            if r["size"] is not None and r["benchmark"] in (
                "memmove",
                "crc_gen",
                "copy_crc",
                "sw_memcpy",
                "sw_crc32c",
            ):
                latency_records.append(
                    {
                        "queue_type": queue_type,
                        "benchmark": r["benchmark"],
                        "size": r["size"],
                        "median_ns": r["ns"]["median"],
                        "p99_ns": r["ns"].get("p99", r["ns"]["median"]),
                    }
                )

    # Extract unique dimension values
    queue_types = sorted(set(r["queue_type"] for r in records))
    strategies = sorted(
        set(r["strategy"] for r in records),
        key=lambda s: list(STRATEGY_INFO.keys()).index(s) if s in STRATEGY_INFO else 99,
    )
    batch_sizes = sorted(set(r["batch_size"] for r in records))
    concurrencies = sorted(set(r["concurrency"] for r in records))
    sizes = sorted(set(r["size"] for r in records))

    # Build Plotly traces for throughput subplots (ops/sec and bandwidth)
    trace_metadata = []
    traces_json = []

    for qi, queue_type in enumerate(queue_types):
        for si, strategy in enumerate(strategies):
            info = STRATEGY_INFO.get(
                strategy,
                {"label": strategy, "color": COLORS[si % len(COLORS)], "dash": "solid"},
            )
            for bi, bs in enumerate(batch_sizes):
                for ci, conc in enumerate(concurrencies):
                    # Filter records for this combination
                    pts = [
                        r
                        for r in records
                        if r["queue_type"] == queue_type
                        and r["strategy"] == strategy
                        and r["batch_size"] == bs
                        and r["concurrency"] == conc
                    ]
                    if not pts:
                        continue

                    pts.sort(key=lambda r: r["size"])
                    xs = [p["size"] for p in pts]
                    ys_ops = [p["ops_per_sec"] / 1e6 for p in pts]
                    ys_bw = [p["bandwidth_mb_s"] / 1e3 for p in pts]  # GB/s

                    bs_label = f"b={bs}" if bs > 0 else "single"
                    trace_label = (
                        f"{queue_type} | {info['label']} ({bs_label}, c={conc})"
                    )

                    marker_sym = MARKERS[(ci + qi) % len(MARKERS)]
                    color = info["color"]
                    dash = info["dash"]

                    # Default: show first queue type and first strategy tuple
                    visible = (
                        queue_type == queue_types[0]
                        and strategy == strategies[0]
                        and bs == batch_sizes[0]
                        and conc == concurrencies[0]
                    )

                    meta = {
                        "queue_type": queue_type,
                        "strategy": strategy,
                        "batch_size": bs,
                        "concurrency": conc,
                        "label": trace_label,
                    }

                    # Message rate subplot
                    traces_json.append(
                        {
                            "x": xs,
                            "y": ys_ops,
                            "mode": "lines+markers",
                            "name": trace_label,
                            "line": {"color": color, "dash": dash, "width": 2},
                            "marker": {"size": 7, "symbol": marker_sym},
                            "visible": visible,
                            "showlegend": visible,
                            "legendgroup": trace_label,
                            "hovertemplate": f"{trace_label}<br>%{{x}}B: %{{y:.2f}} Mops/s<extra></extra>",
                            "xaxis": "x",
                            "yaxis": "y",
                        }
                    )
                    trace_metadata.append({**meta, "subplot": "msg_rate"})

                    # Bandwidth subplot
                    traces_json.append(
                        {
                            "x": xs,
                            "y": ys_bw,
                            "mode": "lines+markers",
                            "name": trace_label,
                            "line": {"color": color, "dash": dash, "width": 2},
                            "marker": {"size": 7, "symbol": marker_sym},
                            "visible": visible,
                            "showlegend": False,
                            "legendgroup": trace_label,
                            "hovertemplate": f"{trace_label}<br>%{{x}}B: %{{y:.2f}} GB/s<extra></extra>",
                            "xaxis": "x2",
                            "yaxis": "y2",
                        }
                    )
                    trace_metadata.append({**meta, "subplot": "bandwidth"})

    # Add latency traces
    latency_benchmarks = sorted(set(r["benchmark"] for r in latency_records))
    lat_colors = {
        "memmove": "#1f77b4",
        "crc_gen": "#ff7f0e",
        "copy_crc": "#2ca02c",
        "sw_memcpy": "#d62728",
        "sw_crc32c": "#9467bd",
    }
    lat_markers = {
        "memmove": "circle",
        "crc_gen": "square",
        "copy_crc": "diamond",
        "sw_memcpy": "triangle-up",
        "sw_crc32c": "cross",
    }

    latency_queue_types = sorted(set(r["queue_type"] for r in latency_records))
    for queue_type in latency_queue_types:
        for bench in latency_benchmarks:
            pts = sorted(
                [
                    r
                    for r in latency_records
                    if r["queue_type"] == queue_type and r["benchmark"] == bench
                ],
                key=lambda r: r["size"],
            )
            if not pts:
                continue
            xs = [p["size"] for p in pts]
            ys_med = [p["median_ns"] for p in pts]
            ys_p99 = [p["p99_ns"] for p in pts]
            color = lat_colors.get(bench, "#333")
            marker = lat_markers.get(bench, "circle")
            trace_label = f"{queue_type} | {bench}"

            # Median latency
            traces_json.append(
                {
                    "x": xs,
                    "y": ys_med,
                    "mode": "lines+markers",
                    "name": trace_label,
                    "line": {"color": color, "width": 2},
                    "marker": {"size": 7, "symbol": marker},
                    "visible": True,
                    "showlegend": False,
                    "legendgroup": f"lat_{trace_label}",
                    "hovertemplate": f"{trace_label}<br>%{{x}}B: %{{y:.0f}} ns<extra></extra>",
                    "xaxis": "x3",
                    "yaxis": "y3",
                }
            )
            trace_metadata.append(
                {
                    "subplot": "latency_median",
                    "queue_type": queue_type,
                    "latency_bench": bench,
                }
            )

            # P99 latency
            traces_json.append(
                {
                    "x": xs,
                    "y": ys_p99,
                    "mode": "lines+markers",
                    "name": f"{trace_label} p99",
                    "line": {"color": color, "dash": "dash", "width": 2},
                    "marker": {"size": 7, "symbol": marker},
                    "visible": True,
                    "showlegend": False,
                    "legendgroup": f"lat_{trace_label}",
                    "hovertemplate": f"{trace_label} p99<br>%{{x}}B: %{{y:.0f}} ns<extra></extra>",
                    "xaxis": "x4",
                    "yaxis": "y4",
                }
            )
            trace_metadata.append(
                {
                    "subplot": "latency_p99",
                    "queue_type": queue_type,
                    "latency_bench": bench,
                }
            )

    # Layout with 4 subplots (2x2)
    layout = {
        "title": {
            "text": "hw-eval: DSA Hardware Benchmark Results",
            "x": 0.5,
            "xanchor": "center",
        },
        "height": 750,
        "hovermode": "closest",
        "margin": {"t": 80, "b": 80, "r": 220, "l": 80},
        "legend": {
            "orientation": "v",
            "yanchor": "top",
            "y": 1,
            "xanchor": "left",
            "x": 1.02,
            "font": {"size": 10},
            "bgcolor": "rgba(255,255,255,0.8)",
        },
        "grid": {
            "rows": 2,
            "columns": 2,
            "pattern": "independent",
            "roworder": "top to bottom",
        },
        "xaxis": {
            "type": "log",
            "title": "Message Size (bytes)",
            "domain": [0, 0.45],
            "anchor": "y",
        },
        "yaxis": {
            "title": "Message Rate (Mops/s)",
            "rangemode": "tozero",
            "domain": [0.55, 1],
            "anchor": "x",
        },
        "xaxis2": {
            "type": "log",
            "title": "Message Size (bytes)",
            "domain": [0.55, 1],
            "anchor": "y2",
        },
        "yaxis2": {
            "title": "Bandwidth (GB/s)",
            "rangemode": "tozero",
            "domain": [0.55, 1],
            "anchor": "x2",
        },
        "xaxis3": {
            "type": "log",
            "title": "Message Size (bytes)",
            "domain": [0, 0.45],
            "anchor": "y3",
        },
        "yaxis3": {
            "title": "Median Latency (ns)",
            "type": "log",
            "domain": [0, 0.4],
            "anchor": "x3",
        },
        "xaxis4": {
            "type": "log",
            "title": "Message Size (bytes)",
            "domain": [0.55, 1],
            "anchor": "y4",
        },
        "yaxis4": {
            "title": "P99 Latency (ns)",
            "type": "log",
            "domain": [0, 0.4],
            "anchor": "x4",
        },
        "annotations": [
            {
                "text": "<b>Message Rate vs Size</b>",
                "x": 0.225,
                "y": 1.02,
                "xref": "paper",
                "yref": "paper",
                "showarrow": False,
                "font": {"size": 14},
            },
            {
                "text": "<b>Bandwidth vs Size</b>",
                "x": 0.775,
                "y": 1.02,
                "xref": "paper",
                "yref": "paper",
                "showarrow": False,
                "font": {"size": 14},
            },
            {
                "text": "<b>Median Latency vs Size</b>",
                "x": 0.225,
                "y": 0.43,
                "xref": "paper",
                "yref": "paper",
                "showarrow": False,
                "font": {"size": 14},
            },
            {
                "text": "<b>P99 Latency vs Size</b>",
                "x": 0.775,
                "y": 0.43,
                "xref": "paper",
                "yref": "paper",
                "showarrow": False,
                "font": {"size": 14},
            },
        ],
    }

    # Metadata
    meta_text = build_meta_text(result_sets)

    # Generate checkboxes HTML
    def make_checkboxes(name, values, labels=None, default_all=False):
        if labels is None:
            labels = [str(v) for v in values]
        items = []
        for i, (val, label) in enumerate(zip(values, labels)):
            checked = "checked" if (default_all or i == 0) else ""
            items.append(
                f'<label><input type="checkbox" name="{name}" value="{val}" {checked}> {label}</label>'
            )
        return "\n                ".join(items)

    strategy_labels = [STRATEGY_INFO.get(s, {}).get("label", s) for s in strategies]
    batch_labels = [str(b) if b > 0 else "single" for b in batch_sizes]

    html = f"""<!DOCTYPE html>
<html>
<head>
    <title>hw-eval: DSA Benchmark Dashboard</title>
    <script src="https://cdn.plot.ly/plotly-latest.min.js"></script>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            margin: 0;
            padding: 20px;
            background: #f5f5f5;
        }}
        .container {{
            max-width: 1400px;
            margin: 0 auto;
        }}
        h1 {{
            text-align: center;
            color: #333;
            margin-bottom: 5px;
        }}
        .meta-info {{
            text-align: center;
            color: #888;
            font-size: 12px;
            margin-bottom: 15px;
        }}
        .controls {{
            background: white;
            padding: 15px 20px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            margin-bottom: 20px;
            display: flex;
            flex-wrap: wrap;
            gap: 20px;
        }}
        .filter-group {{
            flex: 1;
            min-width: 130px;
        }}
        .filter-group h3 {{
            margin: 0 0 8px 0;
            font-size: 14px;
            color: #666;
            border-bottom: 1px solid #eee;
            padding-bottom: 5px;
        }}
        .filter-group label {{
            display: block;
            padding: 3px 0;
            cursor: pointer;
            font-size: 13px;
        }}
        .filter-group input[type="checkbox"] {{
            margin-right: 6px;
        }}
        .btn-group {{
            display: flex;
            gap: 5px;
            margin-top: 5px;
        }}
        .btn-group button {{
            font-size: 11px;
            padding: 2px 8px;
            cursor: pointer;
            border: 1px solid #ccc;
            background: #f9f9f9;
            border-radius: 3px;
        }}
        .btn-group button:hover {{
            background: #e9e9e9;
        }}
        .plot-container {{
            background: white;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            padding: 10px;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>hw-eval: DSA Hardware Benchmark Results</h1>
        <div class="meta-info">{meta_text}</div>

        <div class="controls">
            <div class="filter-group">
                <h3>Queue Type</h3>
                {make_checkboxes("queue_type", queue_types, default_all=True)}
                <div class="btn-group">
                    <button onclick="selectAll('queue_type')">All</button>
                    <button onclick="selectNone('queue_type')">None</button>
                </div>
            </div>

            <div class="filter-group">
                <h3>Strategy</h3>
                {make_checkboxes("strategy", strategies, strategy_labels, default_all=True)}
                <div class="btn-group">
                    <button onclick="selectAll('strategy')">All</button>
                    <button onclick="selectNone('strategy')">None</button>
                </div>
            </div>

            <div class="filter-group">
                <h3>Batch Size</h3>
                {make_checkboxes("batch_size", batch_sizes, batch_labels)}
                <div class="btn-group">
                    <button onclick="selectAll('batch_size')">All</button>
                    <button onclick="selectNone('batch_size')">None</button>
                </div>
            </div>

            <div class="filter-group">
                <h3>Concurrency</h3>
                {make_checkboxes("concurrency", concurrencies)}
                <div class="btn-group">
                    <button onclick="selectAll('concurrency')">All</button>
                    <button onclick="selectNone('concurrency')">None</button>
                </div>
            </div>
        </div>

        <div class="plot-container">
            <div id="plotDiv"></div>
        </div>
    </div>

    <script>
        const traces = {json.dumps(traces_json)};
        const traceMetadata = {json.dumps(trace_metadata)};
        const layout = {json.dumps(layout)};
        const STORAGE_KEY = 'hweval_dashboard_filters';

        Plotly.newPlot('plotDiv', traces, layout, {{responsive: true}});

        function getCheckedValues(name) {{
            const cbs = document.querySelectorAll(`input[name="${{name}}"]:checked`);
            return Array.from(cbs).map(cb => cb.value);
        }}

        function saveSelections() {{
            const state = {{}};
            document.querySelectorAll('.filter-group').forEach(group => {{
                const cbs = group.querySelectorAll('input[type="checkbox"]');
                if (cbs.length === 0) return;
                const name = cbs[0].name;
                state[name] = Array.from(cbs).filter(cb => cb.checked).map(cb => cb.value);
            }});
            try {{ localStorage.setItem(STORAGE_KEY, JSON.stringify(state)); }} catch(e) {{}}
        }}

        function restoreSelections() {{
            try {{
                const saved = JSON.parse(localStorage.getItem(STORAGE_KEY));
                if (!saved) return false;
                let restored = false;
                for (const [name, values] of Object.entries(saved)) {{
                    const cbs = document.querySelectorAll(`input[name="${{name}}"]`);
                    if (cbs.length === 0) continue;
                    const available = new Set(Array.from(cbs).map(cb => cb.value));
                    const valid = values.filter(v => available.has(v));
                    if (valid.length > 0) {{
                        cbs.forEach(cb => cb.checked = valid.includes(cb.value));
                        restored = true;
                    }}
                }}
                return restored;
            }} catch(e) {{ return false; }}
        }}

        function selectAll(name) {{
            document.querySelectorAll(`input[name="${{name}}"]`).forEach(cb => cb.checked = true);
            updatePlot();
        }}

        function selectNone(name) {{
            document.querySelectorAll(`input[name="${{name}}"]`).forEach(cb => cb.checked = false);
            updatePlot();
        }}

        function updatePlot() {{
            saveSelections();

            const selStrategy = getCheckedValues('strategy');
            const selQueueType = getCheckedValues('queue_type');
            const selBatch = getCheckedValues('batch_size').map(Number);
            const selConc = getCheckedValues('concurrency').map(Number);

            const visibility = traceMetadata.map(meta => {{
                if (meta.subplot === 'latency_median' || meta.subplot === 'latency_p99') {{
                    return selQueueType.includes(meta.queue_type);
                }}
                return selQueueType.includes(meta.queue_type) &&
                       selStrategy.includes(meta.strategy) &&
                       selBatch.includes(meta.batch_size) &&
                       selConc.includes(meta.concurrency);
            }});

            // Show legend for first visible trace of each label (msg_rate subplot only)
            const seenLabels = new Set();
            const showLegend = traceMetadata.map((meta, i) => {{
                if (visibility[i] && meta.subplot === 'msg_rate' && meta.label && !seenLabels.has(meta.label)) {{
                    seenLabels.add(meta.label);
                    return true;
                }}
                return false;
            }});

            Plotly.restyle('plotDiv', {{
                visible: visibility,
                showlegend: showLegend
            }});
        }}

        document.querySelectorAll('.controls input[type="checkbox"]').forEach(cb => {{
            cb.addEventListener('change', updatePlot);
        }});

        if (restoreSelections()) {{
            updatePlot();
        }}
    </script>
</body>
</html>
"""

    with open(output_path, "w") as f:
        f.write(html)
    print(f"Saved dashboard to: {output_path}")


def create_heatmap_dashboard(result_sets, output_path: Path):
    """Create interactive heatmap dashboard with radio filters."""

    records = []
    for result_set in result_sets:
        queue_type = result_set["queue_type"]
        for r in result_set["data"]["throughput"]:
            strategy, batch_n = classify_benchmark(r["benchmark"])
            if strategy not in ("pipelined_batch", "burst_batch"):
                continue
            records.append(
                {
                    "queue_type": queue_type,
                    "strategy": strategy,
                    "batch_size": batch_n,
                    "concurrency": r["concurrency"],
                    "size": r["size"],
                    "ops_per_sec": r["ops_per_sec"],
                    "bandwidth_mb_s": r["bandwidth_mb_s"],
                }
            )

    if not records:
        print("No pipelined/burst-batch data for heatmap")
        return

    queue_types = sorted(set(r["queue_type"] for r in records))
    strategies = sorted(
        set(r["strategy"] for r in records),
        key=lambda s: list(STRATEGY_INFO.keys()).index(s) if s in STRATEGY_INFO else 99,
    )
    sizes = sorted(set(r["size"] for r in records))

    metrics = [
        ("ops_per_sec", "Message Rate (Mops/s)", 1e-6),
        ("bandwidth_mb_s", "Bandwidth (GB/s)", 1e-3),
    ]

    traces_json = []
    trace_metadata = []

    for queue_type in queue_types:
        for strategy in strategies:
            for size in sizes:
                for metric_key, metric_label, scale in metrics:
                    pts = [
                        r
                        for r in records
                        if r["queue_type"] == queue_type
                        and r["strategy"] == strategy
                        and r["size"] == size
                    ]
                    if not pts:
                        continue

                    batch_sizes = sorted(set(p["batch_size"] for p in pts))
                    conc_levels = sorted(set(p["concurrency"] for p in pts))

                    # Build grid: rows=batch_size, cols=concurrency
                    grid = []
                    for bs in batch_sizes:
                        row = []
                        for c in conc_levels:
                            val = [
                                p[metric_key] * scale
                                for p in pts
                                if p["batch_size"] == bs and p["concurrency"] == c
                            ]
                            row.append(val[0] if val else 0)
                        grid.append(row)

                    conc_label = (
                        "Concurrency"
                        if strategy == "pipelined_batch"
                        else "Burst Count"
                    )
                    visible = (
                        queue_type == queue_types[0]
                        and strategy == strategies[0]
                        and size == sizes[0]
                        and metric_key == "ops_per_sec"
                    )

                    traces_json.append(
                        {
                            "z": grid,
                            "x": [str(c) for c in conc_levels],
                            "y": [str(b) for b in batch_sizes],
                            "type": "heatmap",
                            "colorscale": "YlOrRd",
                            "reversescale": True,
                            "visible": visible,
                            "hovertemplate": f"batch=%{{y}}<br>{conc_label}=%{{x}}<br>%{{z:.1f}}<extra></extra>",
                            "colorbar": {"title": metric_label},
                            "text": [[f"{v:.1f}" for v in row] for row in grid],
                            "texttemplate": "%{text}",
                            "textfont": {"size": 10},
                        }
                    )
                    trace_metadata.append(
                        {
                            "queue_type": queue_type,
                            "strategy": strategy,
                            "size": size,
                            "metric": metric_key,
                            "metric_label": metric_label,
                        }
                    )

    layout = {
        "title": {
            "text": "hw-eval: Batch Strategy Heatmap",
            "x": 0.5,
            "xanchor": "center",
        },
        "xaxis": {"title": "Concurrency / Burst Count"},
        "yaxis": {"title": "Batch Size"},
        "height": 550,
    }

    def make_radios(name, values, labels=None):
        if labels is None:
            labels = [str(v) for v in values]
        items = []
        for i, (val, label) in enumerate(zip(values, labels)):
            checked = "checked" if i == 0 else ""
            items.append(
                f'<label><input type="radio" name="{name}" value="{val}" {checked}> {label}</label>'
            )
        return "\n                ".join(items)

    strategy_labels = [STRATEGY_INFO.get(s, {}).get("label", s) for s in strategies]
    size_labels = [human_size(s) for s in sizes]
    metric_values = [m[0] for m in metrics]
    metric_labels = [m[1] for m in metrics]

    meta_text = build_meta_text(result_sets)

    html = f"""<!DOCTYPE html>
<html>
<head>
    <title>hw-eval: Batch Heatmap</title>
    <script src="https://cdn.plot.ly/plotly-latest.min.js"></script>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            margin: 0;
            padding: 20px;
            background: #f5f5f5;
        }}
        .container {{
            max-width: 1000px;
            margin: 0 auto;
        }}
        h1 {{
            text-align: center;
            color: #333;
            margin-bottom: 5px;
        }}
        .meta-info {{
            text-align: center;
            color: #888;
            font-size: 12px;
            margin-bottom: 15px;
        }}
        .controls {{
            background: white;
            padding: 15px 20px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            margin-bottom: 20px;
            display: flex;
            flex-wrap: wrap;
            gap: 20px;
        }}
        .filter-group {{
            flex: 1;
            min-width: 120px;
        }}
        .filter-group h3 {{
            margin: 0 0 8px 0;
            font-size: 14px;
            color: #666;
            border-bottom: 1px solid #eee;
            padding-bottom: 5px;
        }}
        .filter-group label {{
            display: block;
            padding: 3px 0;
            cursor: pointer;
            font-size: 13px;
        }}
        .filter-group input[type="radio"] {{
            margin-right: 6px;
        }}
        .plot-container {{
            background: white;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            padding: 10px;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>hw-eval: Batch Strategy Heatmap</h1>
        <div class="meta-info">{meta_text}</div>

        <div class="controls">
            <div class="filter-group">
                <h3>Queue Type</h3>
                {make_radios("queue_type", queue_types)}
            </div>

            <div class="filter-group">
                <h3>Strategy</h3>
                {make_radios("strategy", strategies, strategy_labels)}
            </div>

            <div class="filter-group">
                <h3>Message Size</h3>
                {make_radios("size", sizes, size_labels)}
            </div>

            <div class="filter-group">
                <h3>Metric</h3>
                {make_radios("metric", metric_values, metric_labels)}
            </div>
        </div>

        <div class="plot-container">
            <div id="heatmapDiv"></div>
        </div>
    </div>

    <script>
        const traces = {json.dumps(traces_json)};
        const traceMetadata = {json.dumps(trace_metadata)};
        const layout = {json.dumps(layout)};
        const STORAGE_KEY = 'hweval_heatmap_filters';

        Plotly.newPlot('heatmapDiv', traces, layout, {{responsive: true}});

        function getSelectedValue(name) {{
            const radio = document.querySelector(`input[name="${{name}}"]:checked`);
            return radio ? radio.value : null;
        }}

        function saveSelections() {{
            const state = {{}};
            const names = new Set();
            document.querySelectorAll('.controls input[type="radio"]').forEach(r => names.add(r.name));
            names.forEach(name => {{ state[name] = getSelectedValue(name); }});
            try {{ localStorage.setItem(STORAGE_KEY, JSON.stringify(state)); }} catch(e) {{}}
        }}

        function restoreSelections() {{
            try {{
                const saved = JSON.parse(localStorage.getItem(STORAGE_KEY));
                if (!saved) return false;
                let restored = false;
                for (const [name, value] of Object.entries(saved)) {{
                    if (!value) continue;
                    const radio = document.querySelector(`input[name="${{name}}"][value="${{CSS.escape(value)}}"]`);
                    if (radio) {{
                        radio.checked = true;
                        restored = true;
                    }}
                }}
                return restored;
            }} catch(e) {{ return false; }}
        }}

        function updateHeatmap() {{
            saveSelections();

            const selQueueType = getSelectedValue('queue_type');
            const selStrategy = getSelectedValue('strategy');
            const selSize = parseInt(getSelectedValue('size'));
            const selMetric = getSelectedValue('metric');

            const visibility = traceMetadata.map(meta => {{
                return meta.queue_type === selQueueType &&
                       meta.strategy === selStrategy &&
                       meta.size === selSize &&
                       meta.metric === selMetric;
            }});

            const selectedMeta = traceMetadata.find((meta, i) => visibility[i]);
            const xLabel = selStrategy === 'pipelined_batch' ? 'Concurrency' : 'Burst Count';
            const title = selectedMeta ?
                `${{selQueueType}} | ${{selStrategy === 'pipelined_batch' ? 'Pipelined Batch' : 'Burst-Batch'}}: ${{selectedMeta.metric_label}}` :
                'Batch Heatmap';

            Plotly.restyle('heatmapDiv', {{ visible: visibility }});
            Plotly.relayout('heatmapDiv', {{
                'title.text': title,
                'xaxis.title': xLabel
            }});
        }}

        document.querySelectorAll('.controls input[type="radio"]').forEach(r => {{
            r.addEventListener('change', updateHeatmap);
        }});

        if (restoreSelections()) {{
            updateHeatmap();
        }}
    </script>
</body>
</html>
"""

    with open(output_path, "w") as f:
        f.write(html)
    print(f"Saved heatmap to: {output_path}")


def main():
    parser = argparse.ArgumentParser(
        description="Interactive hw-eval benchmark visualization"
    )
    parser.add_argument(
        "input", nargs="+", help="One or more JSON results files from hw-eval --json"
    )
    parser.add_argument("-o", "--outdir", default="graphs", help="Output directory")
    args = parser.parse_args()

    result_sets = load_result_sets(args.input)
    outdir = Path(args.outdir)
    outdir.mkdir(exist_ok=True)

    for result_set in result_sets:
        meta = result_set["metadata"]
        print(
            f"Loaded {result_set['path']}: {meta['device']} ({result_set['queue_type']})"
        )
    print(f"Queue types: {', '.join(sorted({rs['queue_type'] for rs in result_sets}))}")

    create_dashboard(result_sets, outdir / "dashboard.html")
    create_heatmap_dashboard(result_sets, outdir / "heatmap.html")

    print(f"\nOpen in browser:")
    print(f"  {outdir / 'dashboard.html'}")
    print(f"  {outdir / 'heatmap.html'}")


if __name__ == "__main__":
    main()
