// Experiment 3: traffic-class isolation ladder.
//
// Keep the same logical window and add one source of hardware traffic at a
// time. Every class still uses one logical operation per MMIO submission.
//
//   SubmitOnly
//      |
//      v
//   NoopCompletion  (+ completion writes)
//      |
//      v
//   Memmove64       (+ tiny payload DMA)
//      |
//      v
//   Memmove4K       (+ larger payload DMA)
//
// Question: which added traffic class first changes the submit/completion
// curve?
//
use hw_eval::dsa::{
    completion_flags_no_cache_control, poll_completion, reset_completion, DsaCompletionRecord,
    DsaFlags, DsaHwDesc, DSA_COMP_SUCCESS,
};
use hw_eval::submit::{cycles_to_ns, lfence, rdtscp, WqPortal};

use crate::config::{DsaOperationClass, TrafficClass};
use crate::report::{stats_from_values, TrafficClassLadderResult};

use super::common::{
    count_visible_completions, fill_descriptor, ops_per_second, optional_stats,
    reset_sample_completions, OperationSlots,
};

const TRAFFIC_CLASS_LADDER_BENCHMARK: &str = "traffic_class_ladder";

pub(crate) fn bench_traffic_class_ladder(
    wq: &WqPortal,
    windows: &[usize],
    traffic_classes: &[TrafficClass],
    iterations: usize,
    tsc_freq: u64,
    json: bool,
    results: &mut Vec<TrafficClassLadderResult>,
) {
    let Some(&max_window) = windows.iter().max() else {
        return;
    };
    let mut slots = OperationSlots::new(max_window, DsaOperationClass::Memmove4k);
    let mut submit_only_desc = DsaHwDesc::default();
    submit_only_desc.fill_noop(DsaFlags::empty());
    let mut sentinel_desc = DsaHwDesc::default();
    let mut sentinel_comp = DsaCompletionRecord::default();
    sentinel_desc.fill_noop(completion_flags_no_cache_control());
    sentinel_desc.set_completion(&mut sentinel_comp);
    let mut seen = vec![false; max_window];

    if !json {
        println!("\n=== {TRAFFIC_CLASS_LADDER_BENCHMARK} ===");
        println!(
            "{:>18} {:>8} {:>14} {:>14} {:>14}",
            "class", "window", "submit_ns", "complete_ns", "ops/sec"
        );
    }

    for &traffic_class in traffic_classes {
        fill_traffic_descriptors(&mut slots, traffic_class);
        for &window in windows {
            let mut submit_tsc = Vec::with_capacity(iterations);
            let mut submit_ns = Vec::with_capacity(iterations);
            let mut completion_tsc = Vec::new();
            let mut completion_ns = Vec::new();
            let mut completed_counts = Vec::new();
            let mut missing_counts = Vec::new();
            let mut error_counts = Vec::new();
            let mut ops_per_sec = Vec::with_capacity(iterations);

            for _ in 0..iterations {
                if traffic_class == TrafficClass::SubmitOnly {
                    reset_completion(&mut sentinel_comp);

                    lfence();
                    let t0 = rdtscp().0;
                    for _ in 0..window {
                        unsafe { wq.submit(&submit_only_desc) };
                    }
                    let submit_ticks = rdtscp().0 - t0;

                    unsafe { wq.submit(&sentinel_desc) };
                    let status = poll_completion(&sentinel_comp);
                    if status != DSA_COMP_SUCCESS {
                        panic!("traffic-class-ladder submit-only sentinel failed: {status:#x}");
                    }

                    submit_tsc.push(submit_ticks);
                    submit_ns.push(cycles_to_ns(submit_ticks, tsc_freq));
                    ops_per_sec.push(ops_per_second(window as u64, submit_ticks, tsc_freq));
                    continue;
                }

                reset_sample_completions(&mut slots.completions[..window]);

                lfence();
                let t0 = rdtscp().0;
                for index in 0..window {
                    unsafe { wq.submit(&slots.descriptors[index]) };
                }
                let t1 = rdtscp().0;
                let outcome =
                    count_visible_completions(&slots.completions[..window], &mut seen[..window]);
                let t2 = rdtscp().0;
                reset_completion(&mut sentinel_comp);
                unsafe { wq.submit(&sentinel_desc) };
                let status = poll_completion(&sentinel_comp);
                if status != DSA_COMP_SUCCESS {
                    panic!("traffic-class-ladder drain sentinel failed: {status:#x}");
                }

                let submit_ticks = t1 - t0;
                let complete_ticks = t2 - t1;
                submit_tsc.push(submit_ticks);
                submit_ns.push(cycles_to_ns(submit_ticks, tsc_freq));
                completion_tsc.push(complete_ticks);
                completion_ns.push(cycles_to_ns(complete_ticks, tsc_freq));
                completed_counts.push(outcome.completed as u64);
                missing_counts.push((window - outcome.completed) as u64);
                error_counts.push(outcome.errors as u64);
                ops_per_sec.push(ops_per_second(window as u64, t2 - t0, tsc_freq));
            }

            let submit_ns_stats = stats_from_values(submit_ns);
            let completion_ns_stats = optional_stats(completion_ns);
            let ops_per_sec_stats = stats_from_values(ops_per_sec);

            if !json {
                println!(
                    "{:>18} {:>8} {:>14} {:>14} {:>14}",
                    traffic_class.as_str(),
                    window,
                    submit_ns_stats.median,
                    completion_ns_stats
                        .as_ref()
                        .map(|stats| stats.median.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    ops_per_sec_stats.median
                );
            }

            results.push(TrafficClassLadderResult {
                benchmark: TRAFFIC_CLASS_LADDER_BENCHMARK.to_string(),
                traffic_class: traffic_class.as_str().to_string(),
                operation_size: traffic_class.operation_size(),
                window,
                submit_tsc_ticks: stats_from_values(submit_tsc),
                submit_ns: submit_ns_stats,
                completion_visible_tsc_ticks: optional_stats(completion_tsc),
                completion_visible_ns: completion_ns_stats,
                completed: optional_stats(completed_counts),
                missing: optional_stats(missing_counts),
                errors: optional_stats(error_counts),
                ops_per_sec: ops_per_sec_stats,
            });
        }
    }
}

fn fill_traffic_descriptors(slots: &mut OperationSlots, traffic_class: TrafficClass) {
    let operation = match traffic_class {
        TrafficClass::SubmitOnly | TrafficClass::NoopCompletion => DsaOperationClass::Noop,
        TrafficClass::Memmove64 => DsaOperationClass::Memmove64,
        TrafficClass::Memmove4k => DsaOperationClass::Memmove4k,
    };
    let payload_size = operation.payload_size();

    for slot in 0..slots.descriptors.len() {
        fill_descriptor(
            &mut slots.descriptors[slot],
            &mut slots.completions[slot],
            &mut slots.sources,
            &mut slots.destinations,
            slot,
            payload_size,
            operation,
        );
    }
}
