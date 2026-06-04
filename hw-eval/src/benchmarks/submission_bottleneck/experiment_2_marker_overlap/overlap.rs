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
    count_visible_completions, measured_call, optional_stats, reset_sample_completions,
    OperationSlots, COMPLETION_TIMEOUT_NS, TIMEOUT_CHECK_STRIDE,
};

const SUBMIT_MARKER_OVERLAP_BENCHMARK: &str = "submit_marker_overlap";
const POLL_STEP: usize = 1;

pub(crate) fn bench_submit_marker_overlap(
    wq: &WqPortal,
    bursts: &[usize],
    positions: &[MarkerPosition],
    poll_offsets: &[usize],
    operation: DsaOperationClass,
    payload_size: usize,
    iterations: usize,
    tsc_freq: u64,
    json: bool,
    results: &mut Vec<SubmitMarkerOverlapResult>,
) {
    let Some(&max_burst) = bursts.iter().max() else {
        return;
    };
    let mut scratch = OverlapScratch::new(max_burst, operation, payload_size);

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
                let result = run_overlap_case(
                    wq,
                    &mut scratch,
                    &drain_desc,
                    &mut drain_comp,
                    OverlapCase {
                        n,
                        position,
                        marker_position,
                        poll_offset,
                        operation,
                        iterations,
                        tsc_freq,
                    },
                );

                if !json {
                    print_result_row(&result);
                }

                results.push(result);
            }
        }
    }
}

fn print_result_row(result: &SubmitMarkerOverlapResult) {
    let marker_ns = result
        .marker_visible_ns
        .as_ref()
        .map(|stats| stats.median.to_string())
        .unwrap_or_else(|| "-".to_string());
    println!(
        "{:>8} {:>8} {:>8} {:>12} {:>14} {:>14} {:>10}",
        result.n,
        result.marker_position,
        result.marker_poll_offset,
        result.trace.len(),
        result.submit_tail_ns.median,
        marker_ns,
        result.completed.median
    );
}

#[derive(Clone, Copy)]
struct OverlapCase {
    n: usize,
    position: MarkerPosition,
    marker_position: usize,
    poll_offset: usize,
    operation: DsaOperationClass,
    iterations: usize,
    tsc_freq: u64,
}

struct OverlapScratch {
    slots: OperationSlots,
    seen: Vec<bool>,
    observations: CompletionObservations,
    iteration_trace: IterationTrace,
    poll_result: CompletionPollResult,
}

impl OverlapScratch {
    fn new(max_burst: usize, operation: DsaOperationClass, payload_size: usize) -> Self {
        Self {
            slots: OperationSlots::new_with_payload(max_burst, operation, payload_size),
            seen: vec![false; max_burst],
            observations: CompletionObservations::new(max_burst),
            iteration_trace: IterationTrace::new(max_burst),
            poll_result: CompletionPollResult::with_capacity(max_burst),
        }
    }

    fn reset_iteration(&mut self, n: usize) {
        reset_sample_completions(&mut self.slots.completions[..n]);
        self.observations.reset(n);
        self.iteration_trace.reset(n);
    }
}

fn run_overlap_case(
    wq: &WqPortal,
    scratch: &mut OverlapScratch,
    drain_desc: &DsaHwDesc,
    drain_comp: &mut DsaCompletionRecord,
    case: OverlapCase,
) -> SubmitMarkerOverlapResult {
    let mut submit_tail_tsc = Vec::with_capacity(case.iterations);
    let mut marker_visible_tsc = Vec::with_capacity(case.iterations);
    let mut completed_counts = Vec::with_capacity(case.iterations);
    let mut missing_counts = Vec::with_capacity(case.iterations);
    let mut error_counts = Vec::with_capacity(case.iterations);
    let mut observed_before_final_submit = 0_u64;

    let mut request_accumulators = request_completion_accumulators(case.n, case.iterations);
    let mut trace_accumulators = trace_accumulators(case.n, case.poll_offset, case.iterations);
    let mut sample_trace = Vec::with_capacity(case.iterations);

    for iteration_index in 0..case.iterations {
        scratch.reset_iteration(case.n);

        let mut next_completion_to_poll = 0_usize;
        let mut marker_submit_tsc = None;

        for submit_index in 0..case.n {
            let submit =
                measured_call(|| unsafe { wq.submit(&scratch.slots.descriptors[submit_index]) });
            scratch
                .iteration_trace
                .record_submit(submit_index, submit.start_tsc, submit.end_tsc);

            if submit_index == case.marker_position {
                marker_submit_tsc = Some(submit.start_tsc);
            }

            trace_accumulators[submit_index].record_submit(
                submit.elapsed_tsc(),
                marker_submit_tsc.map(|marker_tsc| submit.start_tsc.saturating_sub(marker_tsc)),
                marker_submit_tsc.map(|marker_tsc| submit.end_tsc.saturating_sub(marker_tsc)),
            );

            if submit_index >= case.poll_offset && next_completion_to_poll < case.n {
                poll_to_next_unfinished(
                    &scratch.slots.completions[..case.n],
                    &mut next_completion_to_poll,
                    &mut scratch.observations,
                    submit_index,
                    marker_submit_tsc,
                    &mut scratch.poll_result,
                );
                scratch
                    .iteration_trace
                    .record_poll_event(submit_index, &scratch.poll_result);
                trace_accumulators[submit_index].record_poll_summary(
                    scratch.poll_result.visible_prefix_len,
                    scratch.poll_result.read_count(),
                );
            } else {
                trace_accumulators[submit_index].record_no_poll();
            }
        }

        for (submit_index, accumulator) in trace_accumulators.iter_mut().enumerate() {
            accumulator.record_poll_latencies(scratch.iteration_trace.poll_latencies(submit_index));
        }

        let final_submit_tsc = scratch.iteration_trace.submit_end_tsc(case.n - 1);
        let marker_observed_tsc = observe_remaining_completions(
            &scratch.slots.completions[..case.n],
            &mut scratch.observations,
            case.tsc_freq,
            case.n - 1,
            marker_submit_tsc,
            case.marker_position,
        );
        scratch.observations.record_request_completions(
            case.n,
            scratch.iteration_trace.submit_start_tscs(case.n),
            &mut request_accumulators[..case.n],
        );

        if marker_observed_tsc != 0 && marker_observed_tsc < final_submit_tsc {
            observed_before_final_submit += 1;
        }

        if marker_observed_tsc != 0 {
            marker_visible_tsc.push(
                marker_observed_tsc.saturating_sub(
                    scratch
                        .iteration_trace
                        .submit_start_tsc(case.marker_position),
                ),
            );
        }

        // Trace boundary: materialize only submit-loop observations.
        // The drain below is cleanup and must not affect per-submit trace points.
        sample_trace.push(scratch.iteration_trace.to_sample_trace(
            case.n,
            iteration_index,
            case.marker_position,
            marker_submit_tsc,
        ));

        // Cleanup boundary: after this point we may force queue progress with a
        // drain descriptor, so only aggregate end-of-iteration accounting belongs here.
        let outcome =
            count_visible_completions(&scratch.slots.completions[..case.n], &mut scratch.seen);
        drain_with_drain_descriptor(
            wq,
            drain_desc,
            drain_comp,
            case.operation,
            case.n,
            case.poll_offset,
        );
        if let Some(marker_tsc) = marker_submit_tsc {
            submit_tail_tsc.push(final_submit_tsc.saturating_sub(marker_tsc));
        }
        completed_counts.push(outcome.completed as u64);
        missing_counts.push((case.n - outcome.completed) as u64);
        error_counts.push(outcome.errors as u64);
    }

    let submit_tail_ns_stats = stats_from_tsc_slice(&submit_tail_tsc, case.tsc_freq);
    let marker_visible_ns_stats = optional_stats_from_tsc_slice(&marker_visible_tsc, case.tsc_freq);
    let marker_visible_tsc_stats = optional_stats(marker_visible_tsc);
    let completed = stats_from_values(completed_counts);

    let request_completions = request_accumulators
        .into_iter()
        .map(|accumulator| accumulator.into_trace(case.tsc_freq, case.iterations))
        .collect::<Vec<_>>();

    let trace = trace_accumulators
        .into_iter()
        .map(|accumulator| accumulator.into_point(case.tsc_freq))
        .collect::<Vec<_>>();

    SubmitMarkerOverlapResult {
        benchmark: SUBMIT_MARKER_OVERLAP_BENCHMARK.to_string(),
        operation_class: case.operation.as_str().to_string(),
        n: case.n,
        marker_position: case.marker_position,
        marker_position_label: case.position.as_str().to_string(),
        poll_cadence: POLL_STEP.to_string(),
        marker_poll_offset: case.poll_offset,
        poll_step: POLL_STEP,
        tracked_completions: 0,
        submit_tail_tsc_ticks: stats_from_values(submit_tail_tsc),
        submit_tail_ns: submit_tail_ns_stats,
        marker_visible_tsc_ticks: marker_visible_tsc_stats,
        marker_visible_ns: marker_visible_ns_stats,
        marker_observed_before_final_submit_count: observed_before_final_submit,
        marker_observed_before_final_submit_fraction: observed_before_final_submit as f64
            / case.iterations as f64,
        completed,
        missing: stats_from_values(missing_counts),
        errors: stats_from_values(error_counts),
        request_completions,
        sample_trace,
        trace,
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

struct CompletionObservations {
    tscs: Vec<u64>,
    statuses: Vec<u8>,
    after_submit_indices: Vec<usize>,
    from_marker_tscs: Vec<Option<u64>>,
}

impl CompletionObservations {
    fn new(max_burst: usize) -> Self {
        Self {
            tscs: vec![0; max_burst],
            statuses: vec![DSA_COMP_NONE; max_burst],
            after_submit_indices: vec![0; max_burst],
            from_marker_tscs: vec![None; max_burst],
        }
    }

    fn reset(&mut self, n: usize) {
        self.tscs[..n].fill(0);
        self.statuses[..n].fill(DSA_COMP_NONE);
        self.after_submit_indices[..n].fill(0);
        self.from_marker_tscs[..n].fill(None);
    }

    fn is_observed(&self, completion_index: usize) -> bool {
        self.tscs[completion_index] != 0
    }

    fn marker_tsc(&self, marker_position: usize) -> u64 {
        self.tscs[marker_position]
    }

    fn record(
        &mut self,
        completion_index: usize,
        status: u8,
        completion_tsc: u64,
        observed_after_submit_index: usize,
        marker_submit_tsc: Option<u64>,
    ) {
        self.tscs[completion_index] = completion_tsc;
        self.statuses[completion_index] = status;
        self.after_submit_indices[completion_index] = observed_after_submit_index;
        self.from_marker_tscs[completion_index] =
            marker_submit_tsc.map(|marker_tsc| completion_tsc.saturating_sub(marker_tsc));
    }

    fn record_request_completions(
        &self,
        n: usize,
        submit_start_tscs: &[u64],
        request_accumulators: &mut [RequestCompletionAccumulator],
    ) {
        for completion_index in 0..n {
            if !self.is_observed(completion_index) {
                continue;
            }

            record_request_completion(
                completion_index,
                self.statuses[completion_index],
                self.tscs[completion_index],
                self.after_submit_indices[completion_index],
                self.from_marker_tscs[completion_index],
                submit_start_tscs,
                request_accumulators,
            );
        }
    }
}

struct IterationTrace {
    submit_start_tscs: Vec<u64>,
    submit_end_tscs: Vec<u64>,
    poll_event_offsets: Vec<usize>,
    poll_event_counts: Vec<usize>,
    poll_visible_prefix_lens: Vec<u64>,
    poll_event_request_indices: Vec<usize>,
    poll_event_statuses: Vec<u8>,
    poll_event_latency_tsc_ticks: Vec<u64>,
}

impl IterationTrace {
    fn new(max_burst: usize) -> Self {
        let max_poll_reads_per_iteration = max_burst.saturating_mul(2);
        Self {
            submit_start_tscs: vec![0; max_burst],
            submit_end_tscs: vec![0; max_burst],
            poll_event_offsets: vec![0; max_burst],
            poll_event_counts: vec![0; max_burst],
            poll_visible_prefix_lens: vec![0; max_burst],
            poll_event_request_indices: Vec::with_capacity(max_poll_reads_per_iteration),
            poll_event_statuses: Vec::with_capacity(max_poll_reads_per_iteration),
            poll_event_latency_tsc_ticks: Vec::with_capacity(max_poll_reads_per_iteration),
        }
    }

    fn reset(&mut self, n: usize) {
        self.submit_start_tscs[..n].fill(0);
        self.submit_end_tscs[..n].fill(0);
        self.poll_event_offsets[..n].fill(0);
        self.poll_event_counts[..n].fill(0);
        self.poll_visible_prefix_lens[..n].fill(0);
        self.poll_event_request_indices.clear();
        self.poll_event_statuses.clear();
        self.poll_event_latency_tsc_ticks.clear();
    }

    fn record_submit(&mut self, submit_index: usize, start_tsc: u64, end_tsc: u64) {
        self.submit_start_tscs[submit_index] = start_tsc;
        self.submit_end_tscs[submit_index] = end_tsc;
    }

    fn record_poll_event(&mut self, submit_index: usize, result: &CompletionPollResult) {
        let event_start = self.poll_event_request_indices.len();

        self.poll_event_offsets[submit_index] = event_start;
        self.poll_event_counts[submit_index] = result.request_indices.len();
        self.poll_visible_prefix_lens[submit_index] = result.visible_prefix_len;
        self.poll_event_request_indices
            .extend_from_slice(&result.request_indices);
        self.poll_event_statuses.extend_from_slice(&result.statuses);
        self.poll_event_latency_tsc_ticks
            .extend_from_slice(&result.latency_tsc_ticks);
    }

    fn submit_start_tsc(&self, submit_index: usize) -> u64 {
        self.submit_start_tscs[submit_index]
    }

    fn submit_end_tsc(&self, submit_index: usize) -> u64 {
        self.submit_end_tscs[submit_index]
    }

    fn submit_start_tscs(&self, n: usize) -> &[u64] {
        &self.submit_start_tscs[..n]
    }

    fn poll_latencies(&self, submit_index: usize) -> &[u64] {
        let event_start = self.poll_event_offsets[submit_index];
        let event_end = event_start + self.poll_event_counts[submit_index];
        &self.poll_event_latency_tsc_ticks[event_start..event_end]
    }

    fn to_sample_trace(
        &self,
        n: usize,
        iteration_index: usize,
        marker_position: usize,
        marker_submit_tsc: Option<u64>,
    ) -> SubmitMarkerSampleTrace {
        let points = (0..n)
            .map(|submit_index| {
                let point_marker_submit_tsc = if submit_index >= marker_position {
                    marker_submit_tsc
                } else {
                    None
                };
                self.sample_trace_point(submit_index, point_marker_submit_tsc)
            })
            .collect();

        SubmitMarkerSampleTrace {
            iteration_index,
            points,
        }
    }

    fn sample_trace_point(
        &self,
        submit_index: usize,
        marker_submit_tsc: Option<u64>,
    ) -> SubmitMarkerSampleTracePoint {
        let event_start = self.poll_event_offsets[submit_index];
        let event_end = event_start + self.poll_event_counts[submit_index];

        sample_trace_point(
            submit_index,
            self.submit_start_tscs[submit_index],
            self.submit_end_tscs[submit_index],
            marker_submit_tsc,
            self.poll_visible_prefix_lens[submit_index],
            &self.poll_event_request_indices[event_start..event_end],
            &self.poll_event_latency_tsc_ticks[event_start..event_end],
            &self.poll_event_statuses[event_start..event_end],
        )
    }
}

fn observe_remaining_completions(
    completions: &[DsaCompletionRecord],
    observations: &mut CompletionObservations,
    tsc_freq: u64,
    observed_after_submit_index: usize,
    marker_submit_tsc: Option<u64>,
    marker_position: usize,
) -> u64 {
    let mut marker_observed_tsc = observations.marker_tsc(marker_position);

    for completion_index in 0..completions.len() {
        if observations.is_observed(completion_index) {
            continue;
        }

        if let Some((completion_tsc, status)) =
            wait_for_completion(&completions[completion_index], tsc_freq)
        {
            observations.record(
                completion_index,
                status,
                completion_tsc,
                observed_after_submit_index,
                marker_submit_tsc,
            );
            if completion_index == marker_position {
                marker_observed_tsc = completion_tsc;
            }
        }
    }

    marker_observed_tsc
}

fn poll_to_next_unfinished(
    completions: &[DsaCompletionRecord],
    next_completion_to_poll: &mut usize,
    observations: &mut CompletionObservations,
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

        observations.record(
            completion_index,
            status,
            poll.end_tsc,
            submit_index,
            marker_submit_tsc,
        );
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
    fn iteration_trace_materializes_only_active_submit_indices() {
        let mut trace = IterationTrace::new(5);
        let mut poll = CompletionPollResult::with_capacity(2);

        trace.reset(3);
        trace.record_submit(0, 10, 14);
        trace.record_submit(1, 20, 29);
        trace.record_submit(2, 30, 41);
        poll.record_read(0, DSA_COMP_SUCCESS, 7);
        poll.record_read(1, DSA_COMP_NONE, 11);
        poll.visible_prefix_len = 1;
        trace.record_poll_event(1, &poll);

        let sample = trace.to_sample_trace(3, 0, 0, Some(10));

        assert_eq!(sample.points.len(), 3);
        assert_eq!(sample.points[0].submit_tsc_ticks, 4);
        assert_eq!(sample.points[1].submit_tsc_ticks, 9);
        assert_eq!(sample.points[1].polled_request_indices, vec![0, 1]);
        assert_eq!(sample.points[1].poll_latency_tsc_ticks, vec![7, 11]);
        assert_eq!(sample.points[1].visible_prefix_len, 1);
        assert_eq!(sample.points[2].submit_tsc_ticks, 11);
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
