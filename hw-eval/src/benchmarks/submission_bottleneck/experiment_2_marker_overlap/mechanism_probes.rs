// Experiment 2 mechanism probes: completion cacheline visibility.
//
// These probes keep the Experiment 2 submit/poll shape but vary only the
// completion-record layout, prefetch distance, timing method, or cache state.
//
//   submit order:
//   [0] [1] [2] [3] ... [poll_offset] [poll_offset+1] ... [N-1]
//       ^ one logical op/MMIO       ^ poll next unfinished completion frontier
//
// Question: is the high visible-status-read latency the first CPU touch of a
// DSA-written 64-byte completion cacheline, and can layout/prefetch/cache state
// change that cost?

use std::mem;
use std::time::Instant;

use hw_eval::dsa::{
    completion_flags_no_cache_control, poll_completion, reset_completion, DsaCompletionRecord,
    DsaCompletionStatus, DsaHwDesc, DSA_COMP_NONE, DSA_COMP_SUCCESS,
};
use hw_eval::submit::{cycles_to_ns, flush_range, mfence, WqPortal};

use crate::config::DsaOperationClass;
use crate::report::{
    stats_from_values, LatencyStats, MedianGap, SubmitMarkerMechanismBaselineComparison,
    SubmitMarkerMechanismResult, SubmitMarkerSampleTrace, SubmitMarkerSampleTracePoint,
};

use super::super::common::{
    fill_descriptor, measured_call, optional_stats, OperationSlots, COMPLETION_TIMEOUT_NS,
    TIMEOUT_CHECK_STRIDE,
};

const SUBMIT_MARKER_MECHANISM_BENCHMARK: &str = "submit_marker_mechanism";
const CACHELINE_BYTES: usize = 64;
const COMPLETION_RECORD_BYTES: usize = 32;
const DEFAULT_POLL_SUBMIT_BATCH_N: usize = 1;

pub(crate) fn bench_submit_marker_mechanism_probes(
    wq: &WqPortal,
    bursts: &[usize],
    poll_offsets: &[usize],
    poll_submit_batches: &[usize],
    operation: DsaOperationClass,
    payload_size: usize,
    iterations: usize,
    tsc_freq: u64,
    json: bool,
    results: &mut Vec<SubmitMarkerMechanismResult>,
) {
    let Some(&max_burst) = bursts.iter().max() else {
        return;
    };

    let mut packed_slots = OperationSlots::new_with_payload(max_burst, operation, payload_size);
    let mut padded_slots = PaddedOperationSlots::new(max_burst, operation, payload_size);

    if !json {
        println!(
            "\n=== {SUBMIT_MARKER_MECHANISM_BENCHMARK} traced ({}) ===",
            operation.as_str()
        );
        println!(
            "{:>8} {:>8} {:>8} {:>18} {:>22} {:>14} {:>14} {:>14}",
            "n", "offset", "poll_n", "sub_exp", "variant", "visible_ns", "none_ns", "completed"
        );
    }

    for &n in bursts {
        for &poll_offset in poll_offsets {
            let baseline_spec = ProbeSpec::baseline();
            let baseline_result = run_probe(
                wq,
                &mut packed_slots,
                n,
                poll_offset,
                operation,
                iterations,
                tsc_freq,
                baseline_spec,
            );
            let baseline = BaselineSummary::from_result(&baseline_result);
            print_result_row(json, &baseline_result);
            results.push(baseline_result);

            for &poll_submit_batch_n in poll_submit_batches {
                if poll_submit_batch_n == DEFAULT_POLL_SUBMIT_BATCH_N {
                    continue;
                }

                let mut result = run_probe(
                    wq,
                    &mut packed_slots,
                    n,
                    poll_offset,
                    operation,
                    iterations,
                    tsc_freq,
                    ProbeSpec::poll_submit_batch(poll_submit_batch_n),
                );
                baseline.apply_to(&mut result);
                print_result_row(json, &result);
                results.push(result);
            }
            let packed_specs = [
                ProbeSpec::prefetch(
                    "prefetch-1-lines",
                    1,
                    CacheState::ResetOnly,
                    TimingMode::PerRead,
                ),
                ProbeSpec::prefetch(
                    "prefetch-2-lines",
                    2,
                    CacheState::ResetOnly,
                    TimingMode::PerRead,
                ),
                ProbeSpec::prefetch(
                    "prefetch-4-lines",
                    4,
                    CacheState::ResetOnly,
                    TimingMode::PerRead,
                ),
                ProbeSpec::measurement(
                    "batch-scan-timing",
                    0,
                    CacheState::ResetOnly,
                    TimingMode::BatchScan,
                ),
                ProbeSpec::cache_state("pre-touch", 0, CacheState::PreTouch, TimingMode::PerRead),
                ProbeSpec::cache_state("clflush", 0, CacheState::Clflush, TimingMode::PerRead),
            ];

            for spec in packed_specs {
                let mut result = run_probe(
                    wq,
                    &mut packed_slots,
                    n,
                    poll_offset,
                    operation,
                    iterations,
                    tsc_freq,
                    spec,
                );
                baseline.apply_to(&mut result);
                print_result_row(json, &result);
                results.push(result);
            }

            let padded_spec =
                ProbeSpec::layout("padded-64b", 0, CacheState::ResetOnly, TimingMode::PerRead);
            let mut result = run_probe(
                wq,
                &mut padded_slots,
                n,
                poll_offset,
                operation,
                iterations,
                tsc_freq,
                padded_spec,
            );
            baseline.apply_to(&mut result);
            print_result_row(json, &result);
            results.push(result);
        }
    }
}

fn print_result_row(json: bool, result: &SubmitMarkerMechanismResult) {
    if json {
        return;
    }

    let visible_ns = result
        .visible_poll_ns
        .as_ref()
        .map(|stats| stats.median.to_string())
        .unwrap_or_else(|| "-".to_string());
    let none_ns = result
        .none_poll_ns
        .as_ref()
        .map(|stats| stats.median.to_string())
        .unwrap_or_else(|| "-".to_string());

    println!(
        "{:>8} {:>8} {:>8} {:>18} {:>22} {:>14} {:>14} {:>14}",
        result.n,
        result.marker_poll_offset,
        result.poll_submit_batch_n,
        result.sub_experiment,
        result.variant,
        visible_ns,
        none_ns,
        result.completed.median,
    );
}

struct BaselineSummary {
    label: String,
    visible_poll_ns_median: Option<u64>,
    none_poll_ns_median: Option<u64>,
    line_position_0_visible_ns_median: Option<u64>,
    line_position_1_visible_ns_median: Option<u64>,
    same_line_first_visible_ns_median: Option<u64>,
    same_line_second_visible_ns_median: Option<u64>,
}

impl BaselineSummary {
    fn from_result(result: &SubmitMarkerMechanismResult) -> Self {
        Self {
            label: format!("{}/{}", result.sub_experiment, result.variant),
            visible_poll_ns_median: median_of(&result.visible_poll_ns),
            none_poll_ns_median: median_of(&result.none_poll_ns),
            line_position_0_visible_ns_median: median_of(&result.line_position_0_visible_ns),
            line_position_1_visible_ns_median: median_of(&result.line_position_1_visible_ns),
            same_line_first_visible_ns_median: median_of(&result.same_line_first_visible_ns),
            same_line_second_visible_ns_median: median_of(&result.same_line_second_visible_ns),
        }
    }

    fn apply_to(&self, result: &mut SubmitMarkerMechanismResult) {
        result.baseline_comparison = Some(SubmitMarkerMechanismBaselineComparison {
            baseline: self.label.clone(),
            visible_poll_ns: median_gap(&result.visible_poll_ns, self.visible_poll_ns_median),
            none_poll_ns: median_gap(&result.none_poll_ns, self.none_poll_ns_median),
            line_position_0_visible_ns: median_gap(
                &result.line_position_0_visible_ns,
                self.line_position_0_visible_ns_median,
            ),
            line_position_1_visible_ns: median_gap(
                &result.line_position_1_visible_ns,
                self.line_position_1_visible_ns_median,
            ),
            same_line_first_visible_ns: median_gap(
                &result.same_line_first_visible_ns,
                self.same_line_first_visible_ns_median,
            ),
            same_line_second_visible_ns: median_gap(
                &result.same_line_second_visible_ns,
                self.same_line_second_visible_ns_median,
            ),
        });
    }
}

fn median_of(stats: &Option<LatencyStats>) -> Option<u64> {
    stats.as_ref().map(|stats| stats.median)
}

fn median_gap(stats: &Option<LatencyStats>, baseline_median: Option<u64>) -> Option<MedianGap> {
    let baseline_median = baseline_median?;
    let current_median = stats.as_ref()?.median;
    if baseline_median == 0 {
        return None;
    }

    Some(MedianGap {
        delta_ns: current_median as i64 - baseline_median as i64,
        ratio_to_baseline: current_median as f64 / baseline_median as f64,
    })
}

#[derive(Clone, Copy)]
struct ProbeSpec {
    sub_experiment: &'static str,
    variant: &'static str,
    prefetch_distance_lines: usize,
    cache_state: CacheState,
    timing_mode: TimingMode,
    poll_submit_batch_n: usize,
}

impl ProbeSpec {
    const fn baseline() -> Self {
        Self {
            sub_experiment: "baseline",
            variant: "packed-32b",
            prefetch_distance_lines: 0,
            cache_state: CacheState::ResetOnly,
            timing_mode: TimingMode::PerRead,
            poll_submit_batch_n: DEFAULT_POLL_SUBMIT_BATCH_N,
        }
    }

    const fn layout(
        variant: &'static str,
        prefetch_distance_lines: usize,
        cache_state: CacheState,
        timing_mode: TimingMode,
    ) -> Self {
        Self {
            sub_experiment: "layout",
            variant,
            prefetch_distance_lines,
            cache_state,
            timing_mode,
            poll_submit_batch_n: DEFAULT_POLL_SUBMIT_BATCH_N,
        }
    }

    const fn prefetch(
        variant: &'static str,
        prefetch_distance_lines: usize,
        cache_state: CacheState,
        timing_mode: TimingMode,
    ) -> Self {
        Self {
            sub_experiment: "prefetch",
            variant,
            prefetch_distance_lines,
            cache_state,
            timing_mode,
            poll_submit_batch_n: DEFAULT_POLL_SUBMIT_BATCH_N,
        }
    }

    const fn measurement(
        variant: &'static str,
        prefetch_distance_lines: usize,
        cache_state: CacheState,
        timing_mode: TimingMode,
    ) -> Self {
        Self {
            sub_experiment: "measurement",
            variant,
            prefetch_distance_lines,
            cache_state,
            timing_mode,
            poll_submit_batch_n: DEFAULT_POLL_SUBMIT_BATCH_N,
        }
    }

    const fn cache_state(
        variant: &'static str,
        prefetch_distance_lines: usize,
        cache_state: CacheState,
        timing_mode: TimingMode,
    ) -> Self {
        Self {
            sub_experiment: "cache-state",
            variant,
            prefetch_distance_lines,
            cache_state,
            timing_mode,
            poll_submit_batch_n: DEFAULT_POLL_SUBMIT_BATCH_N,
        }
    }

    fn poll_submit_batch(poll_submit_batch_n: usize) -> Self {
        Self {
            sub_experiment: "poll-submit-batch",
            variant: "configured",
            prefetch_distance_lines: 0,
            cache_state: CacheState::ResetOnly,
            timing_mode: TimingMode::PerRead,
            poll_submit_batch_n,
        }
    }
}

#[derive(Clone, Copy)]
enum CacheState {
    ResetOnly,
    PreTouch,
    Clflush,
}

impl CacheState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::ResetOnly => "reset-only",
            Self::PreTouch => "pre-touch",
            Self::Clflush => "clflush",
        }
    }
}

#[derive(Clone, Copy)]
enum TimingMode {
    PerRead,
    BatchScan,
}

impl TimingMode {
    const fn as_str(self) -> &'static str {
        match self {
            Self::PerRead => "per-read",
            Self::BatchScan => "batch-scan",
        }
    }
}

trait CompletionStorage {
    fn layout_name(&self) -> &'static str;
    fn len(&self) -> usize;
    fn descriptor(&self, index: usize) -> &DsaHwDesc;
    fn completion(&self, index: usize) -> &DsaCompletionRecord;
    fn completion_mut(&mut self, index: usize) -> &mut DsaCompletionRecord;
    fn base_addr(&self) -> usize;
    fn storage_len_bytes(&self) -> usize;
    fn stride_bytes(&self) -> usize;
    fn alignment_bytes(&self) -> usize;

    fn completion_addr(&self, index: usize) -> usize {
        self.completion(index) as *const DsaCompletionRecord as usize
    }

    fn line_position(&self, index: usize) -> usize {
        ((self.completion_addr(index) & (CACHELINE_BYTES - 1)) / COMPLETION_RECORD_BYTES).min(1)
    }
}

impl CompletionStorage for OperationSlots {
    fn layout_name(&self) -> &'static str {
        "packed-32b"
    }

    fn len(&self) -> usize {
        self.completions.len()
    }

    fn descriptor(&self, index: usize) -> &DsaHwDesc {
        &self.descriptors[index]
    }

    fn completion(&self, index: usize) -> &DsaCompletionRecord {
        &self.completions[index]
    }

    fn completion_mut(&mut self, index: usize) -> &mut DsaCompletionRecord {
        &mut self.completions[index]
    }

    fn base_addr(&self) -> usize {
        self.completions.as_ptr() as usize
    }

    fn storage_len_bytes(&self) -> usize {
        self.completions.len() * mem::size_of::<DsaCompletionRecord>()
    }

    fn stride_bytes(&self) -> usize {
        mem::size_of::<DsaCompletionRecord>()
    }

    fn alignment_bytes(&self) -> usize {
        mem::align_of::<DsaCompletionRecord>()
    }
}

#[repr(C, align(64))]
#[derive(Clone, Copy)]
struct PaddedCompletionSlot {
    record: DsaCompletionRecord,
    _pad: [u8; 32],
}

impl Default for PaddedCompletionSlot {
    fn default() -> Self {
        Self {
            record: DsaCompletionRecord::default(),
            _pad: [0; 32],
        }
    }
}

struct PaddedOperationSlots {
    descriptors: Vec<DsaHwDesc>,
    completions: Vec<PaddedCompletionSlot>,
    _sources: Vec<u8>,
    _destinations: Vec<u8>,
}

impl PaddedOperationSlots {
    fn new(count: usize, operation: DsaOperationClass, payload_size: usize) -> Self {
        let mut descriptors = vec![DsaHwDesc::default(); count];
        let mut completions = vec![PaddedCompletionSlot::default(); count];
        let mut sources = vec![0xa5; count * payload_size];
        let mut destinations = vec![0; count * payload_size];
        touch_pages(&mut sources);
        touch_pages(&mut destinations);

        for slot in 0..count {
            fill_descriptor(
                &mut descriptors[slot],
                &mut completions[slot].record,
                &mut sources,
                &mut destinations,
                slot,
                payload_size,
                operation,
            );
        }

        Self {
            descriptors,
            completions,
            _sources: sources,
            _destinations: destinations,
        }
    }
}

fn touch_pages(buf: &mut [u8]) {
    for offset in (0..buf.len()).step_by(4096) {
        touch_byte(buf, offset);
    }

    if !buf.is_empty() {
        touch_byte(buf, buf.len() - 1);
    }
}

fn touch_byte(buf: &mut [u8], offset: usize) {
    unsafe {
        let ptr = buf.as_mut_ptr().add(offset);
        let value = ptr.read_volatile();
        ptr.write_volatile(value);
    }
}

impl CompletionStorage for PaddedOperationSlots {
    fn layout_name(&self) -> &'static str {
        "padded-64b"
    }

    fn len(&self) -> usize {
        self.completions.len()
    }

    fn descriptor(&self, index: usize) -> &DsaHwDesc {
        &self.descriptors[index]
    }

    fn completion(&self, index: usize) -> &DsaCompletionRecord {
        &self.completions[index].record
    }

    fn completion_mut(&mut self, index: usize) -> &mut DsaCompletionRecord {
        &mut self.completions[index].record
    }

    fn base_addr(&self) -> usize {
        self.completions.as_ptr() as usize
    }

    fn storage_len_bytes(&self) -> usize {
        self.completions.len() * mem::size_of::<PaddedCompletionSlot>()
    }

    fn stride_bytes(&self) -> usize {
        mem::size_of::<PaddedCompletionSlot>()
    }

    fn alignment_bytes(&self) -> usize {
        mem::align_of::<PaddedCompletionSlot>()
    }
}

struct IterationTrace {
    submit_start_tscs: Vec<u64>,
    submit_end_tscs: Vec<u64>,
    poll_event_offsets: Vec<usize>,
    poll_event_counts: Vec<usize>,
    poll_latency_offsets: Vec<usize>,
    poll_latency_counts: Vec<usize>,
    poll_visible_prefix_lens: Vec<u64>,
    poll_window_tsc_ticks: Vec<Option<u64>>,
    poll_event_request_indices: Vec<usize>,
    poll_event_statuses: Vec<u8>,
    poll_event_latency_tsc_ticks: Vec<u64>,
}

impl IterationTrace {
    fn new(n: usize) -> Self {
        Self {
            submit_start_tscs: vec![0; n],
            submit_end_tscs: vec![0; n],
            poll_event_offsets: vec![0; n],
            poll_event_counts: vec![0; n],
            poll_latency_offsets: vec![0; n],
            poll_latency_counts: vec![0; n],
            poll_visible_prefix_lens: vec![0; n],
            poll_window_tsc_ticks: vec![None; n],
            poll_event_request_indices: Vec::with_capacity(n * 2),
            poll_event_statuses: Vec::with_capacity(n * 2),
            poll_event_latency_tsc_ticks: Vec::with_capacity(n * 2),
        }
    }

    fn reset(&mut self) {
        self.submit_start_tscs.fill(0);
        self.submit_end_tscs.fill(0);
        self.poll_event_offsets.fill(0);
        self.poll_event_counts.fill(0);
        self.poll_latency_offsets.fill(0);
        self.poll_latency_counts.fill(0);
        self.poll_visible_prefix_lens.fill(0);
        self.poll_window_tsc_ticks.fill(None);
        self.poll_event_request_indices.clear();
        self.poll_event_statuses.clear();
        self.poll_event_latency_tsc_ticks.clear();
    }

    fn record_submit(&mut self, submit_index: usize, start_tsc: u64, end_tsc: u64) {
        self.submit_start_tscs[submit_index] = start_tsc;
        self.submit_end_tscs[submit_index] = end_tsc;
    }

    fn begin_poll_event(&mut self, submit_index: usize) {
        self.poll_event_offsets[submit_index] = self.poll_event_request_indices.len();
        self.poll_latency_offsets[submit_index] = self.poll_event_latency_tsc_ticks.len();
    }

    fn record_poll_read(&mut self, completion_index: usize, status: u8, latency_tsc: Option<u64>) {
        self.poll_event_request_indices.push(completion_index);
        self.poll_event_statuses.push(status);
        if let Some(latency_tsc) = latency_tsc {
            self.poll_event_latency_tsc_ticks.push(latency_tsc);
        }
    }

    fn finish_poll_event(
        &mut self,
        submit_index: usize,
        visible_prefix_len: u64,
        poll_window_tsc: Option<u64>,
    ) {
        self.poll_event_counts[submit_index] =
            self.poll_event_request_indices.len() - self.poll_event_offsets[submit_index];
        self.poll_latency_counts[submit_index] =
            self.poll_event_latency_tsc_ticks.len() - self.poll_latency_offsets[submit_index];
        self.poll_visible_prefix_lens[submit_index] = visible_prefix_len;
        self.poll_window_tsc_ticks[submit_index] = poll_window_tsc;
    }

    fn to_sample_trace(&self, iteration_index: usize) -> SubmitMarkerSampleTrace {
        let points = (0..self.submit_start_tscs.len())
            .map(|submit_index| self.sample_trace_point(submit_index))
            .collect();

        SubmitMarkerSampleTrace {
            iteration_index,
            points,
        }
    }

    fn sample_trace_point(&self, submit_index: usize) -> SubmitMarkerSampleTracePoint {
        let event_start = self.poll_event_offsets[submit_index];
        let event_end = event_start + self.poll_event_counts[submit_index];
        let latency_start = self.poll_latency_offsets[submit_index];
        let latency_end = latency_start + self.poll_latency_counts[submit_index];
        let request_indices = &self.poll_event_request_indices[event_start..event_end];
        let statuses = &self.poll_event_statuses[event_start..event_end];
        let latencies = &self.poll_event_latency_tsc_ticks[latency_start..latency_end];
        let poll_count = request_indices.len() as u64;
        let last_status = statuses.last().copied().unwrap_or(DSA_COMP_NONE);

        SubmitMarkerSampleTracePoint {
            submit_index,
            submit_tsc_ticks: self.submit_end_tscs[submit_index]
                .saturating_sub(self.submit_start_tscs[submit_index]),
            submit_start_from_marker_tsc: None,
            submit_end_from_marker_tsc: None,
            poll_performed: poll_count != 0,
            poll_end_from_marker_tsc: None,
            poll_window_tsc_ticks: self.poll_window_tsc_ticks[submit_index],
            polled_request_indices: request_indices.to_vec(),
            poll_latency_tsc_ticks: latencies.to_vec(),
            poll_count,
            first_polled_request_index: request_indices
                .first()
                .copied()
                .map(|request_index| request_index as u64),
            last_polled_request_index: request_indices
                .last()
                .copied()
                .map(|request_index| request_index as u64),
            visible_prefix_len: self.poll_visible_prefix_lens[submit_index],
            polled_statuses: statuses.to_vec(),
            polled_status: last_status,
            marker_status: last_status,
        }
    }
}

fn run_probe<S: CompletionStorage>(
    wq: &WqPortal,
    slots: &mut S,
    n: usize,
    poll_offset: usize,
    operation: DsaOperationClass,
    iterations: usize,
    tsc_freq: u64,
    spec: ProbeSpec,
) -> SubmitMarkerMechanismResult {
    assert!(n <= slots.len());

    let mut accumulator = ProbeAccumulator::default();
    let mut seen = vec![false; n];
    let mut iteration_trace = IterationTrace::new(n);
    let poll_submit_batch_n = spec.poll_submit_batch_n;

    let mut drain_desc = DsaHwDesc::default();
    let mut drain_comp = DsaCompletionRecord::default();
    drain_desc.fill_drain(completion_flags_no_cache_control());
    drain_desc.set_completion(&mut drain_comp);
    assert!(poll_submit_batch_n != 0);

    for iteration_index in 0..iterations {
        prepare_completions(slots, n, spec.cache_state);
        iteration_trace.reset();

        let mut next_completion_to_poll = 0_usize;
        for submit_index in 0..n {
            let submit = measured_call(|| unsafe { wq.submit(slots.descriptor(submit_index)) });
            iteration_trace.record_submit(submit_index, submit.start_tsc, submit.end_tsc);

            if should_poll_after_submit(submit_index, n, poll_offset, poll_submit_batch_n)
                && next_completion_to_poll < n
            {
                poll_to_next_unfinished(
                    slots,
                    n,
                    &mut next_completion_to_poll,
                    spec.prefetch_distance_lines,
                    spec.timing_mode,
                    &mut accumulator,
                    submit_index,
                    &mut iteration_trace,
                );
            }
        }

        let outcome = wait_for_all_completions(slots, n, &mut seen);
        if outcome.completed < n as u64 {
            drain_after_timeout(
                wq,
                &drain_desc,
                &mut drain_comp,
                operation,
                n,
                poll_offset,
                spec,
            );
        }
        accumulator.record_outcome(n, outcome);
        accumulator.record_sample_trace(iteration_trace.to_sample_trace(iteration_index));
    }

    accumulator.into_result(slots, n, poll_offset, operation, tsc_freq, spec)
}

fn prepare_completions<S: CompletionStorage>(slots: &mut S, n: usize, cache_state: CacheState) {
    for index in 0..n {
        reset_completion(slots.completion_mut(index));
    }

    match cache_state {
        CacheState::ResetOnly => {}
        CacheState::PreTouch => {
            for index in 0..n {
                core::hint::black_box(slots.completion(index).status());
            }
        }
        CacheState::Clflush => {
            flush_range(slots.base_addr() as *const u8, slots.storage_len_bytes());
        }
    }
}

fn should_poll_after_submit(
    submit_index: usize,
    n: usize,
    poll_offset: usize,
    poll_submit_batch_n: usize,
) -> bool {
    if submit_index < poll_offset {
        return false;
    }

    let submitted_since_poll_start = submit_index - poll_offset + 1;
    submitted_since_poll_start % poll_submit_batch_n == 0 || submit_index + 1 == n
}

fn poll_to_next_unfinished<S: CompletionStorage>(
    slots: &S,
    n: usize,
    next_completion_to_poll: &mut usize,
    prefetch_distance_lines: usize,
    timing_mode: TimingMode,
    accumulator: &mut ProbeAccumulator,
    submit_index: usize,
    iteration_trace: &mut IterationTrace,
) {
    iteration_trace.begin_poll_event(submit_index);

    match timing_mode {
        TimingMode::PerRead => {
            let mut current_line = None;
            let mut visible_reads_in_line = 0_u64;

            while *next_completion_to_poll < n {
                let completion_index = *next_completion_to_poll;
                prefetch_ahead(slots, completion_index, prefetch_distance_lines);

                let poll = measured_call(|| {
                    DsaCompletionStatus::mask(slots.completion(completion_index).status())
                });
                let status = poll.value;
                let latency_tsc = poll.elapsed_tsc();
                iteration_trace.record_poll_read(completion_index, status, Some(latency_tsc));

                if status == DSA_COMP_NONE {
                    accumulator.record_none(latency_tsc);
                    break;
                }

                record_visible_poll_read(
                    slots,
                    completion_index,
                    latency_tsc,
                    &mut current_line,
                    &mut visible_reads_in_line,
                    accumulator,
                );
                *next_completion_to_poll += 1;
            }

            iteration_trace.finish_poll_event(submit_index, *next_completion_to_poll as u64, None);
        }
        TimingMode::BatchScan => {
            let scan = measured_call(|| {
                let mut reads = 0_u64;
                let mut visible = 0_u64;

                while *next_completion_to_poll < n {
                    let completion_index = *next_completion_to_poll;
                    prefetch_ahead(slots, completion_index, prefetch_distance_lines);

                    reads += 1;
                    let status =
                        DsaCompletionStatus::mask(slots.completion(completion_index).status());
                    iteration_trace.record_poll_read(completion_index, status, None);
                    if status == DSA_COMP_NONE {
                        break;
                    }

                    visible += 1;
                    *next_completion_to_poll += 1;
                }

                (reads, visible)
            });

            let (reads, visible) = scan.value;
            if reads != 0 {
                let latency_tsc = scan.elapsed_tsc();
                accumulator.record_poll_window(latency_tsc, reads, visible);
                iteration_trace.finish_poll_event(
                    submit_index,
                    *next_completion_to_poll as u64,
                    Some(latency_tsc),
                );
            }
        }
    }
}

fn record_visible_poll_read<S: CompletionStorage>(
    slots: &S,
    completion_index: usize,
    latency_tsc: u64,
    current_line: &mut Option<usize>,
    visible_reads_in_line: &mut u64,
    accumulator: &mut ProbeAccumulator,
) {
    let line_addr = slots.completion_addr(completion_index) & !(CACHELINE_BYTES - 1);
    if *current_line != Some(line_addr) {
        *current_line = Some(line_addr);
        *visible_reads_in_line = 0;
    }
    *visible_reads_in_line += 1;

    accumulator.record_visible(
        latency_tsc,
        slots.line_position(completion_index),
        *visible_reads_in_line,
    );
}

fn prefetch_ahead<S: CompletionStorage>(
    slots: &S,
    completion_index: usize,
    prefetch_distance_lines: usize,
) {
    if prefetch_distance_lines == 0 {
        return;
    }

    let current_line = slots.completion_addr(completion_index) & !(CACHELINE_BYTES - 1);
    let target_line = current_line + CACHELINE_BYTES * prefetch_distance_lines;
    let start = slots.base_addr();
    let end = start + slots.storage_len_bytes();
    if target_line >= start && target_line < end {
        prefetch_t0(target_line as *const u8);
    }
}

#[inline(always)]
fn prefetch_t0(ptr: *const u8) {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::x86_64::_mm_prefetch(ptr.cast::<i8>(), core::arch::x86_64::_MM_HINT_T0);
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = ptr;
    }
}

#[derive(Clone, Copy, Default)]
struct CompletionOutcome {
    completed: u64,
    errors: u64,
}

fn wait_for_all_completions<S: CompletionStorage>(
    slots: &S,
    n: usize,
    seen: &mut [bool],
) -> CompletionOutcome {
    seen.fill(false);
    let mut outcome = CompletionOutcome::default();
    let start = Instant::now();
    let mut spins = 0_u64;

    loop {
        for (index, slot_seen) in seen.iter_mut().enumerate().take(n) {
            if *slot_seen {
                continue;
            }

            let status = DsaCompletionStatus::mask(slots.completion(index).status());
            if status == DSA_COMP_NONE {
                continue;
            }

            outcome.completed += 1;
            if status != DSA_COMP_SUCCESS {
                outcome.errors += 1;
            }
            *slot_seen = true;
        }

        if outcome.completed == n as u64 {
            return outcome;
        }

        spins = spins.wrapping_add(1);
        if spins & (TIMEOUT_CHECK_STRIDE - 1) == 0
            && start.elapsed().as_nanos() >= COMPLETION_TIMEOUT_NS
        {
            return outcome;
        }

        core::hint::spin_loop();
    }
}

fn drain_after_timeout(
    wq: &WqPortal,
    drain_desc: &DsaHwDesc,
    drain_comp: &mut DsaCompletionRecord,
    operation: DsaOperationClass,
    n: usize,
    poll_offset: usize,
    spec: ProbeSpec,
) {
    reset_completion(drain_comp);
    mfence();
    unsafe { wq.submit(drain_desc) };
    let status = poll_completion(drain_comp);
    if status != DSA_COMP_SUCCESS {
        panic!(
            "submit-marker-mechanism drain descriptor failed: status {status:#x} \
             (operation={}, n={n}, poll_offset={poll_offset}, sub_experiment={}, variant={})",
            operation.as_str(),
            spec.sub_experiment,
            spec.variant
        );
    }
}

#[derive(Default)]
struct ProbeAccumulator {
    completed_counts: Vec<u64>,
    missing_counts: Vec<u64>,
    error_counts: Vec<u64>,
    none_poll_tsc: Vec<u64>,
    visible_poll_tsc: Vec<u64>,
    line_position_visible_tsc: [Vec<u64>; 2],
    same_line_first_visible_tsc: Vec<u64>,
    same_line_second_visible_tsc: Vec<u64>,
    poll_window_tsc: Vec<u64>,
    poll_window_reads: Vec<u64>,
    poll_window_visible: Vec<u64>,
    sample_trace: Vec<SubmitMarkerSampleTrace>,
}

impl ProbeAccumulator {
    fn record_none(&mut self, latency_tsc: u64) {
        self.none_poll_tsc.push(latency_tsc);
    }

    fn record_visible(
        &mut self,
        latency_tsc: u64,
        line_position: usize,
        visible_reads_in_line: u64,
    ) {
        self.visible_poll_tsc.push(latency_tsc);
        self.line_position_visible_tsc[line_position.min(1)].push(latency_tsc);
        if visible_reads_in_line == 1 {
            self.same_line_first_visible_tsc.push(latency_tsc);
        } else if visible_reads_in_line == 2 {
            self.same_line_second_visible_tsc.push(latency_tsc);
        }
    }

    fn record_poll_window(&mut self, latency_tsc: u64, reads: u64, visible: u64) {
        self.poll_window_tsc.push(latency_tsc);
        self.poll_window_reads.push(reads);
        self.poll_window_visible.push(visible);
    }

    fn record_outcome(&mut self, n: usize, outcome: CompletionOutcome) {
        self.completed_counts.push(outcome.completed);
        self.missing_counts
            .push((n as u64).saturating_sub(outcome.completed));
        self.error_counts.push(outcome.errors);
    }

    fn record_sample_trace(&mut self, trace: SubmitMarkerSampleTrace) {
        self.sample_trace.push(trace);
    }

    fn into_result<S: CompletionStorage>(
        self,
        slots: &S,
        n: usize,
        poll_offset: usize,
        operation: DsaOperationClass,
        tsc_freq: u64,
        spec: ProbeSpec,
    ) -> SubmitMarkerMechanismResult {
        SubmitMarkerMechanismResult {
            benchmark: SUBMIT_MARKER_MECHANISM_BENCHMARK.to_string(),
            sub_experiment: spec.sub_experiment.to_string(),
            variant: spec.variant.to_string(),
            operation_class: operation.as_str().to_string(),
            n,
            marker_poll_offset: poll_offset,
            poll_submit_batch_n: spec.poll_submit_batch_n,
            completion_layout: slots.layout_name().to_string(),
            completion_stride_bytes: slots.stride_bytes(),
            completion_alignment_bytes: slots.alignment_bytes(),
            completion_base_mod_64: slots.base_addr() % CACHELINE_BYTES,
            completion_base_mod_4096: slots.base_addr() % 4096,
            prefetch_distance_lines: Some(spec.prefetch_distance_lines),
            cache_state: spec.cache_state.as_str().to_string(),
            timing_mode: spec.timing_mode.as_str().to_string(),
            submitted: (n as u64) * (self.completed_counts.len() as u64),
            completed: stats_from_values(self.completed_counts),
            missing: stats_from_values(self.missing_counts),
            errors: stats_from_values(self.error_counts),
            none_poll_tsc_ticks: optional_stats(self.none_poll_tsc.clone()),
            none_poll_ns: optional_ns_stats(&self.none_poll_tsc, tsc_freq),
            visible_poll_tsc_ticks: optional_stats(self.visible_poll_tsc.clone()),
            visible_poll_ns: optional_ns_stats(&self.visible_poll_tsc, tsc_freq),
            line_position_0_visible_tsc_ticks: optional_stats(
                self.line_position_visible_tsc[0].clone(),
            ),
            line_position_0_visible_ns: optional_ns_stats(
                &self.line_position_visible_tsc[0],
                tsc_freq,
            ),
            line_position_1_visible_tsc_ticks: optional_stats(
                self.line_position_visible_tsc[1].clone(),
            ),
            line_position_1_visible_ns: optional_ns_stats(
                &self.line_position_visible_tsc[1],
                tsc_freq,
            ),
            same_line_first_visible_tsc_ticks: optional_stats(
                self.same_line_first_visible_tsc.clone(),
            ),
            same_line_first_visible_ns: optional_ns_stats(
                &self.same_line_first_visible_tsc,
                tsc_freq,
            ),
            same_line_second_visible_tsc_ticks: optional_stats(
                self.same_line_second_visible_tsc.clone(),
            ),
            same_line_second_visible_ns: optional_ns_stats(
                &self.same_line_second_visible_tsc,
                tsc_freq,
            ),
            poll_window_tsc_ticks: optional_stats(self.poll_window_tsc.clone()),
            poll_window_ns: optional_ns_stats(&self.poll_window_tsc, tsc_freq),
            poll_window_reads: optional_stats(self.poll_window_reads),
            poll_window_visible: optional_stats(self.poll_window_visible),
            sample_trace: self.sample_trace,
            baseline_comparison: None,
        }
    }
}

fn optional_ns_stats(values: &[u64], tsc_freq: u64) -> Option<LatencyStats> {
    if values.is_empty() {
        None
    } else {
        Some(stats_from_values(
            values
                .iter()
                .map(|&ticks| cycles_to_ns(ticks, tsc_freq))
                .collect(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padded_completion_slot_is_one_cacheline() {
        assert_eq!(
            mem::size_of::<DsaCompletionRecord>(),
            COMPLETION_RECORD_BYTES
        );
        assert_eq!(
            mem::align_of::<DsaCompletionRecord>(),
            COMPLETION_RECORD_BYTES
        );
        assert_eq!(mem::size_of::<PaddedCompletionSlot>(), CACHELINE_BYTES);
        assert_eq!(mem::align_of::<PaddedCompletionSlot>(), CACHELINE_BYTES);
    }

    #[test]
    fn cacheline_position_uses_actual_base_alignment() {
        fn position(base_mod_64: usize, index: usize, stride: usize) -> usize {
            (((base_mod_64 + index * stride) & (CACHELINE_BYTES - 1)) / COMPLETION_RECORD_BYTES)
                .min(1)
        }

        assert_eq!(position(0, 0, COMPLETION_RECORD_BYTES), 0);
        assert_eq!(position(0, 1, COMPLETION_RECORD_BYTES), 1);
        assert_eq!(position(0, 2, COMPLETION_RECORD_BYTES), 0);
        assert_eq!(position(32, 0, COMPLETION_RECORD_BYTES), 1);
        assert_eq!(position(32, 1, COMPLETION_RECORD_BYTES), 0);
        assert_eq!(position(0, 1, CACHELINE_BYTES), 0);
    }

    #[test]
    fn poll_submit_batch_counts_submissions_between_polls() {
        let n = 10;
        let poll_offset = 2;
        let poll_submit_batch_n = 3;

        let poll_points = (0..n)
            .filter(|&submit_index| {
                should_poll_after_submit(submit_index, n, poll_offset, poll_submit_batch_n)
            })
            .collect::<Vec<_>>();

        assert_eq!(poll_points, vec![4, 7, 9]);
    }

    #[test]
    fn mechanism_probe_specs_have_stable_labels() {
        let baseline = ProbeSpec::baseline();
        assert_eq!(baseline.sub_experiment, "baseline");
        assert_eq!(baseline.variant, "packed-32b");
        assert_eq!(baseline.prefetch_distance_lines, 0);
        assert_eq!(baseline.cache_state.as_str(), "reset-only");
        assert_eq!(baseline.timing_mode.as_str(), "per-read");

        let spec = ProbeSpec::prefetch(
            "prefetch-2-lines",
            2,
            CacheState::PreTouch,
            TimingMode::BatchScan,
        );

        assert_eq!(spec.sub_experiment, "prefetch");
        assert_eq!(spec.variant, "prefetch-2-lines");
        assert_eq!(spec.prefetch_distance_lines, 2);
        assert_eq!(spec.cache_state.as_str(), "pre-touch");
        assert_eq!(spec.timing_mode.as_str(), "batch-scan");
    }
}
