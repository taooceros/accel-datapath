// Experiment 2: traced completion visibility across the submit wall.
//
// Submit N logical operations one per MMIO. Time every submit, and after a
// configurable submit-index offset immediately poll the next unfinished completion.
//
//   submit order:
//   [0: marker] [1] [2] [3] ... [poll_offset] [poll_offset+1] ... [N-1]
//       ^ trace submit latency at every index   ^ poll frontier comp[j]
//
// Question: while the submit path approaches the admission wall, what does a
// single completion-memory status read cost?

use hw_eval::dsa::{
    completion_flags_no_cache_control, poll_completion, reset_completion, DsaCompletionRecord,
    DsaCompletionStatus, DsaHwDesc, DSA_COMP_NONE, DSA_COMP_SUCCESS,
};
use hw_eval::submit::{cycles_to_ns, mfence, rdtscp, WqPortal};

use crate::config::{DsaOperationClass, MarkerPosition};
use crate::report::{
    stats_from_values, SubmitMarkerOverlapResult, SubmitMarkerRequestCompletionTrace,
    SubmitMarkerSampleTrace, SubmitMarkerSampleTracePoint, SubmitMarkerTracePoint,
};

use super::super::common::{
    count_visible_completions, optional_stats, reset_sample_completions, OperationSlots,
    COMPLETION_TIMEOUT_NS, TIMEOUT_CHECK_STRIDE,
};

const SUBMIT_MARKER_OVERLAP_BENCHMARK: &str = "submit_marker_overlap";
const POLL_STEP: usize = 1;

pub(crate) fn bench_submit_marker_overlap(
    wq: &WqPortal,
    bursts: &[usize],
    positions: &[MarkerPosition],
    poll_offsets: &[usize],
    operation: DsaOperationClass,
    iterations: usize,
    tsc_freq: u64,
    json: bool,
    results: &mut Vec<SubmitMarkerOverlapResult>,
) {
    let Some(&max_burst) = bursts.iter().max() else {
        return;
    };
    let mut slots = OperationSlots::new(max_burst, operation);
    let mut seen = vec![false; max_burst];
    let mut submit_start_tscs = vec![0_u64; max_burst];
    let mut observed_tscs = vec![0_u64; max_burst];
    let mut submit_end_tscs = vec![0_u64; max_burst];
    let mut observed_statuses = vec![DSA_COMP_NONE; max_burst];
    let mut observed_after_submit_indices = vec![0_usize; max_burst];
    let mut observed_from_marker_tscs = vec![None; max_burst];

    let mut drain_desc = DsaHwDesc::default();
    let mut drain_comp = DsaCompletionRecord::default();
    drain_desc.fill_drain(completion_flags_no_cache_control());
    drain_desc.set_completion(&mut drain_comp);

    if !json {
        println!(
            "\n=== {SUBMIT_MARKER_OVERLAP_BENCHMARK} traced ({}) ===",
            operation.as_str()
        );
        println!(
            "{:>8} {:>8} {:>8} {:>12} {:>14} {:>14} {:>10}",
            "n", "marker", "offset", "points", "submit_ns", "marker_ns", "completed"
        );
    }

    for &n in bursts {
        for &position in positions {
            let marker_position = position.to_index(n);

            for &poll_offset in poll_offsets {
                let mut submit_tail_tsc = Vec::with_capacity(iterations);
                let mut marker_visible_tsc = Vec::with_capacity(iterations);
                let mut completed_counts = Vec::with_capacity(iterations);
                let mut missing_counts = Vec::with_capacity(iterations);
                let mut error_counts = Vec::with_capacity(iterations);
                let mut observed_before_final_submit = 0_u64;

                let mut request_accumulators = request_completion_accumulators(n, iterations);
                let mut trace_accumulators = trace_accumulators(n, poll_offset, iterations);
                let mut sample_trace = Vec::with_capacity(iterations);
                let max_poll_reads_per_iteration =
                    n.saturating_add(n.saturating_sub(poll_offset.min(n)));
                let mut poll_result = CompletionPollResult::with_capacity(n);
                let mut poll_event_offsets = vec![0_usize; n];
                let mut poll_event_counts = vec![0_usize; n];
                let mut poll_visible_prefix_lens = vec![0_u64; n];
                let mut poll_event_request_indices =
                    Vec::with_capacity(max_poll_reads_per_iteration);
                let mut poll_event_statuses = Vec::with_capacity(max_poll_reads_per_iteration);
                let mut poll_event_latency_tsc_ticks =
                    Vec::with_capacity(max_poll_reads_per_iteration);

                for iteration_index in 0..iterations {
                    reset_sample_completions(&mut slots.completions[..n]);
                    submit_start_tscs[..n].fill(0);
                    submit_end_tscs[..n].fill(0);
                    observed_tscs[..n].fill(0);
                    observed_statuses[..n].fill(DSA_COMP_NONE);
                    observed_after_submit_indices[..n].fill(0);
                    observed_from_marker_tscs[..n].fill(None);
                    poll_event_offsets.fill(0);
                    poll_event_counts.fill(0);
                    poll_visible_prefix_lens.fill(0);
                    poll_event_request_indices.clear();
                    poll_event_statuses.clear();
                    poll_event_latency_tsc_ticks.clear();

                    let mut next_completion_to_poll = 0_usize;
                    let mut marker_submit_tsc = None;
                    let mut final_submit_tsc = 0_u64;

                    for index in 0..n {
                        let submit =
                            measured_call(|| unsafe { wq.submit(&slots.descriptors[index]) });
                        let submit_start_tsc = submit.start_tsc;
                        submit_start_tscs[index] = submit_start_tsc;
                        let submit_end_tsc = submit.end_tsc;
                        submit_end_tscs[index] = submit_end_tsc;

                        if index == marker_position {
                            marker_submit_tsc = Some(submit_start_tsc);
                        }
                        if index == n - 1 {
                            final_submit_tsc = submit_end_tsc;
                        }

                        trace_accumulators[index].record_submit(
                            submit_end_tsc.saturating_sub(submit_start_tsc),
                            marker_submit_tsc
                                .map(|marker_tsc| submit_start_tsc.saturating_sub(marker_tsc)),
                            marker_submit_tsc
                                .map(|marker_tsc| submit_end_tsc.saturating_sub(marker_tsc)),
                        );

                        if index >= poll_offset && next_completion_to_poll < n {
                            poll_to_next_unfinished(
                                &slots.completions[..n],
                                &mut next_completion_to_poll,
                                &mut observed_tscs[..n],
                                &mut observed_statuses[..n],
                                &mut observed_after_submit_indices[..n],
                                &mut observed_from_marker_tscs[..n],
                                index,
                                marker_submit_tsc,
                                &mut poll_result,
                            );

                            let event_start = poll_event_request_indices.len();
                            let read_count = poll_result.read_count() as usize;
                            debug_assert!(
                                event_start + read_count <= poll_event_request_indices.capacity()
                            );
                            debug_assert!(
                                event_start + read_count <= poll_event_statuses.capacity()
                            );
                            debug_assert!(
                                event_start + read_count <= poll_event_latency_tsc_ticks.capacity()
                            );

                            poll_event_offsets[index] = event_start;
                            poll_event_counts[index] = read_count;
                            poll_visible_prefix_lens[index] = poll_result.visible_prefix_len;
                            poll_event_request_indices
                                .extend_from_slice(&poll_result.request_indices);
                            poll_event_statuses.extend_from_slice(&poll_result.statuses);
                            poll_event_latency_tsc_ticks
                                .extend_from_slice(&poll_result.latency_tsc_ticks);

                            trace_accumulators[index].record_poll_summary(
                                poll_result.visible_prefix_len,
                                poll_result.read_count(),
                            );
                        } else {
                            trace_accumulators[index].record_no_poll();
                        }
                    }

                    for submit_index in 0..n {
                        let event_count = poll_event_counts[submit_index];
                        if event_count == 0 {
                            continue;
                        }

                        let event_start = poll_event_offsets[submit_index];
                        let event_end = event_start + event_count;
                        trace_accumulators[submit_index].record_poll_latencies(
                            &poll_event_latency_tsc_ticks[event_start..event_end],
                        );
                    }

                    let mut marker_observed_tsc = observed_tscs[marker_position];
                    for completion_index in 0..n {
                        if observed_tscs[completion_index] != 0 {
                            continue;
                        }

                        if let Some((completion_tsc, status)) =
                            wait_for_completion(&slots.completions[completion_index], tsc_freq)
                        {
                            observed_tscs[completion_index] = completion_tsc;
                            observed_statuses[completion_index] = status;
                            observed_after_submit_indices[completion_index] = n - 1;
                            observed_from_marker_tscs[completion_index] = marker_submit_tsc
                                .map(|marker_tsc| completion_tsc.saturating_sub(marker_tsc));
                            if completion_index == marker_position {
                                marker_observed_tsc = completion_tsc;
                            }
                        }
                    }

                    for completion_index in 0..n {
                        if observed_tscs[completion_index] == 0 {
                            continue;
                        }

                        record_request_completion(
                            completion_index,
                            observed_statuses[completion_index],
                            observed_tscs[completion_index],
                            observed_after_submit_indices[completion_index],
                            observed_from_marker_tscs[completion_index],
                            &submit_start_tscs[..n],
                            &mut request_accumulators[..n],
                        );
                    }

                    if marker_observed_tsc != 0 && marker_observed_tsc < final_submit_tsc {
                        observed_before_final_submit += 1;
                    }

                    if marker_observed_tsc != 0 {
                        marker_visible_tsc.push(
                            marker_observed_tsc.saturating_sub(submit_start_tscs[marker_position]),
                        );
                    }

                    // Trace boundary: materialize only submit-loop observations.
                    // The drain below is cleanup and must not affect per-submit trace points.
                    let mut iteration_sample_trace = Vec::with_capacity(n);
                    for submit_index in 0..n {
                        let event_start = poll_event_offsets[submit_index];
                        let event_end = event_start + poll_event_counts[submit_index];

                        let point_marker_submit_tsc = if submit_index >= marker_position {
                            marker_submit_tsc
                        } else {
                            None
                        };
                        iteration_sample_trace.push(sample_trace_point(
                            submit_index,
                            submit_start_tscs[submit_index],
                            submit_end_tscs[submit_index],
                            point_marker_submit_tsc,
                            poll_visible_prefix_lens[submit_index],
                            &poll_event_request_indices[event_start..event_end],
                            &poll_event_latency_tsc_ticks[event_start..event_end],
                            &poll_event_statuses[event_start..event_end],
                        ));
                    }

                    sample_trace.push(SubmitMarkerSampleTrace {
                        iteration_index,
                        points: iteration_sample_trace,
                    });

                    // Cleanup boundary: after this point we may force queue progress with a
                    // drain descriptor, so only aggregate end-of-iteration accounting belongs here.
                    let outcome =
                        count_visible_completions(&slots.completions[..n], &mut seen[..n]);
                    drain_with_drain_descriptor(
                        wq,
                        &drain_desc,
                        &mut drain_comp,
                        operation,
                        n,
                        poll_offset,
                    );
                    if let Some(marker_tsc) = marker_submit_tsc {
                        submit_tail_tsc.push(final_submit_tsc.saturating_sub(marker_tsc));
                    }
                    completed_counts.push(outcome.completed as u64);
                    missing_counts.push((n - outcome.completed) as u64);
                    error_counts.push(outcome.errors as u64);
                }

                let submit_tail_ns_stats = stats_from_tsc_slice(&submit_tail_tsc, tsc_freq);
                let marker_visible_ns_stats =
                    optional_stats_from_tsc_slice(&marker_visible_tsc, tsc_freq);
                let marker_visible_tsc_stats = optional_stats(marker_visible_tsc);
                let completed = stats_from_values(completed_counts);

                let request_completions = request_accumulators
                    .into_iter()
                    .map(|accumulator| accumulator.into_trace(tsc_freq, iterations))
                    .collect::<Vec<_>>();

                let trace = trace_accumulators
                    .into_iter()
                    .map(|accumulator| accumulator.into_point(tsc_freq))
                    .collect::<Vec<_>>();

                if !json {
                    let marker_ns = marker_visible_ns_stats
                        .as_ref()
                        .map(|stats| stats.median.to_string())
                        .unwrap_or_else(|| "-".to_string());
                    println!(
                        "{:>8} {:>8} {:>8} {:>12} {:>14} {:>14} {:>10}",
                        n,
                        marker_position,
                        poll_offset,
                        trace.len(),
                        submit_tail_ns_stats.median,
                        marker_ns,
                        completed.median
                    );
                }

                results.push(SubmitMarkerOverlapResult {
                    benchmark: SUBMIT_MARKER_OVERLAP_BENCHMARK.to_string(),
                    operation_class: operation.as_str().to_string(),
                    n,
                    marker_position,
                    marker_position_label: position.as_str().to_string(),
                    poll_cadence: POLL_STEP.to_string(),
                    marker_poll_offset: poll_offset,
                    poll_step: POLL_STEP,
                    tracked_completions: 0,
                    submit_tail_tsc_ticks: stats_from_values(submit_tail_tsc),
                    submit_tail_ns: submit_tail_ns_stats,
                    marker_visible_tsc_ticks: marker_visible_tsc_stats,
                    marker_visible_ns: marker_visible_ns_stats,
                    marker_observed_before_final_submit_count: observed_before_final_submit,
                    marker_observed_before_final_submit_fraction: observed_before_final_submit
                        as f64
                        / iterations as f64,
                    completed,
                    missing: stats_from_values(missing_counts),
                    errors: stats_from_values(error_counts),
                    request_completions,
                    sample_trace,
                    trace,
                });
            }
        }
    }
}

fn trace_accumulators(n: usize, poll_offset: usize, iterations: usize) -> Vec<TraceAccumulator> {
    (0..n)
        .map(|submit_index| {
            TraceAccumulator::new(submit_index, submit_index >= poll_offset, iterations)
        })
        .collect()
}

fn request_completion_accumulators(
    n: usize,
    iterations: usize,
) -> Vec<RequestCompletionAccumulator> {
    (0..n)
        .map(|request_index| RequestCompletionAccumulator::new(request_index, iterations))
        .collect()
}

fn poll_to_next_unfinished(
    completions: &[DsaCompletionRecord],
    next_completion_to_poll: &mut usize,
    observed_tscs: &mut [u64],
    observed_statuses: &mut [u8],
    observed_after_submit_indices: &mut [usize],
    observed_from_marker_tscs: &mut [Option<u64>],
    submit_index: usize,
    marker_submit_tsc: Option<u64>,
    result: &mut CompletionPollResult,
) {
    result.clear();

    while *next_completion_to_poll < completions.len() {
        let completion_index = *next_completion_to_poll;
        let poll =
            measured_call(|| DsaCompletionStatus::mask(completions[completion_index].status()));
        let status = poll.value;

        result.record_read(completion_index, status, poll.elapsed_tsc());

        if status == DSA_COMP_NONE {
            break;
        }

        observed_tscs[completion_index] = poll.end_tsc;
        observed_statuses[completion_index] = status;
        observed_after_submit_indices[completion_index] = submit_index;
        observed_from_marker_tscs[completion_index] =
            marker_submit_tsc.map(|marker_tsc| poll.end_tsc.saturating_sub(marker_tsc));
        *next_completion_to_poll += 1;
    }

    result.visible_prefix_len = *next_completion_to_poll as u64;
}

fn record_request_completion(
    completion_index: usize,
    status: u8,
    completion_tsc: u64,
    observed_after_submit_index: usize,
    observed_from_marker_tsc: Option<u64>,
    submit_start_tscs: &[u64],
    request_accumulators: &mut [RequestCompletionAccumulator],
) {
    request_accumulators[completion_index].record(
        status,
        completion_tsc.saturating_sub(submit_start_tscs[completion_index]),
        observed_after_submit_index as u64,
        observed_from_marker_tsc,
    );
}

#[derive(Clone, Copy)]
struct MeasuredCall<T> {
    value: T,
    start_tsc: u64,
    end_tsc: u64,
}

impl<T> MeasuredCall<T> {
    fn elapsed_tsc(&self) -> u64 {
        self.end_tsc.saturating_sub(self.start_tsc)
    }
}

fn measured_call<T>(call: impl FnOnce() -> T) -> MeasuredCall<T> {
    let start_tsc = rdtscp().0;
    let value = call();
    let end_tsc = rdtscp().0;

    MeasuredCall {
        value,
        start_tsc,
        end_tsc,
    }
}

fn sample_trace_point(
    submit_index: usize,
    submit_start_tsc: u64,
    submit_end_tsc: u64,
    marker_submit_tsc: Option<u64>,
    visible_prefix_len: u64,
    request_indices: &[usize],
    latency_tsc_ticks: &[u64],
    statuses: &[u8],
) -> SubmitMarkerSampleTracePoint {
    debug_assert_eq!(request_indices.len(), latency_tsc_ticks.len());
    debug_assert_eq!(request_indices.len(), statuses.len());

    let poll_count = request_indices.len() as u64;
    let last_status = statuses.last().copied().unwrap_or(DSA_COMP_NONE);

    SubmitMarkerSampleTracePoint {
        submit_index,
        submit_tsc_ticks: submit_end_tsc.saturating_sub(submit_start_tsc),
        submit_start_from_marker_tsc: marker_submit_tsc
            .map(|marker_tsc| submit_start_tsc.saturating_sub(marker_tsc)),
        submit_end_from_marker_tsc: marker_submit_tsc
            .map(|marker_tsc| submit_end_tsc.saturating_sub(marker_tsc)),
        poll_performed: poll_count != 0,
        poll_end_from_marker_tsc: None,
        poll_window_tsc_ticks: None,
        polled_request_indices: request_indices.to_vec(),
        poll_latency_tsc_ticks: latency_tsc_ticks.to_vec(),
        poll_count,
        first_polled_request_index: request_indices
            .first()
            .copied()
            .map(|request_index| request_index as u64),
        last_polled_request_index: request_indices
            .last()
            .copied()
            .map(|request_index| request_index as u64),
        visible_prefix_len,
        polled_statuses: statuses.to_vec(),
        polled_status: last_status,
        marker_status: last_status,
    }
}

struct TraceAccumulator {
    submit_index: usize,
    poll_performed: bool,
    submit_tsc: Vec<u64>,
    submit_start_from_marker_tsc: Vec<u64>,
    submit_end_from_marker_tsc: Vec<u64>,
    poll_latency_tsc: Vec<u64>,
    poll_count: Vec<u64>,
    visible_count: Vec<u64>,
}

impl TraceAccumulator {
    fn new(submit_index: usize, poll_performed: bool, iterations: usize) -> Self {
        Self {
            submit_index,
            poll_performed,
            submit_tsc: Vec::with_capacity(iterations),
            submit_start_from_marker_tsc: Vec::with_capacity(iterations),
            submit_end_from_marker_tsc: Vec::with_capacity(iterations),
            poll_latency_tsc: Vec::with_capacity(if poll_performed { iterations } else { 0 }),
            poll_count: Vec::with_capacity(if poll_performed { iterations } else { 0 }),
            visible_count: Vec::with_capacity(iterations),
        }
    }

    fn record_submit(
        &mut self,
        submit_tsc: u64,
        start_from_marker_tsc: Option<u64>,
        end_from_marker_tsc: Option<u64>,
    ) {
        self.submit_tsc.push(submit_tsc);

        if let Some(start_from_marker_tsc) = start_from_marker_tsc {
            self.submit_start_from_marker_tsc
                .push(start_from_marker_tsc);
        }

        if let Some(end_from_marker_tsc) = end_from_marker_tsc {
            self.submit_end_from_marker_tsc.push(end_from_marker_tsc);
        }
    }

    fn record_poll_summary(&mut self, visible_prefix_len: u64, read_count: u64) {
        self.visible_count.push(visible_prefix_len);
        self.poll_count.push(read_count);
    }

    fn record_no_poll(&mut self) {
        self.visible_count.push(0);
    }

    fn record_poll_latencies(&mut self, latency_tsc_ticks: &[u64]) {
        self.poll_latency_tsc.extend_from_slice(latency_tsc_ticks);
    }

    fn into_point(self, tsc_freq: u64) -> SubmitMarkerTracePoint {
        let submit_ns = stats_from_tsc_slice(&self.submit_tsc, tsc_freq);
        let poll_window_ns = None;
        let poll_latency_ns = optional_stats_from_tsc_slice(&self.poll_latency_tsc, tsc_freq);

        let visible_count = stats_from_values(self.visible_count);

        SubmitMarkerTracePoint {
            poll_performed: self.poll_performed,
            submit_index: self.submit_index,
            submit_tsc_ticks: stats_from_values(self.submit_tsc),
            submit_start_from_marker_tsc: optional_stats(self.submit_start_from_marker_tsc),
            submit_end_from_marker_tsc: optional_stats(self.submit_end_from_marker_tsc),
            poll_end_from_marker_tsc: None,
            poll_window_tsc_ticks: None,
            poll_latency_tsc_ticks: optional_stats(self.poll_latency_tsc),
            poll_latency_ns,
            poll_count: optional_stats(self.poll_count),
            first_polled_request_index: None,
            last_polled_request_index: None,
            poll_window_ns,
            submit_ns,
            visible_prefix_len: visible_count.clone(),
            visible_count,
            completions: Vec::new(),
        }
    }
}

struct CompletionPollResult {
    request_indices: Vec<usize>,
    statuses: Vec<u8>,
    latency_tsc_ticks: Vec<u64>,
    visible_prefix_len: u64,
}

impl CompletionPollResult {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            request_indices: Vec::with_capacity(capacity),
            statuses: Vec::with_capacity(capacity),
            latency_tsc_ticks: Vec::with_capacity(capacity),
            visible_prefix_len: 0,
        }
    }

    fn clear(&mut self) {
        self.request_indices.clear();
        self.statuses.clear();
        self.latency_tsc_ticks.clear();
        self.visible_prefix_len = 0;
    }

    fn record_read(&mut self, request_index: usize, status: u8, latency_tsc: u64) {
        self.request_indices.push(request_index);
        self.statuses.push(status);
        self.latency_tsc_ticks.push(latency_tsc);
    }

    fn read_count(&self) -> u64 {
        self.request_indices.len() as u64
    }
}

struct RequestCompletionAccumulator {
    request_index: usize,
    observed_after_submit_index: Vec<u64>,
    completion_tsc: Vec<u64>,
    observed_from_marker_tsc: Vec<u64>,
    success_count: u64,
    error_count: u64,
}

impl RequestCompletionAccumulator {
    fn new(request_index: usize, iterations: usize) -> Self {
        Self {
            request_index,
            observed_after_submit_index: Vec::with_capacity(iterations),
            completion_tsc: Vec::with_capacity(iterations),
            observed_from_marker_tsc: Vec::with_capacity(iterations),
            success_count: 0,
            error_count: 0,
        }
    }

    fn record(
        &mut self,
        status: u8,
        completion_tsc: u64,
        observed_after_submit_index: u64,
        observed_from_marker_tsc: Option<u64>,
    ) {
        self.observed_after_submit_index
            .push(observed_after_submit_index);
        self.completion_tsc.push(completion_tsc);

        if let Some(observed_from_marker_tsc) = observed_from_marker_tsc {
            self.observed_from_marker_tsc.push(observed_from_marker_tsc);
        }

        if status == DSA_COMP_SUCCESS {
            self.success_count += 1;
        } else {
            self.error_count += 1;
        }
    }

    fn into_trace(self, tsc_freq: u64, iterations: usize) -> SubmitMarkerRequestCompletionTrace {
        let observed_count = self.completion_tsc.len() as u64;
        let completion_ns = optional_stats_from_tsc_slice(&self.completion_tsc, tsc_freq);

        SubmitMarkerRequestCompletionTrace {
            request_index: self.request_index,
            observed_count,
            observed_fraction: observed_count as f64 / iterations as f64,
            success_count: self.success_count,
            error_count: self.error_count,
            observed_after_submit_index: optional_stats(self.observed_after_submit_index),
            completion_tsc_ticks: optional_stats(self.completion_tsc),
            completion_ns,
            observed_from_marker_tsc: optional_stats(self.observed_from_marker_tsc),
        }
    }
}

fn stats_from_tsc_slice(values: &[u64], tsc_freq: u64) -> crate::report::LatencyStats {
    stats_from_values(
        values
            .iter()
            .map(|&ticks| cycles_to_ns(ticks, tsc_freq))
            .collect(),
    )
}

fn optional_stats_from_tsc_slice(
    values: &[u64],
    tsc_freq: u64,
) -> Option<crate::report::LatencyStats> {
    if values.is_empty() {
        None
    } else {
        Some(stats_from_tsc_slice(values, tsc_freq))
    }
}

fn wait_for_completion(completion: &DsaCompletionRecord, tsc_freq: u64) -> Option<(u64, u8)> {
    let timeout_tsc = (COMPLETION_TIMEOUT_NS * u128::from(tsc_freq) / 1_000_000_000) as u64;
    let wait_start_tsc = rdtscp().0;
    let mut spins = 0_u64;

    loop {
        let status = DsaCompletionStatus::mask(completion.status());
        let now = rdtscp().0;
        if status != DSA_COMP_NONE {
            return Some((now, status));
        }

        spins = spins.wrapping_add(1);
        if spins & (TIMEOUT_CHECK_STRIDE - 1) == 0
            && now.saturating_sub(wait_start_tsc) >= timeout_tsc
        {
            return None;
        }

        core::hint::spin_loop();
    }
}

fn drain_with_drain_descriptor(
    wq: &WqPortal,
    drain_desc: &DsaHwDesc,
    drain_comp: &mut DsaCompletionRecord,
    operation: DsaOperationClass,
    n: usize,
    poll_offset: usize,
) {
    reset_completion(drain_comp);
    mfence();
    unsafe { wq.submit(drain_desc) };
    let status = poll_completion(drain_comp);
    if status != DSA_COMP_SUCCESS {
        panic!(
            "submit-marker-overlap drain descriptor failed: status {status:#x} \
             (operation={}, n={n}, poll_offset={poll_offset})",
            operation.as_str()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_accumulators_cover_all_submit_indices() {
        let traces = trace_accumulators(5, 3, 1);

        let submit_indices = traces
            .iter()
            .map(|trace| trace.submit_index)
            .collect::<Vec<_>>();
        let poll_flags = traces
            .iter()
            .map(|trace| trace.poll_performed)
            .collect::<Vec<_>>();

        assert_eq!(submit_indices, vec![0, 1, 2, 3, 4]);
        assert_eq!(poll_flags, vec![false, false, false, true, true]);
    }

    #[test]
    fn trace_records_poll_read_latency_and_frontier() {
        let mut trace = TraceAccumulator::new(3, true, 1);
        let mut poll = CompletionPollResult::with_capacity(2);

        poll.record_read(2, DSA_COMP_SUCCESS, 7);
        poll.record_read(3, DSA_COMP_NONE, 11);
        poll.visible_prefix_len = 3;
        trace.record_submit(10, Some(2), Some(12));
        trace.record_poll_summary(poll.visible_prefix_len, poll.read_count());
        trace.record_poll_latencies(&poll.latency_tsc_ticks);

        let point = trace.into_point(1_000_000_000);
        assert_eq!(point.poll_latency_tsc_ticks.unwrap().median, 11);
        assert_eq!(point.poll_count.unwrap().median, 2);
        assert!(point.completions.is_empty());
        assert_eq!(point.visible_count.median, 3);
    }
    #[test]
    fn request_completion_accumulator_reports_observation_stats() {
        let mut accumulator = RequestCompletionAccumulator::new(3, 4);

        accumulator.record(DSA_COMP_SUCCESS, 120, 7, Some(90));
        let trace = accumulator.into_trace(1_000_000_000, 4);

        assert_eq!(trace.request_index, 3);
        assert_eq!(trace.observed_count, 1);
        assert_eq!(trace.success_count, 1);
        assert_eq!(trace.error_count, 0);
        assert_eq!(trace.observed_fraction, 0.25);
        assert_eq!(trace.completion_tsc_ticks.unwrap().median, 120);
        assert_eq!(trace.observed_after_submit_index.unwrap().median, 7);
        assert_eq!(trace.observed_from_marker_tsc.unwrap().median, 90);
    }

    #[test]
    fn sample_trace_point_preserves_frontier_poll_batch() {
        let mut poll = CompletionPollResult::with_capacity(2);
        poll.record_read(2, DSA_COMP_SUCCESS, 22);
        poll.record_read(3, DSA_COMP_NONE, 24);
        poll.visible_prefix_len = 3;

        let point = sample_trace_point(
            3,
            100,
            124,
            Some(80),
            poll.visible_prefix_len,
            &poll.request_indices,
            &poll.latency_tsc_ticks,
            &poll.statuses,
        );

        assert_eq!(point.submit_index, 3);
        assert_eq!(point.submit_tsc_ticks, 24);
        assert_eq!(point.submit_start_from_marker_tsc, Some(20));
        assert_eq!(point.submit_end_from_marker_tsc, Some(44));
        assert_eq!(point.poll_end_from_marker_tsc, None);
        assert_eq!(point.poll_window_tsc_ticks, None);
        assert_eq!(point.polled_request_indices, vec![2, 3]);
        assert_eq!(point.poll_latency_tsc_ticks, vec![22, 24]);
        assert_eq!(point.poll_count, 2);
        assert_eq!(point.visible_prefix_len, 3);
        assert_eq!(point.first_polled_request_index, Some(2));
        assert_eq!(point.last_polled_request_index, Some(3));
        assert_eq!(point.polled_status, DSA_COMP_NONE);
        assert_eq!(point.polled_statuses, vec![DSA_COMP_SUCCESS, DSA_COMP_NONE]);
        assert_eq!(point.marker_status, DSA_COMP_NONE);
    }
}
