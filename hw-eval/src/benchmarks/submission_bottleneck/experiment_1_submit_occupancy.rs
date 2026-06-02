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

use hw_eval::dsa::{
    completion_flags_no_cache_control, poll_completion, reset_completion, DsaCompletionRecord,
    DsaHwDesc, DSA_COMP_SUCCESS,
};
use hw_eval::submit::{cycles_to_ns, WqPortal};

use crate::config::DsaOperationClass;
use crate::report::{
    SubmitOccupancyExtraTracePoint, SubmitOccupancyPrefillCompletionTracePoint,
    SubmitOccupancyResult, SubmitOccupancyTraceOutcome,
};

use super::common::{
    count_visible_completions, measured_call, reset_sample_completions, scan_visible_completions,
    OperationSlots,
};

const SUBMIT_OCCUPANCY_BENCHMARK: &str = "submit_occupancy_trace";

pub(crate) fn bench_submit_occupancy_one_extra(
    wq: &WqPortal,
    occupancies: &[usize],
    operation: DsaOperationClass,
    payload_size: usize,
    trace_until: Option<usize>,
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
    let mut slots = OperationSlots::new_with_payload(max_submitted, operation, payload_size);
    let mut seen = vec![false; max_submitted];
    let mut sentinel_comp = DsaCompletionRecord::default();

    if !json {
        print_trace_header(&operation_class);
    }

    for &k_prefill in occupancies {
        let submitted = submitted_count(k_prefill, trace_until);
        let mut prefill_completion_trace = Vec::with_capacity(iterations);
        let mut extra_submit_trace = Vec::with_capacity(iterations * (submitted - k_prefill));
        let mut trace_outcomes = Vec::with_capacity(iterations);

        for iteration_index in 0..iterations {
            reset_sample_completions(&mut slots.completions[..submitted]);
            submit_descriptors(wq, &slots.descriptors[..k_prefill]);
            trace_measured_submits(
                wq,
                &slots.descriptors[k_prefill..submitted],
                k_prefill,
                iteration_index,
                tsc_freq,
                &mut extra_submit_trace,
            );

            trace_outcomes.push(measure_trace_outcome(
                wq,
                &mut sentinel_comp,
                &operation_class,
                &slots.completions[..submitted],
                &mut seen[..submitted],
                k_prefill,
                iteration_index,
            ));

            prefill_completion_trace.push(measure_prefill_completion_trace(
                wq,
                &slots.descriptors[..k_prefill],
                &mut slots.completions[..k_prefill],
                &mut seen[..k_prefill],
                tsc_freq,
                iteration_index,
            ));
            if k_prefill > 0 {
                drain_with_sentinel(wq, &mut sentinel_comp, &operation_class, k_prefill);
            }
        }

        let result = SubmitOccupancyResult {
            benchmark: SUBMIT_OCCUPANCY_BENCHMARK.to_string(),
            operation_class: operation_class.clone(),
            k_prefill,
            submitted,
            trace_until: submitted,
            prefill_completion_trace,
            extra_submit_trace,
            trace_outcomes,
        };

        if !json {
            print_trace_row(&result);
        }
        results.push(result);
    }
}

fn submitted_count(k_prefill: usize, trace_until: Option<usize>) -> usize {
    trace_until.unwrap_or(k_prefill + 1).max(k_prefill + 1)
}

fn trace_measured_submits(
    wq: &WqPortal,
    descriptors: &[DsaHwDesc],
    first_submit_index: usize,
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
    }
}

fn measure_trace_outcome(
    wq: &WqPortal,
    sentinel_comp: &mut DsaCompletionRecord,
    operation_class: &str,
    completions: &[DsaCompletionRecord],
    seen: &mut [bool],
    k_prefill: usize,
    iteration_index: usize,
) -> SubmitOccupancyTraceOutcome {
    let bounded_outcome = count_visible_completions(completions, seen);

    drain_with_sentinel(wq, sentinel_comp, operation_class, k_prefill);

    let outcome = if bounded_outcome.completed == completions.len() {
        bounded_outcome
    } else {
        scan_visible_completions(completions)
    };

    SubmitOccupancyTraceOutcome {
        iteration_index,
        completed: outcome.completed,
        missing: completions.len() - outcome.completed,
        errors: outcome.errors,
    }
}

fn submit_descriptors(wq: &WqPortal, descriptors: &[DsaHwDesc]) {
    for desc in descriptors {
        unsafe { wq.submit(desc) };
    }
}

fn measure_prefill_completion_trace(
    wq: &WqPortal,
    descriptors: &[DsaHwDesc],
    completions: &mut [DsaCompletionRecord],
    seen: &mut [bool],
    tsc_freq: u64,
    iteration_index: usize,
) -> SubmitOccupancyPrefillCompletionTracePoint {
    reset_sample_completions(completions);

    if descriptors.is_empty() {
        return SubmitOccupancyPrefillCompletionTracePoint {
            iteration_index,
            prefill_submit_tsc_ticks: 0,
            prefill_submit_ns: 0,
            prefill_completion_tsc_ticks: 0,
            prefill_completion_ns: 0,
            post_submit_completion_tsc_ticks: 0,
            post_submit_completion_ns: 0,
            completed: 0,
            missing: 0,
            errors: 0,
        };
    }

    let submit = measured_call(|| submit_descriptors(wq, descriptors));
    let outcome = measured_call(|| count_visible_completions(completions, seen));
    let prefill_submit_tsc_ticks = submit.elapsed_tsc();
    let prefill_completion_tsc_ticks = outcome.end_tsc.saturating_sub(submit.start_tsc);
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
    operation_class: &str,
    k_prefill: usize,
) {
    let mut sentinel_desc = DsaHwDesc::default();
    sentinel_desc.fill_noop(completion_flags_no_cache_control());
    reset_completion(sentinel_comp);
    sentinel_desc.set_completion(sentinel_comp);

    unsafe { wq.submit(&sentinel_desc) };
    let status = poll_completion(sentinel_comp);
    if status != DSA_COMP_SUCCESS {
        panic!(
            "DSA submit-occupancy drain sentinel failed: status {status:#x} \
             (operation={operation_class}, k_prefill={k_prefill})"
        );
    }
}

fn print_trace_header(operation_class: &str) {
    println!("\n=== {SUBMIT_OCCUPANCY_BENCHMARK} ({operation_class}) ===");
    println!(
        "{:>8} {:>10} {:>15} {:>12} {:>12}",
        "K", "submitted", "prefill_samples", "trace_points", "iterations"
    );
}

fn print_trace_row(result: &SubmitOccupancyResult) {
    println!(
        "{:>8} {:>10} {:>15} {:>12} {:>12}",
        result.k_prefill,
        result.submitted,
        result.prefill_completion_trace.len(),
        result.extra_submit_trace.len(),
        result.trace_outcomes.len()
    );
}
