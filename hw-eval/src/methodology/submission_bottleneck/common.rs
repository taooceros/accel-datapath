use std::time::Instant;

use hw_eval::dsa::{
    completion_flags_no_cache_control, reset_completion, DsaCompletionRecord, DsaCompletionStatus,
    DsaHwDesc, DSA_COMP_NONE, DSA_COMP_SUCCESS,
};

use crate::config::DsaOperationClass;
use crate::report::{stats_from_values, LatencyStats};

pub(super) const COMPLETION_TIMEOUT_NS: u128 = 1_000_000;
pub(super) const TIMEOUT_CHECK_STRIDE: u64 = 256;

pub(super) struct OperationSlots {
    pub(super) descriptors: Vec<DsaHwDesc>,
    pub(super) completions: Vec<DsaCompletionRecord>,
    pub(super) sources: Vec<u8>,
    pub(super) destinations: Vec<u8>,
}

impl OperationSlots {
    pub(super) fn new(count: usize, operation: DsaOperationClass) -> Self {
        let mut descriptors = vec![DsaHwDesc::default(); count];
        let mut completions = vec![DsaCompletionRecord::default(); count];

        let payload_size = operation.payload_size();
        let mut sources = vec![0xa5; count * payload_size];
        let mut destinations = vec![0; count * payload_size];

        for slot in 0..count {
            fill_descriptor(
                &mut descriptors[slot],
                &mut completions[slot],
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
            sources,
            destinations,
        }
    }
}

pub(super) fn fill_descriptor(
    desc: &mut DsaHwDesc,
    completion: &mut DsaCompletionRecord,
    sources: &mut [u8],
    destinations: &mut [u8],
    slot: usize,
    payload_size: usize,
    operation: DsaOperationClass,
) {
    match operation {
        DsaOperationClass::Noop => desc.fill_noop(completion_flags_no_cache_control()),
        DsaOperationClass::Memmove64 | DsaOperationClass::Memmove4k => {
            let offset = slot * payload_size;
            let src = sources.as_ptr().wrapping_add(offset);
            let dst = destinations.as_mut_ptr().wrapping_add(offset);
            desc.fill_memmove(src, dst, payload_size as u32);
        }
    }
    desc.set_completion(completion);
}

#[inline(always)]
pub(super) fn reset_sample_completions(completions: &mut [DsaCompletionRecord]) {
    for completion in completions {
        reset_completion(completion);
    }
}

#[derive(Default)]
pub(super) struct CompletionOutcome {
    pub(super) completed: usize,
    pub(super) errors: usize,
}

impl CompletionOutcome {
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

pub(super) fn count_visible_completions(
    completions: &[DsaCompletionRecord],
    seen: &mut [bool],
) -> CompletionOutcome {
    seen.fill(false);

    let mut outcome = CompletionOutcome::default();
    let start = Instant::now();
    let mut spins = 0_u64;

    loop {
        for (index, completion) in completions.iter().enumerate() {
            if seen[index] {
                continue;
            }

            if outcome.record_status(completion.status()) {
                seen[index] = true;
            }
        }

        if outcome.completed == completions.len() {
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

pub(super) fn scan_visible_completions(completions: &[DsaCompletionRecord]) -> CompletionOutcome {
    let mut outcome = CompletionOutcome::default();

    for completion in completions {
        outcome.record_status(completion.status());
    }

    outcome
}

pub(super) fn optional_stats(values: Vec<u64>) -> Option<LatencyStats> {
    if values.is_empty() {
        None
    } else {
        Some(stats_from_values(values))
    }
}

pub(super) fn ops_per_second(operations: u64, cycles: u64, tsc_freq: u64) -> u64 {
    if cycles == 0 {
        0
    } else {
        ((operations as u128 * tsc_freq as u128) / cycles as u128) as u64
    }
}
