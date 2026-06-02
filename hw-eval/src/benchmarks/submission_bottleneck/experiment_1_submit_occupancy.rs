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
use hw_eval::submit::{cycles_to_ns, rdtscp, WqPortal};

use crate::config::DsaOperationClass;
use crate::report::{
    SubmitOccupancyExtraTracePoint, SubmitOccupancyResult, SubmitOccupancyTraceOutcome,
};

use super::common::{
    count_visible_completions, reset_sample_completions, scan_visible_completions, OperationSlots,
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

    let mut slots = OperationSlots::new_with_payload(max_submitted, operation, payload_size);
    let mut seen = vec![false; max_submitted];
    let mut sentinel_comp = DsaCompletionRecord::default();
    let operation_class = operation.operation_class_label(payload_size);

    if !json {
        print_trace_header(&operation_class);
    }

    for &k_prefill in occupancies {
        let submitted = submitted_count(k_prefill, trace_until);
        let mut trace = Vec::with_capacity(iterations * (submitted - k_prefill));
        let mut outcomes = Vec::with_capacity(iterations);

        for iteration_index in 0..iterations {
            reset_sample_completions(&mut slots.completions[..submitted]);

            for desc in &slots.descriptors[..k_prefill] {
                unsafe { wq.submit(desc) };
            }

            for submit_index in k_prefill..submitted {
                let start_tsc = rdtscp().0;
                unsafe { wq.submit(&slots.descriptors[submit_index]) };
                let submit_tsc_ticks = rdtscp().0 - start_tsc;
                trace.push(SubmitOccupancyExtraTracePoint {
                    iteration_index,
                    submit_index,
                    submit_tsc_ticks,
                    submit_ns: cycles_to_ns(submit_tsc_ticks, tsc_freq),
                });
            }

            let bounded_outcome =
                count_visible_completions(&slots.completions[..submitted], &mut seen[..submitted]);

            drain_with_sentinel(wq, &mut sentinel_comp, operation, payload_size, k_prefill);

            let outcome = if bounded_outcome.completed == submitted {
                bounded_outcome
            } else {
                scan_visible_completions(&slots.completions[..submitted])
            };

            outcomes.push(SubmitOccupancyTraceOutcome {
                iteration_index,
                completed: outcome.completed,
                missing: submitted - outcome.completed,
                errors: outcome.errors,
            });
        }

        let result = SubmitOccupancyResult {
            benchmark: SUBMIT_OCCUPANCY_BENCHMARK.to_string(),
            operation_class: operation_class.clone(),
            k_prefill,
            submitted,
            trace_until: submitted,
            extra_submit_trace: trace,
            trace_outcomes: outcomes,
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

fn drain_with_sentinel(
    wq: &WqPortal,
    sentinel_comp: &mut DsaCompletionRecord,
    operation: DsaOperationClass,
    payload_size: usize,
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
             (operation={}, k_prefill={k_prefill})",
            operation.operation_class_label(payload_size)
        );
    }
}

fn print_trace_header(operation_class: &str) {
    println!("\n=== {SUBMIT_OCCUPANCY_BENCHMARK} ({operation_class}) ===");
    println!(
        "{:>8} {:>10} {:>12} {:>12}",
        "K", "submitted", "trace_points", "iterations"
    );
}

fn print_trace_row(result: &SubmitOccupancyResult) {
    println!(
        "{:>8} {:>10} {:>12} {:>12}",
        result.k_prefill,
        result.submitted,
        result.extra_submit_trace.len(),
        result.trace_outcomes.len()
    );
}
