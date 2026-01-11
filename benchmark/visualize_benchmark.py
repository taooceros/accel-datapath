#!/usr/bin/env python3
"""
Visualization script for DSA benchmark results.
Reads dsa_benchmark_results.csv and generates plots.
"""

import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
from pathlib import Path
import argparse

# Distinct marker styles for each queue type
MARKERS = ['o', 's', '^', 'D', 'v', 'P', 'X']  # circle, square, triangle, diamond, etc.
LINE_STYLES = ['-', '--', '-.', ':', '-', '--', '-.']
# Colorblind-friendly palette
COLORS = ['#1f77b4', '#ff7f0e', '#2ca02c', '#d62728', '#9467bd', '#8c564b', '#e377c2']


def get_style(idx: int) -> dict:
    """Get consistent style for a queue type index."""
    color = COLORS[idx % len(COLORS)]
    return {
        'marker': MARKERS[idx % len(MARKERS)],
        'linestyle': LINE_STYLES[idx % len(LINE_STYLES)],
        'color': color,
        'markersize': 5,
        'linewidth': 1.5,
        'markerfacecolor': 'none',  # Hollow markers - no fill
        'markeredgecolor': color,   # Edge same as line color
        'markeredgewidth': 1,
    }


def load_data(csv_path: str) -> pd.DataFrame:
    """Load benchmark results from CSV."""
    df = pd.read_csv(csv_path)
    # Convert msg_size to human-readable format for labels
    df['msg_size_label'] = df['msg_size'].apply(format_size)
    return df


def format_size(size: int) -> str:
    """Format byte size to human-readable string."""
    if size >= 1024 * 1024:
        return f"{size // (1024 * 1024)}MB"
    elif size >= 1024:
        return f"{size // 1024}KB"
    else:
        return f"{size}B"


def plot_bandwidth_by_queue(df: pd.DataFrame, mode: str, ax: plt.Axes):
    """Plot bandwidth comparison across queue types for a given mode."""
    data = df[df['mode'] == mode]
    queue_types = data['queue_type'].unique()

    # Group by batch_size and msg_size
    configs = data.groupby(['batch_size', 'msg_size_label']).first().index.tolist()
    x = np.arange(len(configs))
    width = 0.12

    for i, qt in enumerate(queue_types):
        qt_data = data[data['queue_type'] == qt]
        bw_values = []
        for batch, msg_label in configs:
            row = qt_data[(qt_data['batch_size'] == batch) &
                          (qt_data['msg_size_label'] == msg_label)]
            bw_values.append(row['bandwidth_gbps'].values[0] if len(row) > 0 else 0)

        offset = (i - len(queue_types) / 2 + 0.5) * width
        ax.bar(x + offset, bw_values, width, label=qt,
               color=COLORS[i % len(COLORS)], edgecolor='white', linewidth=0.5)

    ax.set_xlabel('Configuration (batch_size, msg_size)')
    ax.set_ylabel('Bandwidth (GB/s)')
    ax.set_title(f'Bandwidth by Queue Type ({mode.capitalize()} Polling)')
    ax.set_xticks(x)
    ax.set_xticklabels([f"b{b},{m}" for b, m in configs], rotation=45, ha='right')
    ax.legend(loc='upper left', fontsize='small', framealpha=0.9)
    ax.grid(axis='y', alpha=0.3)


def plot_latency_by_queue(df: pd.DataFrame, mode: str, ax: plt.Axes,
                          latency_type: str = 'avg'):
    """Plot latency comparison across queue types."""
    data = df[df['mode'] == mode]
    queue_types = data['queue_type'].unique()

    col_map = {
        'avg': 'latency_avg_ns',
        'p50': 'latency_p50_ns',
        'p99': 'latency_p99_ns',
        'min': 'latency_min_ns',
        'max': 'latency_max_ns'
    }
    col = col_map.get(latency_type, 'latency_avg_ns')

    configs = data.groupby(['batch_size', 'msg_size_label']).first().index.tolist()
    x = np.arange(len(configs))
    width = 0.12

    for i, qt in enumerate(queue_types):
        qt_data = data[data['queue_type'] == qt]
        lat_values = []
        for batch, msg_label in configs:
            row = qt_data[(qt_data['batch_size'] == batch) &
                          (qt_data['msg_size_label'] == msg_label)]
            # Convert to microseconds for readability
            val = row[col].values[0] / 1000.0 if len(row) > 0 else 0
            lat_values.append(val)

        offset = (i - len(queue_types) / 2 + 0.5) * width
        ax.bar(x + offset, lat_values, width, label=qt,
               color=COLORS[i % len(COLORS)], edgecolor='white', linewidth=0.5)

    ax.set_xlabel('Configuration (batch_size, msg_size)')
    ax.set_ylabel(f'Latency ({latency_type}) (us)')
    ax.set_title(f'{latency_type.upper()} Latency by Queue Type ({mode.capitalize()} Polling)')
    ax.set_xticks(x)
    ax.set_xticklabels([f"b{b},{m}" for b, m in configs], rotation=45, ha='right')
    ax.legend(loc='upper left', fontsize='small', framealpha=0.9)
    ax.grid(axis='y', alpha=0.3)


def plot_bandwidth_vs_msgsize(df: pd.DataFrame, mode: str, batch_size: int, ax: plt.Axes):
    """Plot bandwidth vs message size for a specific batch size."""
    data = df[(df['mode'] == mode) & (df['batch_size'] == batch_size)]
    queue_types = data['queue_type'].unique()

    for i, qt in enumerate(queue_types):
        qt_data = data[data['queue_type'] == qt].sort_values('msg_size')
        style = get_style(i)
        ax.plot(qt_data['msg_size'], qt_data['bandwidth_gbps'],
                label=qt, **style)

    ax.set_xlabel('Message Size (bytes)')
    ax.set_ylabel('Bandwidth (GB/s)')
    ax.set_title(f'Bandwidth vs Message Size (batch={batch_size}, {mode})')
    ax.set_xscale('log', base=2)
    ax.legend(fontsize='small', framealpha=0.9)
    ax.grid(True, alpha=0.3)


def plot_latency_vs_msgsize(df: pd.DataFrame, mode: str, batch_size: int, ax: plt.Axes):
    """Plot latency vs message size for a specific batch size."""
    data = df[(df['mode'] == mode) & (df['batch_size'] == batch_size)]
    queue_types = data['queue_type'].unique()

    for i, qt in enumerate(queue_types):
        qt_data = data[data['queue_type'] == qt].sort_values('msg_size')
        style = get_style(i)
        # Convert to microseconds
        ax.plot(qt_data['msg_size'], qt_data['latency_avg_ns'] / 1000.0,
                label=qt, **style)

    ax.set_xlabel('Message Size (bytes)')
    ax.set_ylabel('Average Latency (us)')
    ax.set_title(f'Latency vs Message Size (batch={batch_size}, {mode})')
    ax.set_xscale('log', base=2)
    ax.legend(fontsize='small', framealpha=0.9)
    ax.grid(True, alpha=0.3)


def plot_latency_p50_vs_msgsize(df: pd.DataFrame, mode: str, batch_size: int, ax: plt.Axes):
    """Plot median (p50) latency vs message size for a specific batch size."""
    data = df[(df['mode'] == mode) & (df['batch_size'] == batch_size)]
    queue_types = data['queue_type'].unique()

    for i, qt in enumerate(queue_types):
        qt_data = data[data['queue_type'] == qt].sort_values('msg_size')
        style = get_style(i)
        # Convert to microseconds
        ax.plot(qt_data['msg_size'], qt_data['latency_p50_ns'] / 1000.0,
                label=qt, **style)

    ax.set_xlabel('Message Size (bytes)')
    ax.set_ylabel('Median Latency (us)')
    ax.set_title(f'P50 Latency vs Message Size (batch={batch_size}, {mode})')
    ax.set_xscale('log', base=2)
    ax.legend(fontsize='small', framealpha=0.9)
    ax.grid(True, alpha=0.3)


def plot_latency_p99_vs_msgsize(df: pd.DataFrame, mode: str, batch_size: int, ax: plt.Axes):
    """Plot p99 latency vs message size for a specific batch size."""
    data = df[(df['mode'] == mode) & (df['batch_size'] == batch_size)]
    queue_types = data['queue_type'].unique()

    for i, qt in enumerate(queue_types):
        qt_data = data[data['queue_type'] == qt].sort_values('msg_size')
        style = get_style(i)
        # Convert to microseconds
        ax.plot(qt_data['msg_size'], qt_data['latency_p99_ns'] / 1000.0,
                label=qt, **style)

    ax.set_xlabel('Message Size (bytes)')
    ax.set_ylabel('P99 Latency (us)')
    ax.set_title(f'P99 Latency vs Message Size (batch={batch_size}, {mode})')
    ax.set_xscale('log', base=2)
    ax.legend(fontsize='small', framealpha=0.9)
    ax.grid(True, alpha=0.3)


def plot_latency_percentiles(df: pd.DataFrame, mode: str, queue_type: str, ax: plt.Axes):
    """Plot latency percentiles (min, p50, avg, p99, max) for a queue type."""
    data = df[(df['mode'] == mode) & (df['queue_type'] == queue_type)]
    data = data.sort_values(['batch_size', 'msg_size'])

    configs = [f"b{r['batch_size']},{format_size(r['msg_size'])}"
               for _, r in data.iterrows()]
    x = np.arange(len(configs))

    # Convert to microseconds
    min_lat = data['latency_min_ns'].values / 1000.0
    p50_lat = data['latency_p50_ns'].values / 1000.0
    avg_lat = data['latency_avg_ns'].values / 1000.0
    p99_lat = data['latency_p99_ns'].values / 1000.0
    max_lat = data['latency_max_ns'].values / 1000.0

    # Use distinct colors for the ranges and avg line
    ax.fill_between(x, min_lat, max_lat, alpha=0.15, color='#1f77b4', label='min-max range')
    ax.fill_between(x, p50_lat, p99_lat, alpha=0.3, color='#ff7f0e', label='p50-p99 range')
    ax.plot(x, avg_lat, 'o-', label='avg', linewidth=1.5, markersize=5,
            color='#d62728', markerfacecolor='none', markeredgewidth=1)
    # Also show p50 and p99 as distinct lines
    ax.plot(x, p50_lat, 's--', label='p50', linewidth=1, markersize=4,
            color='#2ca02c', markerfacecolor='none', markeredgewidth=1)
    ax.plot(x, p99_lat, '^:', label='p99', linewidth=1, markersize=4,
            color='#9467bd', markerfacecolor='none', markeredgewidth=1)

    ax.set_xlabel('Configuration')
    ax.set_ylabel('Latency (us)')
    ax.set_title(f'Latency Distribution: {queue_type} ({mode})')
    ax.set_xticks(x)
    ax.set_xticklabels(configs, rotation=45, ha='right')
    ax.legend(fontsize='small', framealpha=0.9, loc='upper left')
    ax.grid(True, alpha=0.3)


def plot_heatmap(df: pd.DataFrame, mode: str, queue_type: str, metric: str, ax: plt.Axes):
    """Plot heatmap of metric by batch_size and msg_size."""
    data = df[(df['mode'] == mode) & (df['queue_type'] == queue_type)]

    pivot = data.pivot(index='batch_size', columns='msg_size', values=metric)

    # Convert latency to microseconds if applicable
    if 'latency' in metric:
        pivot = pivot / 1000.0
        unit = 'us'
    else:
        unit = 'GB/s'

    im = ax.imshow(pivot.values, cmap='viridis', aspect='auto')

    # Set ticks
    ax.set_xticks(np.arange(len(pivot.columns)))
    ax.set_yticks(np.arange(len(pivot.index)))
    ax.set_xticklabels([format_size(c) for c in pivot.columns])
    ax.set_yticklabels(pivot.index)

    # Add colorbar
    cbar = plt.colorbar(im, ax=ax)
    cbar.set_label(f'{metric} ({unit})')

    # Add text annotations
    for i in range(len(pivot.index)):
        for j in range(len(pivot.columns)):
            val = pivot.values[i, j]
            text = f'{val:.1f}' if val < 100 else f'{val:.0f}'
            ax.text(j, i, text, ha='center', va='center',
                   color='white' if val < pivot.values.max() * 0.7 else 'black',
                   fontsize=8)

    ax.set_xlabel('Message Size')
    ax.set_ylabel('Batch Size')
    ax.set_title(f'{metric}: {queue_type} ({mode})')


def generate_summary_report(df: pd.DataFrame, output_dir: Path):
    """Generate a comprehensive visualization report."""
    output_dir.mkdir(exist_ok=True)

    modes = df['mode'].unique()
    batch_sizes = sorted(df['batch_size'].unique())
    queue_types = df['queue_type'].unique()

    # 1. Overview: Bandwidth comparison for all modes
    fig, axes = plt.subplots(1, len(modes), figsize=(8 * len(modes), 6))
    if len(modes) == 1:
        axes = [axes]
    for ax, mode in zip(axes, modes):
        plot_bandwidth_by_queue(df, mode, ax)
    plt.tight_layout()
    plt.savefig(output_dir / 'bandwidth_overview.png', dpi=150)
    plt.close()
    print(f"Saved: {output_dir / 'bandwidth_overview.png'}")

    # 2. Latency comparison (avg)
    fig, axes = plt.subplots(1, len(modes), figsize=(8 * len(modes), 6))
    if len(modes) == 1:
        axes = [axes]
    for ax, mode in zip(axes, modes):
        plot_latency_by_queue(df, mode, ax, 'avg')
    plt.tight_layout()
    plt.savefig(output_dir / 'latency_avg_overview.png', dpi=150)
    plt.close()
    print(f"Saved: {output_dir / 'latency_avg_overview.png'}")

    # 3. Latency comparison (p99)
    fig, axes = plt.subplots(1, len(modes), figsize=(8 * len(modes), 6))
    if len(modes) == 1:
        axes = [axes]
    for ax, mode in zip(axes, modes):
        plot_latency_by_queue(df, mode, ax, 'p99')
    plt.tight_layout()
    plt.savefig(output_dir / 'latency_p99_overview.png', dpi=150)
    plt.close()
    print(f"Saved: {output_dir / 'latency_p99_overview.png'}")

    # 4. Bandwidth vs message size for each batch size
    for batch in batch_sizes:
        fig, axes = plt.subplots(1, len(modes), figsize=(7 * len(modes), 5))
        if len(modes) == 1:
            axes = [axes]
        for ax, mode in zip(axes, modes):
            plot_bandwidth_vs_msgsize(df, mode, batch, ax)
        plt.tight_layout()
        plt.savefig(output_dir / f'bandwidth_vs_msgsize_batch{batch}.png', dpi=150)
        plt.close()
        print(f"Saved: {output_dir / f'bandwidth_vs_msgsize_batch{batch}.png'}")

    # 5. Latency (avg) vs message size for each batch size
    for batch in batch_sizes:
        fig, axes = plt.subplots(1, len(modes), figsize=(7 * len(modes), 5))
        if len(modes) == 1:
            axes = [axes]
        for ax, mode in zip(axes, modes):
            plot_latency_vs_msgsize(df, mode, batch, ax)
        plt.tight_layout()
        plt.savefig(output_dir / f'latency_avg_vs_msgsize_batch{batch}.png', dpi=150)
        plt.close()
        print(f"Saved: {output_dir / f'latency_avg_vs_msgsize_batch{batch}.png'}")

    # 6. Latency (p50/median) vs message size for each batch size
    for batch in batch_sizes:
        fig, axes = plt.subplots(1, len(modes), figsize=(7 * len(modes), 5))
        if len(modes) == 1:
            axes = [axes]
        for ax, mode in zip(axes, modes):
            plot_latency_p50_vs_msgsize(df, mode, batch, ax)
        plt.tight_layout()
        plt.savefig(output_dir / f'latency_p50_vs_msgsize_batch{batch}.png', dpi=150)
        plt.close()
        print(f"Saved: {output_dir / f'latency_p50_vs_msgsize_batch{batch}.png'}")

    # 7. Latency (p99) vs message size for each batch size
    for batch in batch_sizes:
        fig, axes = plt.subplots(1, len(modes), figsize=(7 * len(modes), 5))
        if len(modes) == 1:
            axes = [axes]
        for ax, mode in zip(axes, modes):
            plot_latency_p99_vs_msgsize(df, mode, batch, ax)
        plt.tight_layout()
        plt.savefig(output_dir / f'latency_p99_vs_msgsize_batch{batch}.png', dpi=150)
        plt.close()
        print(f"Saved: {output_dir / f'latency_p99_vs_msgsize_batch{batch}.png'}")

    # 8. Latency percentile distribution for each queue type
    for mode in modes:
        mode_queues = df[df['mode'] == mode]['queue_type'].unique()
        n_queues = len(mode_queues)
        cols = min(3, n_queues)
        rows = (n_queues + cols - 1) // cols
        fig, axes = plt.subplots(rows, cols, figsize=(5 * cols, 4 * rows))
        axes = np.atleast_2d(axes).flatten()
        for i, qt in enumerate(mode_queues):
            plot_latency_percentiles(df, mode, qt, axes[i])
        # Hide unused axes
        for j in range(n_queues, len(axes)):
            axes[j].set_visible(False)
        plt.tight_layout()
        plt.savefig(output_dir / f'latency_percentiles_{mode}.png', dpi=150)
        plt.close()
        print(f"Saved: {output_dir / f'latency_percentiles_{mode}.png'}")

    # 9. Heatmaps for bandwidth
    for mode in modes:
        mode_queues = df[df['mode'] == mode]['queue_type'].unique()
        n_queues = len(mode_queues)
        cols = min(3, n_queues)
        rows = (n_queues + cols - 1) // cols
        fig, axes = plt.subplots(rows, cols, figsize=(6 * cols, 5 * rows))
        axes = np.atleast_2d(axes).flatten()
        for i, qt in enumerate(mode_queues):
            plot_heatmap(df, mode, qt, 'bandwidth_gbps', axes[i])
        for j in range(n_queues, len(axes)):
            axes[j].set_visible(False)
        plt.tight_layout()
        plt.savefig(output_dir / f'heatmap_bandwidth_{mode}.png', dpi=150)
        plt.close()
        print(f"Saved: {output_dir / f'heatmap_bandwidth_{mode}.png'}")

    # 10. Heatmaps for latency
    for mode in modes:
        mode_queues = df[df['mode'] == mode]['queue_type'].unique()
        n_queues = len(mode_queues)
        cols = min(3, n_queues)
        rows = (n_queues + cols - 1) // cols
        fig, axes = plt.subplots(rows, cols, figsize=(6 * cols, 5 * rows))
        axes = np.atleast_2d(axes).flatten()
        for i, qt in enumerate(mode_queues):
            plot_heatmap(df, mode, qt, 'latency_avg_ns', axes[i])
        for j in range(n_queues, len(axes)):
            axes[j].set_visible(False)
        plt.tight_layout()
        plt.savefig(output_dir / f'heatmap_latency_{mode}.png', dpi=150)
        plt.close()
        print(f"Saved: {output_dir / f'heatmap_latency_{mode}.png'}")


def print_summary_stats(df: pd.DataFrame):
    """Print summary statistics to console."""
    print("\n" + "=" * 60)
    print("BENCHMARK SUMMARY STATISTICS")
    print("=" * 60)

    for mode in df['mode'].unique():
        print(f"\n--- {mode.upper()} POLLING ---")
        mode_data = df[df['mode'] == mode]

        # Best bandwidth per queue type
        print("\nBest Bandwidth (GB/s):")
        for qt in mode_data['queue_type'].unique():
            qt_data = mode_data[mode_data['queue_type'] == qt]
            best = qt_data.loc[qt_data['bandwidth_gbps'].idxmax()]
            print(f"  {qt:12}: {best['bandwidth_gbps']:.2f} GB/s "
                  f"(batch={int(best['batch_size'])}, msg={format_size(int(best['msg_size']))})")

        # Best latency per queue type
        print("\nBest Avg Latency (us):")
        for qt in mode_data['queue_type'].unique():
            qt_data = mode_data[mode_data['queue_type'] == qt]
            best = qt_data.loc[qt_data['latency_avg_ns'].idxmin()]
            print(f"  {qt:12}: {best['latency_avg_ns']/1000:.2f} us "
                  f"(batch={int(best['batch_size'])}, msg={format_size(int(best['msg_size']))})")


def main():
    parser = argparse.ArgumentParser(description='Visualize DSA benchmark results')
    parser.add_argument('csv_file', nargs='?', default='dsa_benchmark_results.csv',
                        help='Path to CSV file (default: dsa_benchmark_results.csv)')
    parser.add_argument('-o', '--output-dir', default='benchmark_plots',
                        help='Output directory for plots (default: benchmark_plots)')
    parser.add_argument('--no-show', action='store_true',
                        help='Do not display plots interactively')
    args = parser.parse_args()

    csv_path = Path(args.csv_file)
    if not csv_path.exists():
        print(f"Error: CSV file not found: {csv_path}")
        print("Run the benchmark first to generate the CSV file.")
        return 1

    print(f"Loading data from {csv_path}...")
    df = load_data(csv_path)
    print(f"Loaded {len(df)} records")

    # Print summary stats
    print_summary_stats(df)

    # Generate plots
    output_dir = Path(args.output_dir)
    print(f"\nGenerating plots in {output_dir}/...")
    generate_summary_report(df, output_dir)

    print(f"\nDone! Plots saved to {output_dir}/")
    return 0


if __name__ == '__main__':
    exit(main())
