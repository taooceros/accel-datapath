// Experiment 5: submit-admission probe.
//
// Push N unique completion-bearing descriptors without software in-flight
// accounting, then count whether every logical operation completes. This is
// the blind-push correctness gate behind the `submit-admission` selector.
//
//   blind push phase:       post-submit accounting:
//   [1][2][3] ... [N]  ->   completed / missing / errors
//      no polling while submitting
//
// Question: does pushing past the nominal WQ depth lose descriptors, report
// descriptor errors, or simply backpressure the submit loop?

use std::time::Instant;

use hw_eval::dsa::{
    completion_flags_no_cache_control, poll_completion, reset_completion, DsaCompletionRecord,
    DsaCompletionStatus, DsaHwDesc, DSA_COMP_NONE, DSA_COMP_SUCCESS,
};
use hw_eval::submit::{cycles_to_ns, lfence, rdtscp, WqPortal};

use crate::report::{stats_from_values, AdmissionResult};

const ADMISSION_COMPLETION_TIMEOUT_NS: u128 = 50_000;

pub(crate) fn bench_submit_admission_probe(
    wq: &WqPortal,
    iterations: usize,
    submit_bursts: &[usize],
    tsc_freq: u64,
    json: bool,
    results: &mut Vec<AdmissionResult>,
) {
    let Some(&max_burst) = submit_bursts.iter().max() else {
        return;
    };

    let (descs, mut comps) = prepare_admission_descriptors(max_burst);
    let mut seen = vec![false; max_burst];
    let mut sentinel_desc = DsaHwDesc::default();
    let mut sentinel_comp = DsaCompletionRecord::default();

    sentinel_desc.fill_noop(completion_flags_no_cache_control());
    sentinel_desc.set_completion(&mut sentinel_comp);

    if !json {
        println!("\n=== submit_admission_distinct ===");
        println!(
            "{:>8} {:>14} {:>14} {:>14} {:>14}",
            "burst", "completed", "missing", "tsc/batch", "ns/batch"
        );
    }

    for &burst in submit_bursts {
        let mut completed_counts = Vec::with_capacity(iterations);
        let mut missing_counts = Vec::with_capacity(iterations);
        let mut error_counts = Vec::with_capacity(iterations);
        let mut submit_tsc = Vec::with_capacity(iterations);
        let mut submit_ns = Vec::with_capacity(iterations);

        for _ in 0..iterations {
            for comp in &mut comps[..burst] {
                reset_completion(comp);
            }

            lfence();
            let tsc_start = rdtscp().0;
            for desc in &descs[..burst] {
                unsafe { wq.submit(desc) };
            }
            let tsc_ticks = rdtscp().0 - tsc_start;
            let submit_tsc_ns = cycles_to_ns(tsc_ticks, tsc_freq);

            let bounded_outcome = count_admission_completions(&comps[..burst], &mut seen[..burst]);
            reset_completion(&mut sentinel_comp);
            drain_admission_queue(wq, &sentinel_desc, &mut sentinel_comp, burst);

            let outcome = if bounded_outcome.completed == burst {
                bounded_outcome
            } else {
                scan_admission_completions(&comps[..burst])
            };

            completed_counts.push(outcome.completed as u64);
            missing_counts.push((burst - outcome.completed) as u64);
            error_counts.push(outcome.errors as u64);
            submit_tsc.push(tsc_ticks);
            submit_ns.push(submit_tsc_ns);
        }

        let completed = stats_from_values(completed_counts);
        let missing = stats_from_values(missing_counts);
        let errors = stats_from_values(error_counts);
        let submit_tsc_ticks = stats_from_values(submit_tsc);
        let submit_ns = stats_from_values(submit_ns);

        if !json {
            println!(
                "{:>8} {:>14} {:>14} {:>14} {:>14}",
                burst, completed.median, missing.median, submit_tsc_ticks.median, submit_ns.median
            );
        }

        results.push(AdmissionResult {
            benchmark: "submit_admission_distinct".to_string(),
            burst_size: burst,
            submitted: burst,
            completed,
            missing,
            errors,
            submit_tsc_ticks,
            submit_ns,
        });
    }
}

fn prepare_admission_descriptors(count: usize) -> (Vec<DsaHwDesc>, Vec<DsaCompletionRecord>) {
    let mut comps: Vec<DsaCompletionRecord> =
        (0..count).map(|_| DsaCompletionRecord::default()).collect();
    let mut descs: Vec<DsaHwDesc> = (0..count).map(|_| DsaHwDesc::default()).collect();

    for (desc, comp) in descs.iter_mut().zip(comps.iter_mut()) {
        desc.fill_noop(completion_flags_no_cache_control());
        desc.set_completion(comp);
    }

    (descs, comps)
}

#[derive(Default)]
struct AdmissionCompletionOutcome {
    completed: usize,
    errors: usize,
}

impl AdmissionCompletionOutcome {
    fn record_status(&mut self, status: u8) -> bool {
        if status == DSA_COMP_NONE {
            return false;
        }

        self.completed += 1;
        if DsaCompletionStatus::mask(status) != DSA_COMP_SUCCESS {
            self.errors += 1;
        }
        true
    }
}

fn count_admission_completions(
    comps: &[DsaCompletionRecord],
    seen: &mut [bool],
) -> AdmissionCompletionOutcome {
    seen.fill(false);
    let mut outcome = AdmissionCompletionOutcome::default();
    let start = Instant::now();

    loop {
        for (index, comp) in comps.iter().enumerate() {
            if seen[index] {
                continue;
            }

            if outcome.record_status(comp.status()) {
                seen[index] = true;
            }
        }

        if outcome.completed == comps.len()
            || start.elapsed().as_nanos() >= ADMISSION_COMPLETION_TIMEOUT_NS
        {
            return outcome;
        }

        core::hint::spin_loop();
    }
}

fn scan_admission_completions(comps: &[DsaCompletionRecord]) -> AdmissionCompletionOutcome {
    let mut outcome = AdmissionCompletionOutcome::default();

    for comp in comps {
        outcome.record_status(comp.status());
    }

    outcome
}

fn drain_admission_queue(
    wq: &WqPortal,
    sentinel_desc: &DsaHwDesc,
    sentinel_comp: &mut DsaCompletionRecord,
    burst: usize,
) {
    unsafe { wq.submit(sentinel_desc) };
    let status = poll_completion(sentinel_comp);
    if status != DSA_COMP_SUCCESS {
        panic!(
            "DSA submit-admission drain sentinel failed: status {status:#x} \
             (benchmark=submit_admission_distinct, burst={burst})"
        );
    }
}
