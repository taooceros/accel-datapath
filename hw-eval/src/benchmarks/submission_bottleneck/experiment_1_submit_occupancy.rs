// Experiment 1: occupancy-conditioned one-extra-submit latency.
//
// Logical operations are submitted one per MMIO. Prefill K outstanding
// descriptors, then time exactly one additional submit.
//
//   WQ before measured submit:
//   [op 0][op 1] ... [op K-1]  +  [extra op]
//        outstanding K              measured submit
//
// Question: does one more submit stay cheap, or bend as occupancy consumes
// the admission/credit domain?
//
use std::time::Instant;

use hw_eval::dsa::{
    completion_flags_no_cache_control, poll_completion, reset_completion, DsaCompletionRecord,
    DsaHwDesc, DSA_COMP_NONE, DSA_COMP_SUCCESS,
};
use hw_eval::submit::{cycles_to_ns, lfence, rdtscp, WqPortal};

use crate::config::DsaOperationClass;
use crate::report::{stats_from_values, SubmitOccupancyResult};

use super::common::{
    count_visible_completions, optional_stats, reset_sample_completions, scan_visible_completions,
    OperationSlots, COMPLETION_TIMEOUT_NS, TIMEOUT_CHECK_STRIDE,
};

const SUBMIT_OCCUPANCY_BENCHMARK: &str = "submit_occupancy_one_extra";

pub(crate) fn bench_submit_occupancy_one_extra(
    wq: &WqPortal,
    occupancies: &[usize],
    operation: DsaOperationClass,
    iterations: usize,
    tsc_freq: u64,
    json: bool,
    results: &mut Vec<SubmitOccupancyResult>,
) {
    let Some(&max_occupancy) = occupancies.iter().max() else {
        return;
    };

    let max_slots = max_occupancy + 1;
    let mut slots = OperationSlots::new(max_slots, operation);
    let mut seen = vec![false; max_slots];

    let mut sentinel_desc = DsaHwDesc::default();
    let mut sentinel_comp = DsaCompletionRecord::default();
    sentinel_desc.fill_noop(completion_flags_no_cache_control());
    sentinel_desc.set_completion(&mut sentinel_comp);

    if !json {
        println!(
            "\n=== {SUBMIT_OCCUPANCY_BENCHMARK} ({}) ===",
            operation.as_str()
        );
        println!(
            "{:>8} {:>10} {:>10} {:>10} {:>14} {:>14} {:>14}",
            "K", "submitted", "completed", "missing", "extra_tsc", "extra_ns", "first_old_tsc"
        );
    }

    for &k_prefill in occupancies {
        let submitted = k_prefill + 1;

        let mut completed_counts = Vec::with_capacity(iterations);
        let mut missing_counts = Vec::with_capacity(iterations);
        let mut error_counts = Vec::with_capacity(iterations);
        let mut extra_submit_tsc = Vec::with_capacity(iterations);
        let mut extra_submit_ns = Vec::with_capacity(iterations);
        let mut first_old_tsc = Vec::with_capacity(iterations);
        let mut first_old_ns = Vec::with_capacity(iterations);

        for _ in 0..iterations {
            reset_sample_completions(&mut slots.completions[..submitted]);

            for desc in &slots.descriptors[..k_prefill] {
                unsafe { wq.submit(desc) };
            }

            lfence();
            let t0 = rdtscp().0;
            unsafe { wq.submit(&slots.descriptors[k_prefill]) };
            let extra_ticks = rdtscp().0 - t0;

            if k_prefill != 0 {
                if let Some(first_old_ticks) =
                    wait_for_first_old_completion(&slots.completions[0], t0)
                {
                    first_old_tsc.push(first_old_ticks);
                    first_old_ns.push(cycles_to_ns(first_old_ticks, tsc_freq));
                }
            }

            let bounded_outcome =
                count_visible_completions(&slots.completions[..submitted], &mut seen[..submitted]);

            reset_completion(&mut sentinel_comp);
            drain_with_sentinel(wq, &sentinel_desc, &mut sentinel_comp, operation, k_prefill);

            let outcome = if bounded_outcome.completed == submitted {
                bounded_outcome
            } else {
                scan_visible_completions(&slots.completions[..submitted])
            };

            completed_counts.push(outcome.completed as u64);
            missing_counts.push((submitted - outcome.completed) as u64);
            error_counts.push(outcome.errors as u64);
            extra_submit_tsc.push(extra_ticks);
            extra_submit_ns.push(cycles_to_ns(extra_ticks, tsc_freq));
        }

        let completed = stats_from_values(completed_counts);
        let missing = stats_from_values(missing_counts);
        let errors = stats_from_values(error_counts);
        let extra_submit_tsc_ticks = stats_from_values(extra_submit_tsc);
        let extra_submit_ns = stats_from_values(extra_submit_ns);
        let first_old_completion_tsc_ticks = optional_stats(first_old_tsc);
        let first_old_completion_ns = optional_stats(first_old_ns);

        if !json {
            if let Some(first_old) = first_old_completion_tsc_ticks.as_ref() {
                println!(
                    "{:>8} {:>10} {:>10} {:>10} {:>14} {:>14} {:>14}",
                    k_prefill,
                    submitted,
                    completed.median,
                    missing.median,
                    extra_submit_tsc_ticks.median,
                    extra_submit_ns.median,
                    first_old.median
                );
            } else {
                println!(
                    "{:>8} {:>10} {:>10} {:>10} {:>14} {:>14} {:>14}",
                    k_prefill,
                    submitted,
                    completed.median,
                    missing.median,
                    extra_submit_tsc_ticks.median,
                    extra_submit_ns.median,
                    "-"
                );
            }
        }

        results.push(SubmitOccupancyResult {
            benchmark: SUBMIT_OCCUPANCY_BENCHMARK.to_string(),
            operation_class: operation.as_str().to_string(),
            k_prefill,
            submitted,
            completed,
            missing,
            errors,
            extra_submit_tsc_ticks,
            extra_submit_ns,
            first_old_completion_tsc_ticks,
            first_old_completion_ns,
        });
    }
}

fn wait_for_first_old_completion(completion: &DsaCompletionRecord, t0: u64) -> Option<u64> {
    let start = Instant::now();
    let mut spins = 0_u64;

    loop {
        if completion.status() != DSA_COMP_NONE {
            return Some(rdtscp().0 - t0);
        }

        spins = spins.wrapping_add(1);
        if spins & (TIMEOUT_CHECK_STRIDE - 1) == 0
            && start.elapsed().as_nanos() >= COMPLETION_TIMEOUT_NS
        {
            return None;
        }

        core::hint::spin_loop();
    }
}

fn drain_with_sentinel(
    wq: &WqPortal,
    sentinel_desc: &DsaHwDesc,
    sentinel_comp: &mut DsaCompletionRecord,
    operation: DsaOperationClass,
    k_prefill: usize,
) {
    unsafe { wq.submit(sentinel_desc) };
    let status = poll_completion(sentinel_comp);
    if status != DSA_COMP_SUCCESS {
        panic!(
            "DSA submit-occupancy drain sentinel failed: status {status:#x} \
             (operation={}, k_prefill={k_prefill})",
            operation.as_str()
        );
    }
}
