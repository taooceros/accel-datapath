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

pub(crate) fn stats_from_values(mut values: Vec<u64>) -> LatencyStats {
    values.sort_unstable();
    compute_stats(&values)
}

#[derive(Default)]
pub(crate) struct SubmissionBottleneckResults {
    pub(crate) admission: Vec<AdmissionResult>,
    pub(crate) submit_occupancy: Vec<SubmitOccupancyResult>,
    pub(crate) submit_marker_overlap: Vec<SubmitMarkerOverlapResult>,
    pub(crate) submit_marker_mechanism: Vec<SubmitMarkerMechanismResult>,
    pub(crate) traffic_class_ladder: Vec<TrafficClassLadderResult>,
    pub(crate) completion_reuse_policy: Vec<CompletionReusePolicyResult>,
}

#[derive(Serialize)]
pub(crate) struct FullReport {
    pub(crate) metadata: Metadata,
    pub(crate) latency: Vec<LatencyResult>,
    pub(crate) throughput: Vec<ThroughputResult>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) admission: Vec<AdmissionResult>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) submit_occupancy: Vec<SubmitOccupancyResult>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) submit_marker_overlap: Vec<SubmitMarkerOverlapResult>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) submit_marker_mechanism: Vec<SubmitMarkerMechanismResult>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) traffic_class_ladder: Vec<TrafficClassLadderResult>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) completion_reuse_policy: Vec<CompletionReusePolicyResult>,
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
pub(crate) struct SubmitOccupancyResult {
    pub(crate) benchmark: String,
    pub(crate) operation_class: String,
    pub(crate) k_prefill: usize,
    pub(crate) submitted: usize,
    pub(crate) completed: LatencyStats,
    pub(crate) missing: LatencyStats,
    pub(crate) errors: LatencyStats,
    pub(crate) extra_submit_tsc_ticks: LatencyStats,
    pub(crate) extra_submit_ns: LatencyStats,
    pub(crate) first_old_completion_tsc_ticks: Option<LatencyStats>,
    pub(crate) first_old_completion_ns: Option<LatencyStats>,
}

#[derive(Serialize)]
pub(crate) struct SubmitMarkerCompletionTrace {
    pub(crate) completion_index: usize,
    pub(crate) none_count: u64,
    pub(crate) success_count: u64,
    pub(crate) success_fraction: f64,
    pub(crate) error_count: u64,
    pub(crate) none_poll_tsc_ticks: Option<LatencyStats>,
    pub(crate) none_poll_ns: Option<LatencyStats>,
    pub(crate) success_poll_tsc_ticks: Option<LatencyStats>,
    pub(crate) success_poll_ns: Option<LatencyStats>,
    pub(crate) error_poll_tsc_ticks: Option<LatencyStats>,
    pub(crate) error_poll_ns: Option<LatencyStats>,
}

#[derive(Serialize)]
pub(crate) struct SubmitMarkerRequestCompletionTrace {
    pub(crate) request_index: usize,
    pub(crate) observed_count: u64,
    pub(crate) observed_fraction: f64,
    pub(crate) success_count: u64,
    pub(crate) error_count: u64,
    pub(crate) observed_after_submit_index: Option<LatencyStats>,
    pub(crate) completion_tsc_ticks: Option<LatencyStats>,
    pub(crate) completion_ns: Option<LatencyStats>,
    pub(crate) observed_from_marker_tsc: Option<LatencyStats>,
}

#[derive(Serialize)]
pub(crate) struct SubmitMarkerSampleTrace {
    pub(crate) iteration_index: usize,
    pub(crate) points: Vec<SubmitMarkerSampleTracePoint>,
}

#[derive(Serialize)]
pub(crate) struct SubmitMarkerSampleTracePoint {
    pub(crate) submit_index: usize,
    pub(crate) submit_tsc_ticks: u64,
    pub(crate) submit_start_from_marker_tsc: Option<u64>,
    pub(crate) submit_end_from_marker_tsc: Option<u64>,
    pub(crate) poll_performed: bool,
    pub(crate) poll_end_from_marker_tsc: Option<u64>,
    pub(crate) poll_window_tsc_ticks: Option<u64>,
    pub(crate) polled_request_indices: Vec<usize>,
    pub(crate) poll_latency_tsc_ticks: Vec<u64>,
    pub(crate) poll_count: u64,
    pub(crate) first_polled_request_index: Option<u64>,
    pub(crate) last_polled_request_index: Option<u64>,
    pub(crate) visible_prefix_len: u64,
    pub(crate) polled_statuses: Vec<u8>,
    pub(crate) polled_status: u8,
    pub(crate) marker_status: u8,
}

#[derive(Serialize)]
pub(crate) struct SubmitMarkerTracePoint {
    pub(crate) poll_performed: bool,
    pub(crate) submit_index: usize,
    pub(crate) submit_tsc_ticks: LatencyStats,
    pub(crate) submit_ns: LatencyStats,
    pub(crate) submit_start_from_marker_tsc: Option<LatencyStats>,
    pub(crate) submit_end_from_marker_tsc: Option<LatencyStats>,
    pub(crate) poll_end_from_marker_tsc: Option<LatencyStats>,
    pub(crate) poll_window_tsc_ticks: Option<LatencyStats>,
    pub(crate) poll_window_ns: Option<LatencyStats>,
    pub(crate) poll_latency_tsc_ticks: Option<LatencyStats>,
    pub(crate) poll_latency_ns: Option<LatencyStats>,
    pub(crate) poll_count: Option<LatencyStats>,
    pub(crate) first_polled_request_index: Option<LatencyStats>,
    pub(crate) last_polled_request_index: Option<LatencyStats>,
    pub(crate) visible_prefix_len: LatencyStats,
    pub(crate) visible_count: LatencyStats,
    pub(crate) completions: Vec<SubmitMarkerCompletionTrace>,
}

#[derive(Serialize)]
pub(crate) struct SubmitMarkerOverlapResult {
    pub(crate) benchmark: String,
    pub(crate) operation_class: String,
    pub(crate) n: usize,
    pub(crate) marker_position: usize,
    pub(crate) marker_position_label: String,
    pub(crate) poll_cadence: String,
    pub(crate) submit_tail_tsc_ticks: LatencyStats,
    pub(crate) marker_poll_offset: usize,
    pub(crate) poll_step: usize,
    pub(crate) tracked_completions: usize,
    pub(crate) submit_tail_ns: LatencyStats,
    pub(crate) marker_visible_tsc_ticks: Option<LatencyStats>,
    pub(crate) marker_visible_ns: Option<LatencyStats>,
    pub(crate) marker_observed_before_final_submit_count: u64,
    pub(crate) marker_observed_before_final_submit_fraction: f64,
    pub(crate) completed: LatencyStats,
    pub(crate) missing: LatencyStats,
    pub(crate) errors: LatencyStats,
    pub(crate) request_completions: Vec<SubmitMarkerRequestCompletionTrace>,
    pub(crate) sample_trace: Vec<SubmitMarkerSampleTrace>,
    pub(crate) trace: Vec<SubmitMarkerTracePoint>,
}

#[derive(Serialize)]
pub(crate) struct SubmitMarkerMechanismBaselineComparison {
    pub(crate) baseline: String,
    pub(crate) visible_poll_ns: Option<MedianGap>,
    pub(crate) none_poll_ns: Option<MedianGap>,
    pub(crate) line_position_0_visible_ns: Option<MedianGap>,
    pub(crate) line_position_1_visible_ns: Option<MedianGap>,
    pub(crate) same_line_first_visible_ns: Option<MedianGap>,
    pub(crate) same_line_second_visible_ns: Option<MedianGap>,
}

#[derive(Serialize)]
pub(crate) struct MedianGap {
    pub(crate) delta_ns: i64,
    pub(crate) ratio_to_baseline: f64,
}

#[derive(Serialize)]
pub(crate) struct SubmitMarkerMechanismResult {
    pub(crate) benchmark: String,
    pub(crate) sub_experiment: String,
    pub(crate) variant: String,
    pub(crate) operation_class: String,
    pub(crate) n: usize,
    pub(crate) marker_poll_offset: usize,
    pub(crate) completion_layout: String,
    pub(crate) completion_stride_bytes: usize,
    pub(crate) completion_alignment_bytes: usize,
    pub(crate) completion_base_mod_64: usize,
    pub(crate) completion_base_mod_4096: usize,
    pub(crate) prefetch_distance_lines: Option<usize>,
    pub(crate) cache_state: String,
    pub(crate) timing_mode: String,
    pub(crate) submitted: u64,
    pub(crate) completed: LatencyStats,
    pub(crate) missing: LatencyStats,
    pub(crate) errors: LatencyStats,
    pub(crate) none_poll_tsc_ticks: Option<LatencyStats>,
    pub(crate) none_poll_ns: Option<LatencyStats>,
    pub(crate) visible_poll_tsc_ticks: Option<LatencyStats>,
    pub(crate) visible_poll_ns: Option<LatencyStats>,
    pub(crate) line_position_0_visible_tsc_ticks: Option<LatencyStats>,
    pub(crate) line_position_0_visible_ns: Option<LatencyStats>,
    pub(crate) line_position_1_visible_tsc_ticks: Option<LatencyStats>,
    pub(crate) line_position_1_visible_ns: Option<LatencyStats>,
    pub(crate) same_line_first_visible_tsc_ticks: Option<LatencyStats>,
    pub(crate) same_line_first_visible_ns: Option<LatencyStats>,
    pub(crate) same_line_second_visible_tsc_ticks: Option<LatencyStats>,
    pub(crate) same_line_second_visible_ns: Option<LatencyStats>,
    pub(crate) poll_window_tsc_ticks: Option<LatencyStats>,
    pub(crate) poll_window_ns: Option<LatencyStats>,
    pub(crate) poll_window_reads: Option<LatencyStats>,
    pub(crate) poll_window_visible: Option<LatencyStats>,
    pub(crate) sample_trace: Vec<SubmitMarkerSampleTrace>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) baseline_comparison: Option<SubmitMarkerMechanismBaselineComparison>,
}

#[derive(Serialize)]
pub(crate) struct TrafficClassLadderResult {
    pub(crate) benchmark: String,
    pub(crate) traffic_class: String,
    pub(crate) operation_size: Option<usize>,
    pub(crate) window: usize,
    pub(crate) submit_tsc_ticks: LatencyStats,
    pub(crate) submit_ns: LatencyStats,
    pub(crate) completion_visible_tsc_ticks: Option<LatencyStats>,
    pub(crate) completion_visible_ns: Option<LatencyStats>,
    pub(crate) completed: Option<LatencyStats>,
    pub(crate) missing: Option<LatencyStats>,
    pub(crate) errors: Option<LatencyStats>,
    pub(crate) ops_per_sec: LatencyStats,
}

#[derive(Serialize)]
pub(crate) struct CompletionReusePolicyResult {
    pub(crate) benchmark: String,
    pub(crate) operation_class: String,
    pub(crate) window: usize,
    pub(crate) policy: String,
    pub(crate) operations_completed: u64,
    pub(crate) ops_per_sec: f64,
    pub(crate) polls_per_completion: f64,
    pub(crate) completion_harvest_tsc_ticks: LatencyStats,
    pub(crate) completion_harvest_ns: LatencyStats,
    pub(crate) reset_to_submit_tsc_ticks: Option<LatencyStats>,
    pub(crate) reset_to_submit_ns: Option<LatencyStats>,
    pub(crate) completed: u64,
    pub(crate) missing: u64,
    pub(crate) errors: u64,
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
