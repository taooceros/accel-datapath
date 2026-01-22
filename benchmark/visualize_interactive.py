#!/usr/bin/env python3
"""
Interactive visualization for DSA benchmark results using Plotly.
Generates a single HTML file with checkbox filters for all dimensions.
"""

import pandas as pd
import plotly.graph_objects as go
from plotly.subplots import make_subplots
import json
import argparse
from pathlib import Path

# Colorblind-friendly palette (for queue types)
COLORS = ['#1f77b4', '#ff7f0e', '#2ca02c', '#d62728', '#9467bd', '#8c564b', '#e377c2']
# Line styles for patterns
LINE_STYLES = ['solid', 'dash', 'dot', 'dashdot']
# Marker symbols for concurrency
MARKERS = ['circle', 'square', 'diamond', 'triangle-up', 'cross', 'x']


def format_size(size: int) -> str:
    """Format byte size to human-readable string."""
    if size >= 1024 * 1024:
        return f"{size // (1024 * 1024)}MB"
    elif size >= 1024:
        return f"{size // 1024}KB"
    else:
        return f"{size}B"


def load_data(csv_path: str) -> pd.DataFrame:
    """Load benchmark results from CSV."""
    df = pd.read_csv(csv_path)
    df['msg_size_label'] = df['msg_size'].apply(format_size)
    return df


def create_checkbox_dashboard(df: pd.DataFrame, output_path: Path):
    """Create an interactive HTML dashboard with checkbox filters."""

    patterns = sorted(df['pattern'].unique().tolist())
    modes = sorted(df['polling_mode'].unique().tolist())
    concurrencies = sorted(df['concurrency'].unique().tolist())
    queue_types = sorted(df['queue_type'].unique().tolist())

    # Create figure with subplots
    fig = make_subplots(
        rows=2, cols=2,
        subplot_titles=(
            'Bandwidth vs Message Size',
            'Message Rate vs Message Size',
            'Average Latency vs Message Size',
            'P99 Latency vs Message Size'
        ),
        vertical_spacing=0.12,
        horizontal_spacing=0.1
    )

    # Track trace metadata for JavaScript filtering
    trace_metadata = []

    for pi, pattern in enumerate(patterns):
        for mi, mode in enumerate(modes):
            for ci, conc in enumerate(concurrencies):
                filtered = df[(df['pattern'] == pattern) &
                             (df['polling_mode'] == mode) &
                             (df['concurrency'] == conc)]

                for qi, qt in enumerate(queue_types):
                    qt_data = filtered[filtered['queue_type'] == qt].sort_values('msg_size')
                    if len(qt_data) == 0:
                        continue

                    # Color by queue type
                    color = COLORS[qi % len(COLORS)]
                    # Line style by pattern
                    line_style = LINE_STYLES[pi % len(LINE_STYLES)]
                    # Marker by concurrency
                    marker_symbol = MARKERS[ci % len(MARKERS)]
                    # Opacity by mode (inline=1.0, threaded=0.6)
                    opacity = 1.0 if mi == 0 else 0.7

                    # Create unique label for this trace
                    trace_label = f'{qt} ({pattern}/{mode}/c={conc})'

                    # Start with first combination visible
                    visible = bool(pattern == patterns[0] and
                                   mode == modes[0] and
                                   conc == concurrencies[0])

                    meta = {
                        'pattern': pattern,
                        'mode': mode,
                        'concurrency': int(conc),
                        'queue_type': qt,
                        'label': trace_label
                    }

                    # Bandwidth
                    fig.add_trace(go.Scatter(
                        x=qt_data['msg_size'].tolist(),
                        y=qt_data['bandwidth_gbps'].tolist(),
                        mode='lines+markers',
                        name=trace_label,
                        line=dict(color=color, dash=line_style, width=2),
                        marker=dict(size=7, symbol=marker_symbol, opacity=opacity),
                        opacity=opacity,
                        visible=visible,
                        showlegend=visible,
                        legendgroup=trace_label,
                        hovertemplate=f'{trace_label}<br>%{{x}}B: %{{y:.2f}} GB/s<extra></extra>'
                    ), row=1, col=1)
                    trace_metadata.append({**meta, 'subplot': 'bandwidth'})

                    # Message Rate
                    fig.add_trace(go.Scatter(
                        x=qt_data['msg_size'].tolist(),
                        y=qt_data['msg_rate_mps'].tolist(),
                        mode='lines+markers',
                        name=trace_label,
                        line=dict(color=color, dash=line_style, width=2),
                        marker=dict(size=7, symbol=marker_symbol, opacity=opacity),
                        opacity=opacity,
                        visible=visible,
                        showlegend=False,
                        legendgroup=trace_label,
                        hovertemplate=f'{trace_label}<br>%{{x}}B: %{{y:.2f}} M/s<extra></extra>'
                    ), row=1, col=2)
                    trace_metadata.append({**meta, 'subplot': 'msg_rate'})

                    # Avg Latency
                    fig.add_trace(go.Scatter(
                        x=qt_data['msg_size'].tolist(),
                        y=(qt_data['latency_avg_ns'] / 1000).tolist(),
                        mode='lines+markers',
                        name=trace_label,
                        line=dict(color=color, dash=line_style, width=2),
                        marker=dict(size=7, symbol=marker_symbol, opacity=opacity),
                        opacity=opacity,
                        visible=visible,
                        showlegend=False,
                        legendgroup=trace_label,
                        hovertemplate=f'{trace_label}<br>%{{x}}B: %{{y:.2f}} us<extra></extra>'
                    ), row=2, col=1)
                    trace_metadata.append({**meta, 'subplot': 'latency_avg'})

                    # P99 Latency
                    fig.add_trace(go.Scatter(
                        x=qt_data['msg_size'].tolist(),
                        y=(qt_data['latency_p99_ns'] / 1000).tolist(),
                        mode='lines+markers',
                        name=trace_label,
                        line=dict(color=color, dash=line_style, width=2),
                        marker=dict(size=7, symbol=marker_symbol, opacity=opacity),
                        opacity=opacity,
                        visible=visible,
                        showlegend=False,
                        legendgroup=trace_label,
                        hovertemplate=f'{trace_label}<br>%{{x}}B: %{{y:.2f}} us<extra></extra>'
                    ), row=2, col=2)
                    trace_metadata.append({**meta, 'subplot': 'latency_p99'})

    fig.update_layout(
        title=dict(text='DSA Benchmark Results', x=0.5, xanchor='center'),
        height=700,
        legend=dict(
            orientation='v',
            yanchor='top', y=1,
            xanchor='left', x=1.02,
            font=dict(size=10),
            bgcolor='rgba(255,255,255,0.8)'
        ),
        hovermode='x unified',
        margin=dict(t=80, b=80, r=200)
    )

    # Update axes
    fig.update_xaxes(type='log', title_text='Message Size (bytes)', row=1, col=1)
    fig.update_xaxes(type='log', title_text='Message Size (bytes)', row=1, col=2)
    fig.update_xaxes(type='log', title_text='Message Size (bytes)', row=2, col=1)
    fig.update_xaxes(type='log', title_text='Message Size (bytes)', row=2, col=2)

    fig.update_yaxes(title_text='Bandwidth (GB/s)', row=1, col=1)
    fig.update_yaxes(title_text='Message Rate (M/s)', row=1, col=2)
    fig.update_yaxes(title_text='Avg Latency (us)', row=2, col=1)
    fig.update_yaxes(title_text='P99 Latency (us)', row=2, col=2)

    # Generate the plotly div
    plot_div = fig.to_html(full_html=False, include_plotlyjs='cdn', div_id='plotDiv')

    # Create HTML with checkbox controls
    html_content = f'''<!DOCTYPE html>
<html>
<head>
    <title>DSA Benchmark Dashboard</title>
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
            margin-bottom: 20px;
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
            min-width: 150px;
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
        <h1>DSA Benchmark Dashboard</h1>

        <div class="controls">
            <div class="filter-group">
                <h3>Pattern</h3>
                {generate_checkboxes('pattern', patterns)}
                <div class="btn-group">
                    <button onclick="selectAll('pattern')">All</button>
                    <button onclick="selectNone('pattern')">None</button>
                </div>
            </div>

            <div class="filter-group">
                <h3>Polling Mode</h3>
                {generate_checkboxes('mode', modes)}
                <div class="btn-group">
                    <button onclick="selectAll('mode')">All</button>
                    <button onclick="selectNone('mode')">None</button>
                </div>
            </div>

            <div class="filter-group">
                <h3>Concurrency</h3>
                {generate_checkboxes('concurrency', [str(c) for c in concurrencies])}
                <div class="btn-group">
                    <button onclick="selectAll('concurrency')">All</button>
                    <button onclick="selectNone('concurrency')">None</button>
                </div>
            </div>

            <div class="filter-group">
                <h3>Queue Type</h3>
                {generate_checkboxes('queue_type', queue_types)}
                <div class="btn-group">
                    <button onclick="selectAll('queue_type')">All</button>
                    <button onclick="selectNone('queue_type')">None</button>
                </div>
            </div>
        </div>

        <div class="plot-container">
            {plot_div}
        </div>
    </div>

    <script>
        const traceMetadata = {json.dumps(trace_metadata)};
        const queueTypes = {json.dumps(queue_types)};
        const colors = {json.dumps(COLORS)};

        function getCheckedValues(name) {{
            const checkboxes = document.querySelectorAll(`input[name="${{name}}"]:checked`);
            return Array.from(checkboxes).map(cb => cb.value);
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
            const selectedPatterns = getCheckedValues('pattern');
            const selectedModes = getCheckedValues('mode');
            const selectedConcurrencies = getCheckedValues('concurrency').map(Number);
            const selectedQueueTypes = getCheckedValues('queue_type');

            const visibility = traceMetadata.map(meta => {{
                return selectedPatterns.includes(meta.pattern) &&
                       selectedModes.includes(meta.mode) &&
                       selectedConcurrencies.includes(meta.concurrency) &&
                       selectedQueueTypes.includes(meta.queue_type);
            }});

            // Show legend for first visible trace of each unique combination (bandwidth subplot only)
            const seenLabels = new Set();
            const showLegend = traceMetadata.map((meta, i) => {{
                if (visibility[i] && meta.subplot === 'bandwidth' && !seenLabels.has(meta.label)) {{
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

        // Add event listeners to all checkboxes
        document.querySelectorAll('.controls input[type="checkbox"]').forEach(cb => {{
            cb.addEventListener('change', updatePlot);
        }});
    </script>
</body>
</html>
'''

    with open(output_path, 'w') as f:
        f.write(html_content)

    print(f"Saved interactive dashboard to: {output_path}")


def generate_checkboxes(name: str, values: list, default_first: bool = True) -> str:
    """Generate HTML checkbox inputs."""
    html = []
    for i, val in enumerate(values):
        checked = 'checked' if (default_first and i == 0) else ''
        html.append(f'<label><input type="checkbox" name="{name}" value="{val}" {checked}> {val}</label>')
    return '\n                '.join(html)


def create_heatmap_dashboard(df: pd.DataFrame, output_path: Path):
    """Create an interactive heatmap dashboard with checkbox filters."""

    patterns = sorted(df['pattern'].unique().tolist())
    modes = sorted(df['polling_mode'].unique().tolist())
    queue_types = sorted(df['queue_type'].unique().tolist())

    metrics = [
        ('bandwidth_gbps', 'Bandwidth (GB/s)', 1),
        ('msg_rate_mps', 'Message Rate (M/s)', 1),
        ('latency_avg_ns', 'Avg Latency (us)', 1/1000),
    ]

    fig = go.Figure()
    trace_metadata = []

    for pattern in patterns:
        for mode in modes:
            for qt in queue_types:
                for metric, label, scale in metrics:
                    filtered = df[(df['pattern'] == pattern) &
                                 (df['polling_mode'] == mode) &
                                 (df['queue_type'] == qt)]

                    if len(filtered) == 0:
                        continue

                    pivot = filtered.pivot(index='concurrency', columns='msg_size', values=metric)
                    pivot = pivot * scale

                    visible = bool(pattern == patterns[0] and
                                   mode == modes[0] and
                                   qt == queue_types[0] and
                                   metric == 'bandwidth_gbps')

                    fig.add_trace(go.Heatmap(
                        z=pivot.values.tolist(),
                        x=[format_size(int(c)) for c in pivot.columns],
                        y=[f'c={int(c)}' for c in pivot.index],
                        colorscale='Viridis',
                        visible=visible,
                        hovertemplate='msg=%{x}<br>%{y}<br>value=%{z:.2f}<extra></extra>',
                        colorbar=dict(title=label)
                    ))
                    trace_metadata.append({
                        'pattern': pattern,
                        'mode': mode,
                        'queue_type': qt,
                        'metric': metric,
                        'metric_label': label
                    })

    fig.update_layout(
        title=dict(text='DSA Benchmark Heatmap', x=0.5, xanchor='center'),
        xaxis_title='Message Size',
        yaxis_title='Concurrency',
        height=500
    )

    plot_div = fig.to_html(full_html=False, include_plotlyjs='cdn', div_id='heatmapDiv')

    html_content = f'''<!DOCTYPE html>
<html>
<head>
    <title>DSA Benchmark Heatmap</title>
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
            margin-bottom: 20px;
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
        <h1>DSA Benchmark Heatmap</h1>

        <div class="controls">
            <div class="filter-group">
                <h3>Pattern</h3>
                {generate_radios('pattern', patterns)}
            </div>

            <div class="filter-group">
                <h3>Polling Mode</h3>
                {generate_radios('mode', modes)}
            </div>

            <div class="filter-group">
                <h3>Queue Type</h3>
                {generate_radios('queue_type', queue_types)}
            </div>

            <div class="filter-group">
                <h3>Metric</h3>
                {generate_radios('metric', [m[0] for m in metrics], [m[1] for m in metrics])}
            </div>
        </div>

        <div class="plot-container">
            {plot_div}
        </div>
    </div>

    <script>
        const traceMetadata = {json.dumps(trace_metadata)};

        function getSelectedValue(name) {{
            const radio = document.querySelector(`input[name="${{name}}"]:checked`);
            return radio ? radio.value : null;
        }}

        function updateHeatmap() {{
            const selectedPattern = getSelectedValue('pattern');
            const selectedMode = getSelectedValue('mode');
            const selectedQueueType = getSelectedValue('queue_type');
            const selectedMetric = getSelectedValue('metric');

            const visibility = traceMetadata.map(meta => {{
                return meta.pattern === selectedPattern &&
                       meta.mode === selectedMode &&
                       meta.queue_type === selectedQueueType &&
                       meta.metric === selectedMetric;
            }});

            // Find the selected metric label for colorbar
            const selectedMeta = traceMetadata.find((meta, i) => visibility[i]);
            const title = selectedMeta ?
                `${{selectedPattern}} / ${{selectedMode}} / ${{selectedQueueType}} - ${{selectedMeta.metric_label}}` :
                'DSA Benchmark Heatmap';

            Plotly.restyle('heatmapDiv', {{ visible: visibility }});
            Plotly.relayout('heatmapDiv', {{ 'title.text': title }});
        }}

        // Add event listeners
        document.querySelectorAll('.controls input[type="radio"]').forEach(r => {{
            r.addEventListener('change', updateHeatmap);
        }});
    </script>
</body>
</html>
'''

    with open(output_path, 'w') as f:
        f.write(html_content)

    print(f"Saved heatmap dashboard to: {output_path}")


def generate_radios(name: str, values: list, labels: list = None) -> str:
    """Generate HTML radio inputs."""
    if labels is None:
        labels = values
    html = []
    for i, (val, label) in enumerate(zip(values, labels)):
        checked = 'checked' if i == 0 else ''
        html.append(f'<label><input type="radio" name="{name}" value="{val}" {checked}> {label}</label>')
    return '\n                '.join(html)


def main():
    parser = argparse.ArgumentParser(description='Interactive DSA benchmark visualization')
    parser.add_argument('csv_file', nargs='?', default='dsa_benchmark_results.csv',
                        help='Path to CSV file')
    parser.add_argument('-o', '--output-dir', default='benchmark_plots',
                        help='Output directory')
    args = parser.parse_args()

    csv_path = Path(args.csv_file)
    if not csv_path.exists():
        print(f"Error: CSV file not found: {csv_path}")
        return 1

    print(f"Loading data from {csv_path}...")
    df = load_data(csv_path)
    print(f"Loaded {len(df)} records")

    output_dir = Path(args.output_dir)
    output_dir.mkdir(exist_ok=True)

    create_checkbox_dashboard(df, output_dir / 'dashboard.html')
    create_heatmap_dashboard(df, output_dir / 'heatmap.html')

    print(f"\nOpen in browser:")
    print(f"  {output_dir / 'dashboard.html'}")
    print(f"  {output_dir / 'heatmap.html'}")
    return 0


if __name__ == '__main__':
    exit(main())
