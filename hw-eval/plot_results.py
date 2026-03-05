#!/usr/bin/env python3
"""Generate benchmark graphs from hw-eval JSON output.

Usage:
    hw-eval --json > results.json
    python3 plot_results.py results.json [--outdir graphs/]
"""

import argparse
import json
import os
import re
import sys
from collections import defaultdict

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np


def load_results(path):
    with open(path) as f:
        return json.load(f)


def human_size(n):
    if n >= 1048576:
        return f"{n // 1048576}MB"
    if n >= 1024:
        return f"{n // 1024}KB"
    return f"{n}B"


# ============================================================================
# Graph 1: Single-op latency vs message size
# ============================================================================
def plot_latency_vs_size(data, outdir):
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

    # Collect DSA latency benchmarks
    benchmarks = defaultdict(lambda: ([], []))  # name -> (sizes, medians)
    for r in data["latency"]:
        if r["size"] is not None and r["benchmark"] in ("memmove", "crc_gen", "copy_crc", "sw_memcpy", "sw_crc32c"):
            sizes, medians = benchmarks[r["benchmark"]]
            sizes.append(r["size"])
            medians.append(r["ns"]["median"])

    colors = {"memmove": "C0", "crc_gen": "C1", "copy_crc": "C2", "sw_memcpy": "C3", "sw_crc32c": "C4"}
    markers = {"memmove": "o", "crc_gen": "s", "copy_crc": "D", "sw_memcpy": "^", "sw_crc32c": "v"}

    for name in ("memmove", "crc_gen", "copy_crc", "sw_memcpy", "sw_crc32c"):
        if name not in benchmarks:
            continue
        sizes, medians = benchmarks[name]
        order = sorted(range(len(sizes)), key=lambda i: sizes[i])
        xs = [sizes[i] for i in order]
        ys = [medians[i] for i in order]
        ax1.plot(xs, ys, marker=markers[name], color=colors[name], label=name, linewidth=2, markersize=6)
        # Bandwidth plot
        bw = [s / (m * 1e-9) / 1e9 for s, m in zip(xs, ys)]  # GB/s
        ax2.plot(xs, bw, marker=markers[name], color=colors[name], label=name, linewidth=2, markersize=6)

    ax1.set_xscale("log", base=2)
    ax1.set_yscale("log")
    ax1.set_xlabel("Message size (bytes)")
    ax1.set_ylabel("Median latency (ns)")
    ax1.set_title("Single-op Latency vs Message Size")
    ax1.legend(fontsize=9)
    ax1.grid(True, alpha=0.3)
    ax1.set_xticks([64, 256, 1024, 4096, 16384, 65536, 262144, 1048576])
    ax1.set_xticklabels([human_size(x) for x in [64, 256, 1024, 4096, 16384, 65536, 262144, 1048576]], rotation=45)

    ax2.set_xscale("log", base=2)
    ax2.set_xlabel("Message size (bytes)")
    ax2.set_ylabel("Bandwidth (GB/s)")
    ax2.set_title("Effective Bandwidth vs Message Size")
    ax2.legend(fontsize=9)
    ax2.grid(True, alpha=0.3)
    ax2.set_xticks([64, 256, 1024, 4096, 16384, 65536, 262144, 1048576])
    ax2.set_xticklabels([human_size(x) for x in [64, 256, 1024, 4096, 16384, 65536, 262144, 1048576]], rotation=45)

    fig.tight_layout()
    fig.savefig(os.path.join(outdir, "latency_vs_size.png"), dpi=150)
    plt.close(fig)
    print(f"  latency_vs_size.png")


# ============================================================================
# Graph 2: Throughput vs concurrency (sliding window vs burst)
# ============================================================================
def plot_throughput_vs_concurrency(data, outdir):
    # Collect sliding window and burst data by size
    sw_data = defaultdict(lambda: ([], []))  # size -> (conc, ops)
    burst_data = defaultdict(lambda: ([], []))

    for r in data["throughput"]:
        if r["benchmark"] == "memmove":
            sw_data[r["size"]][0].append(r["concurrency"])
            sw_data[r["size"]][1].append(r["ops_per_sec"] / 1e6)
        elif r["benchmark"] == "burst_memmove":
            burst_data[r["size"]][0].append(r["concurrency"])
            burst_data[r["size"]][1].append(r["ops_per_sec"] / 1e6)

    # Also collect best pipelined batch per size per concurrency
    pb_data = defaultdict(lambda: defaultdict(float))  # size -> conc -> best_ops
    for r in data["throughput"]:
        m = re.match(r"pipelined_batch_b\d+", r["benchmark"])
        if m:
            key = r["size"]
            if r["ops_per_sec"] > pb_data[key][r["concurrency"]]:
                pb_data[key][r["concurrency"]] = r["ops_per_sec"]

    # Collect best burst_batch per size per concurrency
    bb_data = defaultdict(lambda: defaultdict(float))  # size -> conc -> best_ops
    for r in data["throughput"]:
        m = re.match(r"burst_batch_b\d+", r["benchmark"])
        if m:
            key = r["size"]
            if r["ops_per_sec"] > bb_data[key][r["concurrency"]]:
                bb_data[key][r["concurrency"]] = r["ops_per_sec"]

    sizes = sorted(set(list(sw_data.keys()) + list(burst_data.keys())))
    if not sizes:
        return

    ncols = min(3, len(sizes))
    nrows = (len(sizes) + ncols - 1) // ncols
    fig, axes = plt.subplots(nrows, ncols, figsize=(6 * ncols, 5 * nrows), squeeze=False)

    for idx, size in enumerate(sizes):
        ax = axes[idx // ncols][idx % ncols]

        if size in sw_data:
            conc, ops = sw_data[size]
            order = sorted(range(len(conc)), key=lambda i: conc[i])
            ax.plot([conc[i] for i in order], [ops[i] for i in order],
                    "o-", label="sliding_window", linewidth=2, color="C0")

        if size in burst_data:
            conc, ops = burst_data[size]
            order = sorted(range(len(conc)), key=lambda i: conc[i])
            ax.plot([conc[i] for i in order], [ops[i] for i in order],
                    "s--", label="burst", linewidth=2, color="C1")

        if size in pb_data and pb_data[size]:
            concs = sorted(pb_data[size].keys())
            ops = [pb_data[size][c] / 1e6 for c in concs]
            ax.plot(concs, ops, "D-.", label="pipelined_batch (best)", linewidth=2, color="C2")

        if size in bb_data and bb_data[size]:
            concs = sorted(bb_data[size].keys())
            ops = [bb_data[size][c] / 1e6 for c in concs]
            ax.plot(concs, ops, "^:", label="burst_batch (best)", linewidth=2, color="C3")

        ax.set_xscale("log", base=2)
        ax.set_xlabel("Concurrency / Burst size")
        ax.set_ylabel("Mops/sec")
        ax.set_title(f"Throughput: memmove {human_size(size)}")
        ax.legend(fontsize=8)
        ax.grid(True, alpha=0.3)

    # Hide empty subplots
    for idx in range(len(sizes), nrows * ncols):
        axes[idx // ncols][idx % ncols].set_visible(False)

    fig.tight_layout()
    fig.savefig(os.path.join(outdir, "throughput_vs_concurrency.png"), dpi=150)
    plt.close(fig)
    print(f"  throughput_vs_concurrency.png")


# ============================================================================
# Graph 3: Batch amortization curve
# ============================================================================
def plot_batch_amortization(data, outdir):
    batch_data = defaultdict(lambda: ([], []))  # "batch_memmove" -> (batch_n, per_op_ns)
    for r in data["latency"]:
        if r["benchmark"] == "batch_memmove" and r["batch_size"] is not None:
            batch_data[r["size"]][0].append(r["batch_size"])
            batch_data[r["size"]][1].append(r["ns"]["median"] / r["batch_size"])

    if not batch_data:
        return

    fig, ax = plt.subplots(figsize=(10, 6))
    for size in sorted(batch_data.keys()):
        bs, per_op = batch_data[size]
        order = sorted(range(len(bs)), key=lambda i: bs[i])
        ax.plot([bs[i] for i in order], [per_op[i] for i in order],
                "o-", label=f"{human_size(size)}", linewidth=2, markersize=5)

    # Add NOOP line
    for r in data["latency"]:
        if r["benchmark"] == "noop":
            ax.axhline(y=r["ns"]["median"], color="red", linestyle=":", linewidth=1.5, label=f"NOOP ({r['ns']['median']} ns)")
            break

    ax.set_xscale("log", base=2)
    ax.set_xlabel("Batch size")
    ax.set_ylabel("Per-op latency (ns)")
    ax.set_title("Batch Amortization: Per-op Latency vs Batch Size")
    ax.legend()
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    fig.savefig(os.path.join(outdir, "batch_amortization.png"), dpi=150)
    plt.close(fig)
    print(f"  batch_amortization.png")


# ============================================================================
# Graph 4: Pipelined batch heatmap (batch_size x concurrency -> Mops/sec)
# ============================================================================
def plot_pipelined_heatmap(data, outdir):
    # Collect data per size
    by_size = defaultdict(list)  # size -> [(batch_n, conc, ops)]
    for r in data["throughput"]:
        m = re.match(r"pipelined_batch_b(\d+)", r["benchmark"])
        if m:
            bn = int(m.group(1))
            by_size[r["size"]].append((bn, r["concurrency"], r["ops_per_sec"] / 1e6))

    if not by_size:
        return

    sizes = sorted(by_size.keys())
    ncols = min(3, len(sizes))
    nrows = (len(sizes) + ncols - 1) // ncols
    fig, axes = plt.subplots(nrows, ncols, figsize=(7 * ncols, 5 * nrows), squeeze=False)

    for idx, size in enumerate(sizes):
        ax = axes[idx // ncols][idx % ncols]
        entries = by_size[size]

        batch_sizes = sorted(set(e[0] for e in entries))
        conc_levels = sorted(set(e[1] for e in entries))
        grid = np.zeros((len(batch_sizes), len(conc_levels)))

        bi_map = {b: i for i, b in enumerate(batch_sizes)}
        ci_map = {c: i for i, c in enumerate(conc_levels)}
        for bn, conc, ops in entries:
            grid[bi_map[bn]][ci_map[conc]] = ops

        im = ax.imshow(grid, aspect="auto", origin="lower", cmap="YlOrRd")
        ax.set_xticks(range(len(conc_levels)))
        ax.set_xticklabels(conc_levels)
        ax.set_yticks(range(len(batch_sizes)))
        ax.set_yticklabels(batch_sizes)
        ax.set_xlabel("Concurrency")
        ax.set_ylabel("Batch size")
        ax.set_title(f"Pipelined Batch: {human_size(size)} (Mops/sec)")

        # Annotate cells
        for i in range(len(batch_sizes)):
            for j in range(len(conc_levels)):
                val = grid[i][j]
                color = "white" if val > grid.max() * 0.6 else "black"
                ax.text(j, i, f"{val:.1f}", ha="center", va="center", fontsize=7, color=color)

        fig.colorbar(im, ax=ax, shrink=0.8)

    for idx in range(len(sizes), nrows * ncols):
        axes[idx // ncols][idx % ncols].set_visible(False)

    fig.tight_layout()
    fig.savefig(os.path.join(outdir, "pipelined_batch_heatmap.png"), dpi=150)
    plt.close(fig)
    print(f"  pipelined_batch_heatmap.png")


# ============================================================================
# Graph 4b: Burst-batch heatmap (batch_size x burst_count -> Mops/sec)
# ============================================================================
def plot_burst_batch_heatmap(data, outdir):
    by_size = defaultdict(list)  # size -> [(batch_n, burst, ops)]
    for r in data["throughput"]:
        m = re.match(r"burst_batch_b(\d+)", r["benchmark"])
        if m:
            bn = int(m.group(1))
            by_size[r["size"]].append((bn, r["concurrency"], r["ops_per_sec"] / 1e6))

    if not by_size:
        return

    sizes = sorted(by_size.keys())
    ncols = min(3, len(sizes))
    nrows = (len(sizes) + ncols - 1) // ncols
    fig, axes = plt.subplots(nrows, ncols, figsize=(7 * ncols, 5 * nrows), squeeze=False)

    for idx, size in enumerate(sizes):
        ax = axes[idx // ncols][idx % ncols]
        entries = by_size[size]

        batch_sizes = sorted(set(e[0] for e in entries))
        burst_levels = sorted(set(e[1] for e in entries))
        grid = np.zeros((len(batch_sizes), len(burst_levels)))

        bi_map = {b: i for i, b in enumerate(batch_sizes)}
        ci_map = {c: i for i, c in enumerate(burst_levels)}
        for bn, burst, ops in entries:
            grid[bi_map[bn]][ci_map[burst]] = ops

        im = ax.imshow(grid, aspect="auto", origin="lower", cmap="YlOrRd")
        ax.set_xticks(range(len(burst_levels)))
        ax.set_xticklabels(burst_levels)
        ax.set_yticks(range(len(batch_sizes)))
        ax.set_yticklabels(batch_sizes)
        ax.set_xlabel("Burst count")
        ax.set_ylabel("Batch size")
        ax.set_title(f"Burst-Batch: {human_size(size)} (Mops/sec)")

        for i in range(len(batch_sizes)):
            for j in range(len(burst_levels)):
                val = grid[i][j]
                color = "white" if val > grid.max() * 0.6 else "black"
                ax.text(j, i, f"{val:.1f}", ha="center", va="center", fontsize=7, color=color)

        fig.colorbar(im, ax=ax, shrink=0.8)

    for idx in range(len(sizes), nrows * ncols):
        axes[idx // ncols][idx % ncols].set_visible(False)

    fig.tight_layout()
    fig.savefig(os.path.join(outdir, "burst_batch_heatmap.png"), dpi=150)
    plt.close(fig)
    print(f"  burst_batch_heatmap.png")


# ============================================================================
# Graph 5: Strategy comparison bar chart
# ============================================================================
def plot_strategy_comparison(data, outdir):
    # Find peak ops/sec for each strategy at each size
    strategies = {
        "sliding_window": defaultdict(float),
        "burst": defaultdict(float),
        "pipelined_batch": defaultdict(float),
        "burst_batch": defaultdict(float),
    }

    for r in data["throughput"]:
        sz = r["size"]
        ops = r["ops_per_sec"] / 1e6
        if r["benchmark"] == "memmove":
            strategies["sliding_window"][sz] = max(strategies["sliding_window"][sz], ops)
        elif r["benchmark"] == "burst_memmove":
            strategies["burst"][sz] = max(strategies["burst"][sz], ops)
        elif re.match(r"pipelined_batch_b\d+", r["benchmark"]):
            strategies["pipelined_batch"][sz] = max(strategies["pipelined_batch"][sz], ops)
        elif re.match(r"burst_batch_b\d+", r["benchmark"]):
            strategies["burst_batch"][sz] = max(strategies["burst_batch"][sz], ops)

    sizes = sorted(set().union(*[s.keys() for s in strategies.values()]))
    if not sizes:
        return

    fig, ax = plt.subplots(figsize=(14, 6))
    x = np.arange(len(sizes))
    width = 0.2
    colors = {"sliding_window": "C0", "burst": "C1", "pipelined_batch": "C2", "burst_batch": "C3"}

    for i, (name, vals) in enumerate(strategies.items()):
        heights = [vals.get(sz, 0) for sz in sizes]
        bars = ax.bar(x + i * width, heights, width, label=name, color=colors[name])
        for bar, h in zip(bars, heights):
            if h > 0:
                ax.text(bar.get_x() + bar.get_width() / 2, bar.get_height(),
                        f"{h:.1f}", ha="center", va="bottom", fontsize=7)

    ax.set_xlabel("Message size")
    ax.set_ylabel("Peak Mops/sec")
    ax.set_title("Peak Message Rate by Strategy")
    ax.set_xticks(x + 1.5 * width)
    ax.set_xticklabels([human_size(sz) for sz in sizes])
    ax.legend()
    ax.grid(True, alpha=0.3, axis="y")
    fig.tight_layout()
    fig.savefig(os.path.join(outdir, "strategy_comparison.png"), dpi=150)
    plt.close(fig)
    print(f"  strategy_comparison.png")


def main():
    parser = argparse.ArgumentParser(description="Plot hw-eval benchmark results")
    parser.add_argument("input", help="JSON results file from hw-eval --json")
    parser.add_argument("--outdir", default="graphs", help="Output directory for PNG files")
    args = parser.parse_args()

    data = load_results(args.input)
    os.makedirs(args.outdir, exist_ok=True)

    meta = data["metadata"]
    print(f"Results: {args.input}")
    print(f"  Device: {meta['device']} ({'dedicated' if meta.get('wq_dedicated') else 'shared'})")
    print(f"  Core: {meta['pinned_core']}, NUMA: CPU={meta.get('cpu_numa_node')}, DSA={meta.get('device_numa_node')}")
    print(f"  TSC: {meta['tsc_freq_hz'] / 1e9:.3f} GHz, Iterations: {meta['iterations']}")
    print(f"  Cold cache: {meta['cold_cache']}")
    print(f"\nGenerating graphs to {args.outdir}/:")

    plot_latency_vs_size(data, args.outdir)
    plot_throughput_vs_concurrency(data, args.outdir)
    plot_batch_amortization(data, args.outdir)
    plot_pipelined_heatmap(data, args.outdir)
    plot_burst_batch_heatmap(data, args.outdir)
    plot_strategy_comparison(data, args.outdir)

    print(f"\nDone. {len(os.listdir(args.outdir))} graphs generated.")


if __name__ == "__main__":
    main()
