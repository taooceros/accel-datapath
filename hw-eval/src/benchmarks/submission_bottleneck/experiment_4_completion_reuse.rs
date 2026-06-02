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
use hw_eval::submit::{cycles_to_ns, WqPortal};

use crate::config::{CompletionReusePolicy, DsaOperationClass};
use crate::report::{stats_from_values, CompletionReusePolicyResult};

use super::common::{
    dsa_operation_payload_size, measured_call, ops_per_second, optional_stats, TIMEOUT_CHECK_STRIDE,
};

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
        let payload_size = dsa_operation_payload_size(operation);
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
                let payload_size = dsa_operation_payload_size(operation);
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

    let mut acc = CompletionReuseAccumulator::new(target, tsc_freq);

    let policy_run = measured_call(|| {
        let timeout_start = Instant::now();
        match policy {
            CompletionReusePolicy::PollOnly => {
                run_poll_only_policy(wq, slots, target, timeout_start, &mut acc);
            }
            CompletionReusePolicy::PaddedRoundRobin => {
                run_padded_round_robin_policy(wq, slots, target, timeout_start, &mut acc);
            }
            CompletionReusePolicy::PackedScan => {
                run_packed_scan_policy(wq, slots, target, timeout_start, &mut acc);
            }
            CompletionReusePolicy::DelayedReset => {
                run_delayed_reset_or_batch_harvest_policy(
                    wq,
                    slots,
                    target,
                    timeout_start,
                    window,
                    &mut acc,
                );
            }
            CompletionReusePolicy::BatchHarvest => {
                run_delayed_reset_or_batch_harvest_policy(
                    wq,
                    slots,
                    target,
                    timeout_start,
                    16.min(window),
                    &mut acc,
                );
            }
        }
    });

    let elapsed = policy_run.elapsed_tsc();
    let ops_per_sec = ops_per_second(acc.completed, elapsed, tsc_freq) as f64;
    let polls_per_completion = acc.polls_per_completion();

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
        completed: acc.completed,
        missing: (target as u64).saturating_sub(acc.completed),
        errors: acc.errors + drain_errors,
        ops_per_sec,
        polls_per_completion,
        harvest_tsc: acc.harvest_tsc,
        harvest_ns: acc.harvest_ns,
        reset_to_submit_tsc: acc.reset_to_submit_tsc,
        reset_to_submit_ns: acc.reset_to_submit_ns,
    }
}

struct CompletionReuseAccumulator {
    completed: u64,
    errors: u64,
    polls: u64,
    tsc_freq: u64,
    harvest_tsc: Vec<u64>,
    harvest_ns: Vec<u64>,
    reset_to_submit_tsc: Vec<u64>,
    reset_to_submit_ns: Vec<u64>,
}

impl CompletionReuseAccumulator {
    fn new(target: usize, tsc_freq: u64) -> Self {
        Self {
            completed: 0,
            errors: 0,
            polls: 0,
            tsc_freq,
            harvest_tsc: Vec::with_capacity(target),
            harvest_ns: Vec::with_capacity(target),
            reset_to_submit_tsc: Vec::new(),
            reset_to_submit_ns: Vec::new(),
        }
    }

    fn target_reached(&self, target: usize) -> bool {
        self.completed >= target as u64
    }

    fn record_harvest(&mut self, status: u8, harvest_ticks: u64) {
        self.harvest_tsc.push(harvest_ticks);
        self.harvest_ns
            .push(cycles_to_ns(harvest_ticks, self.tsc_freq));
        self.completed += 1;
        if DsaCompletionStatus::mask(status) != DSA_COMP_SUCCESS {
            self.errors += 1;
        }
    }

    fn record_reset_to_submit(&mut self, reset_ticks: u64) {
        self.reset_to_submit_tsc.push(reset_ticks);
        self.reset_to_submit_ns
            .push(cycles_to_ns(reset_ticks, self.tsc_freq));
    }

    fn polls_per_completion(&self) -> f64 {
        if self.completed == 0 {
            0.0
        } else {
            self.polls as f64 / self.completed as f64
        }
    }
}

fn poll_completion_slot(
    slots: &CompletionReuseSlots,
    slot: usize,
    acc: &mut CompletionReuseAccumulator,
) -> Option<u8> {
    acc.polls += 1;
    let status = measured_call(|| slots.status(slot));
    if status.value == DSA_COMP_NONE {
        None
    } else {
        acc.record_harvest(status.value, status.elapsed_tsc());
        Some(status.value)
    }
}

fn reset_and_submit_slot(
    wq: &WqPortal,
    slots: &mut CompletionReuseSlots,
    slot: usize,
    acc: &mut CompletionReuseAccumulator,
) {
    let reset = measured_call(|| {
        slots.reset(slot);
        unsafe { wq.submit(&slots.descriptors[slot]) };
    });
    acc.record_reset_to_submit(reset.elapsed_tsc());
}

fn reset_and_submit_ready_slots(
    wq: &WqPortal,
    slots: &mut CompletionReuseSlots,
    ready: &[usize],
    acc: &mut CompletionReuseAccumulator,
) {
    let reset = measured_call(|| {
        for &slot in ready {
            slots.reset(slot);
        }
        for &slot in ready {
            unsafe { wq.submit(&slots.descriptors[slot]) };
        }
    });
    let reset_ticks_per_completion = reset.elapsed_tsc() / ready.len() as u64;
    acc.record_reset_to_submit(reset_ticks_per_completion);
}

fn run_poll_only_policy(
    wq: &WqPortal,
    slots: &mut CompletionReuseSlots,
    target: usize,
    timeout_start: Instant,
    acc: &mut CompletionReuseAccumulator,
) {
    let window = slots.descriptors.len();
    let mut seen = vec![false; window];
    let mut outstanding = window;

    while !acc.target_reached(target) && !completion_reuse_timed_out(timeout_start) {
        let mut observed_completion = false;

        for (slot, slot_seen) in seen.iter_mut().enumerate() {
            if *slot_seen {
                continue;
            }

            let Some(_) = poll_completion_slot(slots, slot, acc) else {
                continue;
            };

            observed_completion = true;
            outstanding -= 1;
            *slot_seen = true;
            if acc.target_reached(target) {
                break;
            }
        }

        if outstanding == 0 && !acc.target_reached(target) {
            for (slot, slot_seen) in seen.iter_mut().enumerate() {
                slots.reset(slot);
                unsafe { wq.submit(&slots.descriptors[slot]) };
                *slot_seen = false;
            }
            outstanding = window;
        }

        if !observed_completion {
            core::hint::spin_loop();
        }
    }
}

fn run_padded_round_robin_policy(
    wq: &WqPortal,
    slots: &mut CompletionReuseSlots,
    target: usize,
    timeout_start: Instant,
    acc: &mut CompletionReuseAccumulator,
) {
    let window = slots.descriptors.len();
    let mut slot = 0;
    while !acc.target_reached(target) && !completion_reuse_timed_out(timeout_start) {
        if poll_completion_slot(slots, slot, acc).is_some() {
            reset_and_submit_slot(wq, slots, slot, acc);
        }
        slot += 1;
        if slot == window {
            slot = 0;
        }
        core::hint::spin_loop();
    }
}

fn run_packed_scan_policy(
    wq: &WqPortal,
    slots: &mut CompletionReuseSlots,
    target: usize,
    timeout_start: Instant,
    acc: &mut CompletionReuseAccumulator,
) {
    let window = slots.descriptors.len();
    while !acc.target_reached(target) && !completion_reuse_timed_out(timeout_start) {
        for slot in 0..window {
            if poll_completion_slot(slots, slot, acc).is_none() {
                continue;
            }

            reset_and_submit_slot(wq, slots, slot, acc);
            if acc.target_reached(target) {
                break;
            }
        }
        core::hint::spin_loop();
    }
}

fn run_delayed_reset_or_batch_harvest_policy(
    wq: &WqPortal,
    slots: &mut CompletionReuseSlots,
    target: usize,
    timeout_start: Instant,
    batch: usize,
    acc: &mut CompletionReuseAccumulator,
) {
    let window = slots.descriptors.len();
    let mut ready = Vec::with_capacity(batch);

    while !acc.target_reached(target) && !completion_reuse_timed_out(timeout_start) {
        ready.clear();

        for slot in 0..window {
            if poll_completion_slot(slots, slot, acc).is_none() {
                continue;
            }

            ready.push(slot);
            if ready.len() == batch || acc.target_reached(target) {
                break;
            }
        }

        if !ready.is_empty() {
            reset_and_submit_ready_slots(wq, slots, &ready, acc);
        }

        core::hint::spin_loop();
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
