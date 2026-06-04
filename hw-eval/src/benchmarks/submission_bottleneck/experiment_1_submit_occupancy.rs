// Experiment 1: occupancy-conditioned submit trace.
//
// Logical operations are submitted one per MMIO. Each iteration pre-submits K
// descriptors, then records raw timing for the measured submit range:
//
//   default: [op 0] ... [op K-1] + [op K]
//                         prefill     measured
//
//   trace-until N: [op 0] ... [op K-1] + [op K] ... [op N-1]
//                              prefill     measured submit trace
//
// Statistics are deliberately left to post-processing so plotting can choose
// percentiles and filters without changing the hardware runner.
//

use std::hint::black_box;
use std::time::Duration;

use hw_eval::dsa::{
    completion_flags_no_cache_control, reset_completion, DsaCompletionRecord, DsaCompletionStatus,
    DsaHwDesc, DSA_COMP_NONE, DSA_COMP_SUCCESS,
};
use hw_eval::submit::{cycles_to_ns, rdtsc_relaxed, WqPortal};

use crate::config::DsaOperationClass;
use crate::report::{
    SubmitOccupancyExtraTracePoint, SubmitOccupancyPrefillCompletionTracePoint,
    SubmitOccupancyResult, SubmitOccupancyTraceOutcome,
};

use super::common::{
    count_visible_completions, measured_call, reset_sample_completions, scan_visible_completions,
    MeasuredCall, OperationSlots,
};

const SUBMIT_OCCUPANCY_BENCHMARK: &str = "submit_occupancy_trace";
const DRAIN_SENTINEL_TIMEOUT_STATUS: u8 = 0xff;
const DRAIN_SENTINEL_MAX_SPINS: u64 = 5_000_000_000_000;

#[derive(Clone, Copy)]
struct SubmitGap {
    black_box_iters: u64,
    target_tsc_ticks: u64,
}

impl SubmitGap {
    #[inline(always)]
    fn apply(self) {
        if self.target_tsc_ticks != 0 {
            wait_tsc_gap(self.target_tsc_ticks);
        } else if self.black_box_iters != 0 {
            black_box_gap(self.black_box_iters);
        }
    }
}

pub(crate) fn bench_submit_occupancy_one_extra(
    wq: &WqPortal,
    occupancies: &[usize],
    operation: DsaOperationClass,
    payload_size: usize,
    trace_until: Option<usize>,
    spin_loop_iters: u64,
    gap_tsc_ticks: u64,
    shared_payload: bool,
    iterations: usize,
    tsc_freq: u64,
    json: bool,
    results: &mut Vec<SubmitOccupancyResult>,
) {
    let Some(max_submitted) = occupancies
        .iter()
        .map(|&k_prefill| submitted_count(k_prefill, trace_until))
        .max()
    else {
        return;
    };

    let operation_class = operation.operation_class_label(payload_size);
    let mut slots = if shared_payload {
        OperationSlots::new_with_shared_payload(max_submitted, operation, payload_size)
    } else {
        OperationSlots::new_with_payload(max_submitted, operation, payload_size)
    };
    let mut seen = vec![false; max_submitted];
    let mut sentinel_comp = DsaCompletionRecord::default();
    let mut sentinel_desc = DsaHwDesc::default();
    sentinel_desc.fill_noop(completion_flags_no_cache_control());
    sentinel_desc.set_completion(&mut sentinel_comp);

    let submit_gap = SubmitGap {
        black_box_iters: spin_loop_iters,
        target_tsc_ticks: gap_tsc_ticks,
    };

    if !json {
        print_trace_header(&operation_class);
    }

    for &k_prefill in occupancies {
        let submitted = submitted_count(k_prefill, trace_until);
        let mut prefill_completion_trace = Vec::with_capacity(iterations);
        let mut extra_submit_trace = Vec::with_capacity(iterations * (submitted - k_prefill));
        let mut trace_outcomes = Vec::with_capacity(iterations);
        let mut drain_incomplete = false;

        for iteration_index in 0..iterations {
            reset_sample_completions(&mut slots.completions[..submitted]);
            let prefill_submit = measured_call(|| {
                submit_prefill(wq, &slots.descriptors[..k_prefill], submit_gap);
            });
            trace_measured_submits(
                wq,
                &slots.descriptors[k_prefill..submitted],
                k_prefill,
                submit_gap,
                iteration_index,
                tsc_freq,
                &mut extra_submit_trace,
            );

            prefill_completion_trace.push(measure_prefill_completion_trace(
                prefill_submit,
                &slots.completions[..k_prefill],
                &mut seen[..k_prefill],
                tsc_freq,
                iteration_index,
            ));

            let trace_outcome = measure_trace_outcome(
                wq,
                &mut sentinel_comp,
                &sentinel_desc,
                &slots.completions[..submitted],
                &mut seen[..submitted],
                iteration_index,
            );
            if !trace_outcome.drain_sentinel_completed {
                drain_incomplete = true;
            }
            trace_outcomes.push(trace_outcome);
        }

        let result = SubmitOccupancyResult {
            benchmark: SUBMIT_OCCUPANCY_BENCHMARK.to_string(),
            operation_class: operation_class.clone(),
            k_prefill,
            submitted,
            trace_until: submitted,
            spin_loop_iters,
            gap_tsc_ticks,
            shared_payload,
            prefill_completion_trace,
            extra_submit_trace,
            trace_outcomes,
        };

        if !json {
            print_trace_row(&result);
        }
        results.push(result);

        if drain_incomplete {
            std::mem::forget(slots);
            std::thread::sleep(Duration::from_secs(5));
            return;
        }
    }
}

fn submitted_count(k_prefill: usize, trace_until: Option<usize>) -> usize {
    trace_until.unwrap_or(k_prefill + 1).max(k_prefill + 1)
}

fn trace_measured_submits(
    wq: &WqPortal,
    descriptors: &[DsaHwDesc],
    first_submit_index: usize,
    submit_gap: SubmitGap,
    iteration_index: usize,
    tsc_freq: u64,
    trace: &mut Vec<SubmitOccupancyExtraTracePoint>,
) {
    for (offset, desc) in descriptors.iter().enumerate() {
        let submit = measured_call(|| unsafe { wq.submit(desc) });
        trace.push(SubmitOccupancyExtraTracePoint {
            iteration_index,
            submit_index: first_submit_index + offset,
            submit_tsc_ticks: submit.elapsed_tsc(),
            submit_ns: cycles_to_ns(submit.elapsed_tsc(), tsc_freq),
        });
        if offset + 1 < descriptors.len() {
            submit_gap.apply();
        }
    }
}

fn measure_trace_outcome(
    wq: &WqPortal,
    sentinel_comp: &mut DsaCompletionRecord,
    sentinel_desc: &DsaHwDesc,
    completions: &[DsaCompletionRecord],
    seen: &mut [bool],
    iteration_index: usize,
) -> SubmitOccupancyTraceOutcome {
    let bounded_outcome = count_visible_completions(completions, seen);

    let drain_sentinel_status = drain_with_sentinel(wq, sentinel_comp, sentinel_desc);

    let outcome = if bounded_outcome.completed == completions.len() {
        bounded_outcome
    } else {
        scan_visible_completions(completions)
    };

    SubmitOccupancyTraceOutcome {
        iteration_index,
        completed: outcome.completed,
        hardware_observed: outcome.completed,
        missing: completions.len() - outcome.completed,
        errors: outcome.errors,
        drain_sentinel_completed: drain_sentinel_status == DSA_COMP_SUCCESS,
        drain_sentinel_status,
    }
}

fn submit_prefill(wq: &WqPortal, descriptors: &[DsaHwDesc], submit_gap: SubmitGap) {
    for desc in descriptors {
        unsafe { wq.submit(desc) };
        submit_gap.apply();
    }
}

fn wait_tsc_gap(target_tsc_ticks: u64) {
    let start = rdtsc_relaxed();
    while rdtsc_relaxed().wrapping_sub(start) < target_tsc_ticks {}
}

fn black_box_gap(iterations: u64) {
    for n in 0..iterations {
        black_box(n);
    }
}

fn measure_prefill_completion_trace(
    prefill_submit: MeasuredCall<()>,
    completions: &[DsaCompletionRecord],
    seen: &mut [bool],
    tsc_freq: u64,
    iteration_index: usize,
) -> SubmitOccupancyPrefillCompletionTracePoint {
    let outcome = measured_call(|| count_visible_completions(completions, seen));
    let prefill_submit_tsc_ticks = prefill_submit.elapsed_tsc();
    let prefill_completion_tsc_ticks = outcome.end_tsc.saturating_sub(prefill_submit.start_tsc);
    let post_submit_completion_tsc_ticks =
        prefill_completion_tsc_ticks.saturating_sub(prefill_submit_tsc_ticks);

    SubmitOccupancyPrefillCompletionTracePoint {
        iteration_index,
        prefill_submit_tsc_ticks,
        prefill_submit_ns: cycles_to_ns(prefill_submit_tsc_ticks, tsc_freq),
        prefill_completion_tsc_ticks,
        prefill_completion_ns: cycles_to_ns(prefill_completion_tsc_ticks, tsc_freq),
        post_submit_completion_tsc_ticks,
        post_submit_completion_ns: cycles_to_ns(post_submit_completion_tsc_ticks, tsc_freq),
        completed: outcome.value.completed,
        missing: completions.len() - outcome.value.completed,
        errors: outcome.value.errors,
    }
}

fn drain_with_sentinel(
    wq: &WqPortal,
    sentinel_comp: &mut DsaCompletionRecord,
    sentinel_desc: &DsaHwDesc,
) -> u8 {
    reset_completion(sentinel_comp);

    unsafe { wq.submit(sentinel_desc) };
    poll_sentinel_completion(sentinel_comp)
}

fn poll_sentinel_completion(comp: &DsaCompletionRecord) -> u8 {
    let mut spins = 0_u64;
    loop {
        let status = comp.status();
        if status != DSA_COMP_NONE {
            return DsaCompletionStatus::mask(status);
        }

        spins += 1;
        if spins >= DRAIN_SENTINEL_MAX_SPINS {
            return DRAIN_SENTINEL_TIMEOUT_STATUS;
        }
        core::hint::spin_loop();
    }
}

fn print_trace_header(operation_class: &str) {
    println!("\n=== {SUBMIT_OCCUPANCY_BENCHMARK} ({operation_class}) ===");
    println!(
        "{:>8} {:>10} {:>10} {:>10} {:>8} {:>15} {:>12} {:>12}",
        "K",
        "submitted",
        "bb_iter",
        "gap_tsc",
        "shared",
        "prefill_samples",
        "trace_points",
        "iterations"
    );
}

fn print_trace_row(result: &SubmitOccupancyResult) {
    println!(
        "{:>8} {:>10} {:>10} {:>10} {:>8} {:>15} {:>12} {:>12}",
        result.k_prefill,
        result.submitted,
        result.spin_loop_iters,
        result.gap_tsc_ticks,
        result.shared_payload,
        result.prefill_completion_trace.len(),
        result.extra_submit_trace.len(),
        result.trace_outcomes.len()
    );
}
