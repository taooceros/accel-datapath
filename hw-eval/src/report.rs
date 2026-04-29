use serde::Serialize;

#[derive(Serialize, Clone)]
pub(crate) struct LatencyStats {
    pub(crate) min: u64,
    pub(crate) median: u64,
    pub(crate) mean: u64,
    pub(crate) p99: u64,
    pub(crate) p999: u64,
    pub(crate) cv: f64,
}

pub(crate) fn compute_stats(sorted: &[u64]) -> LatencyStats {
    let n = sorted.len();
    let min = sorted[0];
    let median = sorted[n / 2];
    let sum: u64 = sorted.iter().sum();
    let mean = sum / n as u64;
    let p99 = sorted[(n as f64 * 0.99) as usize];
    let p999 = sorted[((n as f64 * 0.999) as usize).min(n - 1)];

    let mean_f = sum as f64 / n as f64;
    let variance: f64 = sorted
        .iter()
        .map(|&v| {
            let d = v as f64 - mean_f;
            d * d
        })
        .sum::<f64>()
        / n as f64;
    let cv = if mean_f > 0.0 {
        variance.sqrt() / mean_f
    } else {
        0.0
    };

    LatencyStats {
        min,
        median,
        mean,
        p99,
        p999,
        cv,
    }
}

#[derive(Serialize)]
pub(crate) struct FullReport {
    pub(crate) metadata: Metadata,
    pub(crate) latency: Vec<LatencyResult>,
    pub(crate) throughput: Vec<ThroughputResult>,
}

#[derive(Serialize)]
pub(crate) struct Metadata {
    pub(crate) accelerator: String,
    pub(crate) tsc_freq_hz: u64,
    pub(crate) pinned_core: usize,
    pub(crate) cpu_numa_node: Option<usize>,
    pub(crate) device_numa_node: Option<i32>,
    pub(crate) device: String,
    pub(crate) wq_dedicated: Option<bool>,
    pub(crate) iterations: usize,
    pub(crate) cold_cache: bool,
}

#[derive(Serialize)]
pub(crate) struct LatencyResult {
    pub(crate) benchmark: String,
    pub(crate) size: Option<usize>,
    pub(crate) batch_size: Option<usize>,
    pub(crate) cycles: LatencyStats,
    pub(crate) ns: LatencyStats,
}

#[derive(Serialize)]
pub(crate) struct ThroughputResult {
    pub(crate) benchmark: String,
    pub(crate) size: usize,
    pub(crate) concurrency: usize,
    pub(crate) ops_per_sec: f64,
    pub(crate) bandwidth_mb_s: f64,
}

pub(crate) fn print_json_report(report: &FullReport) -> Result<(), crate::HwEvalError> {
    let rendered = serde_json::to_string_pretty(report)
        .map_err(|source| crate::HwEvalError::SerializeReport { source })?;
    println!("{rendered}");
    Ok(())
}
