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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) admission: Vec<AdmissionResult>,
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
    pub(crate) threads: usize,
    pub(crate) cold_cache: bool,
}

#[derive(Serialize)]
pub(crate) struct LatencyResult {
    pub(crate) benchmark: String,
    pub(crate) size: Option<usize>,
    pub(crate) batch_size: Option<usize>,
    pub(crate) cycles: LatencyStats,
    pub(crate) ns: LatencyStats,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) timer: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tsc_ticks: Option<LatencyStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) wall_ns: Option<LatencyStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) core_cycles: Option<LatencyStats>,
}

impl LatencyResult {
    pub(crate) fn basic(
        benchmark: impl Into<String>,
        size: Option<usize>,
        batch_size: Option<usize>,
        cycles: LatencyStats,
        ns: LatencyStats,
    ) -> Self {
        Self {
            benchmark: benchmark.into(),
            size,
            batch_size,
            cycles,
            ns,
            timer: None,
            tsc_ticks: None,
            wall_ns: None,
            core_cycles: None,
        }
    }

    pub(crate) fn with_tsc_ticks(
        benchmark: impl Into<String>,
        timer: &'static str,
        batch_size: usize,
        tsc_ticks: LatencyStats,
        ns: LatencyStats,
    ) -> Self {
        Self::submit_timer_row(
            benchmark,
            timer,
            batch_size,
            tsc_ticks.clone(),
            ns,
            Some(tsc_ticks),
            None,
            None,
        )
    }

    pub(crate) fn with_wall_ns(
        benchmark: impl Into<String>,
        timer: &'static str,
        batch_size: usize,
        wall_ns: LatencyStats,
    ) -> Self {
        Self::submit_timer_row(
            benchmark,
            timer,
            batch_size,
            wall_ns.clone(),
            wall_ns.clone(),
            None,
            Some(wall_ns),
            None,
        )
    }

    pub(crate) fn with_core_cycles(
        benchmark: impl Into<String>,
        timer: &'static str,
        batch_size: usize,
        core_cycles: LatencyStats,
    ) -> Self {
        Self::submit_timer_row(
            benchmark,
            timer,
            batch_size,
            core_cycles.clone(),
            core_cycles.clone(),
            None,
            None,
            Some(core_cycles),
        )
    }

    fn submit_timer_row(
        benchmark: impl Into<String>,
        timer: &'static str,
        batch_size: usize,
        cycles: LatencyStats,
        ns: LatencyStats,
        tsc_ticks: Option<LatencyStats>,
        wall_ns: Option<LatencyStats>,
        core_cycles: Option<LatencyStats>,
    ) -> Self {
        Self {
            benchmark: benchmark.into(),
            size: None,
            batch_size: Some(batch_size),
            cycles,
            ns,
            timer: Some(timer),
            tsc_ticks,
            wall_ns,
            core_cycles,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(value: u64) -> LatencyStats {
        LatencyStats {
            min: value,
            median: value,
            mean: value,
            p99: value,
            p999: value,
            cv: 0.0,
        }
    }

    #[test]
    fn submit_timer_rows_preserve_legacy_alias_fields() {
        let tsc =
            LatencyResult::with_tsc_ticks("submit_only_unloaded", "tsc", 64, stats(10), stats(20));
        assert_eq!(tsc.cycles.median, 10);
        assert_eq!(tsc.ns.median, 20);
        assert_eq!(tsc.tsc_ticks.as_ref().map(|s| s.median), Some(10));
        assert!(tsc.wall_ns.is_none());
        assert!(tsc.core_cycles.is_none());

        let wall = LatencyResult::with_wall_ns("submit_only_unloaded", "wall", 64, stats(30));
        assert_eq!(wall.cycles.median, 30);
        assert_eq!(wall.ns.median, 30);
        assert_eq!(wall.wall_ns.as_ref().map(|s| s.median), Some(30));
        assert!(wall.tsc_ticks.is_none());
        assert!(wall.core_cycles.is_none());

        let core = LatencyResult::with_core_cycles("submit_only_unloaded", "rdpmc", 64, stats(40));
        assert_eq!(core.cycles.median, 40);
        assert_eq!(core.ns.median, 40);
        assert_eq!(core.core_cycles.as_ref().map(|s| s.median), Some(40));
        assert!(core.tsc_ticks.is_none());
        assert!(core.wall_ns.is_none());
    }
}

#[derive(Serialize)]
pub(crate) struct AdmissionResult {
    pub(crate) benchmark: String,
    pub(crate) burst_size: usize,
    pub(crate) submitted: usize,
    pub(crate) completed: LatencyStats,
    pub(crate) missing: LatencyStats,
    pub(crate) errors: LatencyStats,
    pub(crate) submit_tsc_ticks: LatencyStats,
    pub(crate) submit_ns: LatencyStats,
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
