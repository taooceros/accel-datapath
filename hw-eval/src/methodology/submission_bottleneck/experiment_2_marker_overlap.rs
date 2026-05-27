// Experiment 2: marker completion overlap during submit.
//
// Submit N logical operations one per MMIO, mark position p, and optionally
// poll that marker while the CPU is still submitting the tail.
//
//   submit order:
//   [1] ... [p: marker] ... [N]
//            ^ tp             ^ t1
//            |-- poll every M submits --|
//
// Question: can DSA execution/completion visibility overlap the CPU submit
// tail, or does the marker appear only after the submit loop returns?
//
use std::time::Instant;

use hw_eval::dsa::{
    completion_flags_no_cache_control, poll_completion, reset_completion, DsaCompletionRecord,
    DsaHwDesc, DSA_COMP_NONE, DSA_COMP_SUCCESS,
};
use hw_eval::submit::{cycles_to_ns, lfence, rdtscp, WqPortal};

use crate::config::{DsaOperationClass, MarkerPollCadence, MarkerPosition};
use crate::report::{stats_from_values, SubmitMarkerOverlapResult};

use super::common::{
    count_visible_completions, optional_stats, reset_sample_completions, OperationSlots,
    COMPLETION_TIMEOUT_NS, TIMEOUT_CHECK_STRIDE,
};

const SUBMIT_MARKER_OVERLAP_BENCHMARK: &str = "submit_marker_overlap";

pub(crate) fn bench_submit_marker_overlap(
    wq: &WqPortal,
    bursts: &[usize],
    positions: &[MarkerPosition],
    poll_cadences: &[MarkerPollCadence],
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
    let mut sentinel_desc = DsaHwDesc::default();
    let mut sentinel_comp = DsaCompletionRecord::default();
    sentinel_desc.fill_noop(completion_flags_no_cache_control());
    sentinel_desc.set_completion(&mut sentinel_comp);

    if !json {
        println!(
            "\n=== {SUBMIT_MARKER_OVERLAP_BENCHMARK} ({}) ===",
            operation.as_str()
        );
        println!(
            "{:>8} {:>8} {:>8} {:>14} {:>14} {:>10}",
            "n", "marker", "poll", "submit_ns", "marker_ns", "completed"
        );
    }

    for &n in bursts {
        for &position in positions {
            let marker_position = position.to_one_based(n);
            for &cadence in poll_cadences {
                let mut submit_tail_tsc = Vec::with_capacity(iterations);
                let mut submit_tail_ns = Vec::with_capacity(iterations);
                let mut marker_visible_tsc = Vec::with_capacity(iterations);
                let mut marker_visible_ns = Vec::with_capacity(iterations);
                let mut completed_counts = Vec::with_capacity(iterations);
                let mut missing_counts = Vec::with_capacity(iterations);
                let mut error_counts = Vec::with_capacity(iterations);
                let mut observed_before_t1 = 0_u64;

                for _ in 0..iterations {
                    reset_sample_completions(&mut slots.completions[..n]);

                    lfence();
                    let mut marker_submit_tsc = 0_u64;
                    let mut marker_seen_at = None;

                    for index in 0..n {
                        if index + 1 == marker_position {
                            marker_submit_tsc = rdtscp().0;
                        }
                        unsafe { wq.submit(&slots.descriptors[index]) };
                        if marker_seen_at.is_none()
                            && index + 1 >= marker_position
                            && should_poll_marker(cadence, index + 1)
                            && slots.completions[marker_position - 1].status() != DSA_COMP_NONE
                        {
                            marker_seen_at = Some(rdtscp().0);
                        }
                    }

                    let t1 = rdtscp().0;
                    if marker_seen_at.is_some_and(|tc| tc < t1) {
                        observed_before_t1 += 1;
                    }

                    let marker_tsc = match marker_seen_at {
                        Some(tc) => Some(tc.saturating_sub(marker_submit_tsc)),
                        None => wait_for_marker_completion(
                            &slots.completions[marker_position - 1],
                            marker_submit_tsc,
                        ),
                    };

                    let outcome =
                        count_visible_completions(&slots.completions[..n], &mut seen[..n]);
                    reset_completion(&mut sentinel_comp);
                    unsafe { wq.submit(&sentinel_desc) };
                    let status = poll_completion(&sentinel_comp);
                    if status != DSA_COMP_SUCCESS {
                        panic!("submit-marker-overlap drain sentinel failed: {status:#x}");
                    }

                    let submit_tail_ticks = t1.saturating_sub(marker_submit_tsc);
                    submit_tail_tsc.push(submit_tail_ticks);
                    submit_tail_ns.push(cycles_to_ns(submit_tail_ticks, tsc_freq));
                    if let Some(marker_tsc) = marker_tsc {
                        marker_visible_tsc.push(marker_tsc);
                        marker_visible_ns.push(cycles_to_ns(marker_tsc, tsc_freq));
                    }
                    completed_counts.push(outcome.completed as u64);
                    missing_counts.push((n - outcome.completed) as u64);
                    error_counts.push(outcome.errors as u64);
                }

                let submit_tail_ns_stats = stats_from_values(submit_tail_ns);
                let marker_visible_ns_stats = optional_stats(marker_visible_ns);
                let marker_visible_tsc_stats = optional_stats(marker_visible_tsc);
                let completed = stats_from_values(completed_counts);

                if !json {
                    let marker_ns = marker_visible_ns_stats
                        .as_ref()
                        .map(|stats| stats.median.to_string())
                        .unwrap_or_else(|| "-".to_string());
                    println!(
                        "{:>8} {:>8} {:>8} {:>14} {:>14} {:>10}",
                        n,
                        marker_position,
                        cadence.as_str(),
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
                    poll_cadence: cadence.as_str(),
                    submit_tail_tsc_ticks: stats_from_values(submit_tail_tsc),
                    submit_tail_ns: submit_tail_ns_stats,
                    marker_visible_tsc_ticks: marker_visible_tsc_stats,
                    marker_visible_ns: marker_visible_ns_stats,
                    marker_observed_before_final_submit_count: observed_before_t1,
                    marker_observed_before_final_submit_fraction: observed_before_t1 as f64
                        / iterations as f64,
                    completed,
                    missing: stats_from_values(missing_counts),
                    errors: stats_from_values(error_counts),
                });
            }
        }
    }
}

#[inline(always)]
fn should_poll_marker(cadence: MarkerPollCadence, submitted: usize) -> bool {
    match cadence {
        MarkerPollCadence::Every(value) => submitted % value == 0,
        MarkerPollCadence::Never => false,
    }
}

fn wait_for_marker_completion(completion: &DsaCompletionRecord, t0: u64) -> Option<u64> {
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
