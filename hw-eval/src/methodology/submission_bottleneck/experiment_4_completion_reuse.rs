// Experiment 4: completion handling and reuse policy.
//
// Hold a fixed logical window open, then vary only how software discovers,
// resets, and resubmits completed slots.
//
//   fill window
//      |
//      v
//   [poll / harvest completions] -> [reset completion records] -> [resubmit]
//      ^                                                            |
//      |---------------- closed-loop steady window -----------------|
//
// Question: is sustained batch-size-1 throughput limited by CPU completion
// discovery, reset timing, cacheline layout, or resubmit policy?
//
use std::time::Instant;

use hw_eval::dsa::{
    completion_flags_no_cache_control, reset_completion, DsaCompletionRecord, DsaCompletionStatus,
    DsaHwDesc, DSA_COMP_NONE, DSA_COMP_SUCCESS,
};
use hw_eval::submit::{cycles_to_ns, rdtscp, WqPortal};

use crate::config::{CompletionReusePolicy, DsaOperationClass};
use crate::report::{stats_from_values, CompletionReusePolicyResult};

use super::common::{ops_per_second, optional_stats, TIMEOUT_CHECK_STRIDE};

const COMPLETION_REUSE_POLICY_BENCHMARK: &str = "completion_reuse_policy";
const COMPLETION_REUSE_TIMEOUT_NS: u128 = 1_000_000_000;

pub(crate) fn bench_completion_reuse_policy(
    wq: &WqPortal,
    policies: &[CompletionReusePolicy],
    window: usize,
    operation: DsaOperationClass,
    iterations: usize,
    tsc_freq: u64,
    json: bool,
    results: &mut Vec<CompletionReusePolicyResult>,
) {
    if window == 0 {
        return;
    }

    if !json {
        println!(
            "\n=== {COMPLETION_REUSE_POLICY_BENCHMARK} ({}, window={window}) ===",
            operation.as_str()
        );
        println!(
            "{:>20} {:>12} {:>14} {:>14}",
            "policy", "completed", "ops/sec", "polls/comp"
        );
    }

    for &policy in policies {
        let mut slots = CompletionReuseSlots::new(window, operation, policy);
        let target = iterations.max(window);
        let measurement = run_completion_reuse_sample(wq, &mut slots, policy, target, tsc_freq);

        if !json {
            println!(
                "{:>20} {:>12} {:>14.0} {:>14.2}",
                policy.as_str(),
                measurement.completed,
                measurement.ops_per_sec,
                measurement.polls_per_completion
            );
        }

        results.push(CompletionReusePolicyResult {
            benchmark: COMPLETION_REUSE_POLICY_BENCHMARK.to_string(),
            operation_class: operation.as_str().to_string(),
            window,
            policy: policy.as_str().to_string(),
            operations_completed: measurement.completed,
            ops_per_sec: measurement.ops_per_sec,
            polls_per_completion: measurement.polls_per_completion,
            completion_harvest_tsc_ticks: stats_from_values(measurement.harvest_tsc),
            completion_harvest_ns: stats_from_values(measurement.harvest_ns),
            reset_to_submit_tsc_ticks: optional_stats(measurement.reset_to_submit_tsc),
            reset_to_submit_ns: optional_stats(measurement.reset_to_submit_ns),
            completed: measurement.completed,
            missing: measurement.missing,
            errors: measurement.errors,
        });
    }
}

#[repr(align(64))]
#[derive(Clone, Copy)]
struct PaddedCompletion {
    record: DsaCompletionRecord,
}

impl Default for PaddedCompletion {
    fn default() -> Self {
        Self {
            record: DsaCompletionRecord::default(),
        }
    }
}

struct CompletionReuseSlots {
    descriptors: Vec<DsaHwDesc>,
    completions: Vec<DsaCompletionRecord>,
    padded_completions: Vec<PaddedCompletion>,
    sources: Vec<u8>,
    destinations: Vec<u8>,
    padded: bool,
}

impl CompletionReuseSlots {
    fn new(count: usize, operation: DsaOperationClass, policy: CompletionReusePolicy) -> Self {
        let descriptors = vec![DsaHwDesc::default(); count];
        let completions = vec![DsaCompletionRecord::default(); count];
        let padded_completions = vec![PaddedCompletion::default(); count];
        let payload_size = operation.payload_size();
        let sources = vec![0xa5; count * payload_size];
        let destinations = vec![0; count * payload_size];
        let padded = policy == CompletionReusePolicy::PaddedRoundRobin;

        let mut slots = Self {
            descriptors,
            completions,
            padded_completions,
            sources,
            destinations,
            padded,
        };

        for slot in 0..count {
            slots.fill(slot, operation);
        }

        slots
    }

    fn fill(&mut self, slot: usize, operation: DsaOperationClass) {
        match operation {
            DsaOperationClass::Noop => {
                self.descriptors[slot].fill_noop(completion_flags_no_cache_control())
            }
            DsaOperationClass::Memmove64 | DsaOperationClass::Memmove4k => {
                let payload_size = operation.payload_size();
                let offset = slot * payload_size;
                self.descriptors[slot].fill_memmove(
                    self.sources.as_ptr().wrapping_add(offset),
                    self.destinations.as_mut_ptr().wrapping_add(offset),
                    payload_size as u32,
                );
            }
        }

        if self.padded {
            self.descriptors[slot].set_completion(&mut self.padded_completions[slot].record);
        } else {
            self.descriptors[slot].set_completion(&mut self.completions[slot]);
        }
    }

    fn reset(&mut self, slot: usize) {
        if self.padded {
            reset_completion(&mut self.padded_completions[slot].record);
        } else {
            reset_completion(&mut self.completions[slot]);
        }
    }

    fn status(&self, slot: usize) -> u8 {
        if self.padded {
            self.padded_completions[slot].record.status()
        } else {
            self.completions[slot].status()
        }
    }
}

struct CompletionReuseMeasurement {
    completed: u64,
    missing: u64,
    errors: u64,
    ops_per_sec: f64,
    polls_per_completion: f64,
    harvest_tsc: Vec<u64>,
    harvest_ns: Vec<u64>,
    reset_to_submit_tsc: Vec<u64>,
    reset_to_submit_ns: Vec<u64>,
}

fn run_completion_reuse_sample(
    wq: &WqPortal,
    slots: &mut CompletionReuseSlots,
    policy: CompletionReusePolicy,
    target: usize,
    tsc_freq: u64,
) -> CompletionReuseMeasurement {
    let window = slots.descriptors.len();
    for slot in 0..window {
        slots.reset(slot);
        unsafe { wq.submit(&slots.descriptors[slot]) };
    }

    let mut completed = 0_u64;
    let mut errors = 0_u64;
    let mut polls = 0_u64;
    let mut harvest_tsc = Vec::with_capacity(target);
    let mut harvest_ns = Vec::with_capacity(target);
    let mut reset_to_submit_tsc = Vec::new();
    let mut reset_to_submit_ns = Vec::new();
    let start = rdtscp().0;
    let timeout_start = Instant::now();

    match policy {
        CompletionReusePolicy::PollOnly => {
            let mut seen = vec![false; window];
            let mut outstanding = window;

            while completed < target as u64 && !completion_reuse_timed_out(timeout_start) {
                let mut observed_completion = false;

                for slot in 0..window {
                    if seen[slot] {
                        continue;
                    }

                    polls += 1;
                    let t0 = rdtscp().0;
                    let status = slots.status(slot);
                    if status == DSA_COMP_NONE {
                        continue;
                    }

                    let harvest_ticks = rdtscp().0 - t0;
                    harvest_tsc.push(harvest_ticks);
                    harvest_ns.push(cycles_to_ns(harvest_ticks, tsc_freq));
                    completed += 1;
                    observed_completion = true;
                    outstanding -= 1;

                    if DsaCompletionStatus::mask(status) != DSA_COMP_SUCCESS {
                        errors += 1;
                    }

                    seen[slot] = true;
                    if completed == target as u64 {
                        break;
                    }
                }

                if outstanding == 0 && completed < target as u64 {
                    for slot in 0..window {
                        slots.reset(slot);
                        unsafe { wq.submit(&slots.descriptors[slot]) };
                        seen[slot] = false;
                    }
                    outstanding = window;
                }

                if !observed_completion {
                    core::hint::spin_loop();
                }
            }
        }
        CompletionReusePolicy::PaddedRoundRobin => {
            let mut slot = 0;
            while completed < target as u64 && !completion_reuse_timed_out(timeout_start) {
                polls += 1;
                let t0 = rdtscp().0;
                let status = slots.status(slot);
                if status != DSA_COMP_NONE {
                    let harvest_ticks = rdtscp().0 - t0;
                    harvest_tsc.push(harvest_ticks);
                    harvest_ns.push(cycles_to_ns(harvest_ticks, tsc_freq));
                    completed += 1;
                    if DsaCompletionStatus::mask(status) != DSA_COMP_SUCCESS {
                        errors += 1;
                    }

                    let reset_t0 = rdtscp().0;
                    slots.reset(slot);
                    unsafe { wq.submit(&slots.descriptors[slot]) };
                    let reset_ticks = rdtscp().0 - reset_t0;
                    reset_to_submit_tsc.push(reset_ticks);
                    reset_to_submit_ns.push(cycles_to_ns(reset_ticks, tsc_freq));
                }
                slot += 1;
                if slot == window {
                    slot = 0;
                }
                core::hint::spin_loop();
            }
        }
        CompletionReusePolicy::PackedScan => {
            while completed < target as u64 && !completion_reuse_timed_out(timeout_start) {
                for slot in 0..window {
                    polls += 1;
                    let t0 = rdtscp().0;
                    let status = slots.status(slot);
                    if status == DSA_COMP_NONE {
                        continue;
                    }
                    let harvest_ticks = rdtscp().0 - t0;
                    harvest_tsc.push(harvest_ticks);
                    harvest_ns.push(cycles_to_ns(harvest_ticks, tsc_freq));
                    completed += 1;
                    if DsaCompletionStatus::mask(status) != DSA_COMP_SUCCESS {
                        errors += 1;
                    }

                    let reset_t0 = rdtscp().0;
                    slots.reset(slot);
                    unsafe { wq.submit(&slots.descriptors[slot]) };
                    let reset_ticks = rdtscp().0 - reset_t0;
                    reset_to_submit_tsc.push(reset_ticks);
                    reset_to_submit_ns.push(cycles_to_ns(reset_ticks, tsc_freq));

                    if completed == target as u64 {
                        break;
                    }
                }
                core::hint::spin_loop();
            }
        }
        CompletionReusePolicy::DelayedReset | CompletionReusePolicy::BatchHarvest => {
            let batch = if policy == CompletionReusePolicy::BatchHarvest {
                16.min(window)
            } else {
                window
            };
            let mut ready = Vec::with_capacity(batch);

            while completed < target as u64 && !completion_reuse_timed_out(timeout_start) {
                ready.clear();

                for slot in 0..window {
                    polls += 1;
                    let t0 = rdtscp().0;
                    let status = slots.status(slot);
                    if status == DSA_COMP_NONE {
                        continue;
                    }
                    let harvest_ticks = rdtscp().0 - t0;
                    harvest_tsc.push(harvest_ticks);
                    harvest_ns.push(cycles_to_ns(harvest_ticks, tsc_freq));
                    completed += 1;
                    if DsaCompletionStatus::mask(status) != DSA_COMP_SUCCESS {
                        errors += 1;
                    }
                    ready.push(slot);

                    if ready.len() == batch || completed == target as u64 {
                        break;
                    }
                }

                if !ready.is_empty() {
                    let reset_t0 = rdtscp().0;
                    for &slot in &ready {
                        slots.reset(slot);
                    }
                    for &slot in &ready {
                        unsafe { wq.submit(&slots.descriptors[slot]) };
                    }
                    let reset_ticks = rdtscp().0 - reset_t0;
                    let reset_ticks_per_completion = reset_ticks / ready.len() as u64;
                    reset_to_submit_tsc.push(reset_ticks_per_completion);
                    reset_to_submit_ns.push(cycles_to_ns(reset_ticks_per_completion, tsc_freq));
                }

                core::hint::spin_loop();
            }
        }
    }

    let elapsed = rdtscp().0 - start;
    let ops_per_sec = ops_per_second(completed, elapsed, tsc_freq) as f64;
    let polls_per_completion = if completed == 0 {
        0.0
    } else {
        polls as f64 / completed as f64
    };

    let mut drain_errors = 0_u64;
    for slot in 0..window {
        if slots.status(slot) == DSA_COMP_NONE {
            match poll_slot_completion(slots, slot) {
                Some(status) if DsaCompletionStatus::mask(status) != DSA_COMP_SUCCESS => {
                    drain_errors += 1;
                }
                Some(_) => {}
                None => drain_errors += 1,
            }
        }
    }

    CompletionReuseMeasurement {
        completed,
        missing: (target as u64).saturating_sub(completed),
        errors: errors + drain_errors,
        ops_per_sec,
        polls_per_completion,
        harvest_tsc,
        harvest_ns,
        reset_to_submit_tsc,
        reset_to_submit_ns,
    }
}

fn poll_slot_completion(slots: &CompletionReuseSlots, slot: usize) -> Option<u8> {
    let start = Instant::now();
    let mut spins = 0_u64;

    loop {
        let status = slots.status(slot);
        if status != DSA_COMP_NONE {
            return Some(status);
        }

        spins = spins.wrapping_add(1);
        if spins & (TIMEOUT_CHECK_STRIDE - 1) == 0 && completion_reuse_timed_out(start) {
            return None;
        }

        core::hint::spin_loop();
    }
}

#[inline(always)]
fn completion_reuse_timed_out(start: Instant) -> bool {
    start.elapsed().as_nanos() >= COMPLETION_REUSE_TIMEOUT_NS
}
