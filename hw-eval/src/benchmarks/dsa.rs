use std::sync::Barrier;
use std::thread;
use std::time::Instant;

use hw_eval::dsa::*;
use hw_eval::submit::{cycles_to_ns, flush_range, lfence, mfence, rdtscp, WqPortal};

use super::submission_bottleneck::{self, BottleneckRequest};
use crate::config::{BenchmarkKind, SubmissionBottleneckConfig, SubmitOnlyMode};
use crate::report::{compute_stats, LatencyResult, SubmissionBottleneckResults, ThroughputResult};
use crate::timing::MeasurementTimers;

pub(crate) struct DsaBenchmarkRequest<'a> {
    pub(crate) wq: &'a WqPortal,
    pub(crate) sizes: &'a [usize],
    pub(crate) iterations: usize,
    pub(crate) benchmark: BenchmarkKind,
    pub(crate) submit_mode: SubmitOnlyMode,
    pub(crate) submit_bursts: &'a [usize],
    pub(crate) submission_bottleneck: &'a SubmissionBottleneckConfig,
    pub(crate) max_concurrency: usize,
    pub(crate) submit_threads: usize,
    pub(crate) tsc_freq: u64,
    pub(crate) cold: bool,
    pub(crate) json: bool,
}

pub(crate) struct DsaBenchmarkResults<'a> {
    pub(crate) latency: &'a mut Vec<LatencyResult>,
    pub(crate) throughput: &'a mut Vec<ThroughputResult>,
    pub(crate) submission_bottleneck: &'a mut SubmissionBottleneckResults,
}

pub(crate) fn run_dsa_benchmarks(
    request: DsaBenchmarkRequest<'_>,
    results: DsaBenchmarkResults<'_>,
) {
    let DsaBenchmarkRequest {
        wq,
        sizes,
        iterations,
        benchmark,
        submit_mode,
        submit_bursts,
        submission_bottleneck: bottleneck_config,
        max_concurrency,
        submit_threads,
        tsc_freq,
        cold,
        json,
    } = request;

    let DsaBenchmarkResults {
        latency: latency_results,
        throughput: throughput_results,
        submission_bottleneck: submission_bottleneck_results,
    } = results;
    if benchmark == BenchmarkKind::SubmitOnly {
        bench_submit_only_workloads(
            wq,
            iterations,
            submit_mode,
            submit_bursts,
            tsc_freq,
            json,
            latency_results,
        );
        return;
    }

    if submission_bottleneck::run(
        BottleneckRequest {
            wq,
            benchmark,
            config: bottleneck_config,
            submit_bursts,
            iterations,
            tsc_freq,
            json,
        },
        submission_bottleneck_results,
    ) {
        return;
    }

    bench_noop_latency(wq, iterations, tsc_freq, json, latency_results);
    bench_submit_only_workloads(
        wq,
        iterations,
        SubmitOnlyMode::Unloaded,
        submit_bursts,
        tsc_freq,
        json,
        latency_results,
    );

    bench_single_op_latency(
        wq,
        "memmove",
        sizes,
        iterations,
        tsc_freq,
        cold,
        json,
        latency_results,
        |desc, src, dst, size| {
            desc.fill_memmove(src, dst, size);
        },
    );

    bench_single_op_latency(
        wq,
        "crc_gen",
        sizes,
        iterations,
        tsc_freq,
        cold,
        json,
        latency_results,
        |desc, src, _dst, size| {
            desc.fill_crc_gen(src, size, 0, 0);
        },
    );

    bench_single_op_latency(
        wq,
        "copy_crc",
        sizes,
        iterations,
        tsc_freq,
        cold,
        json,
        latency_results,
        |desc, src, dst, size| {
            desc.fill_copy_crc(src, dst, size, 0, 0);
        },
    );

    bench_batch_latency(wq, 4096, iterations, tsc_freq, json, latency_results);

    for &size in sizes {
        bench_pipelined_batch(
            wq,
            size,
            iterations,
            max_concurrency,
            json,
            throughput_results,
        );
    }

    for &size in sizes {
        bench_burst(
            wq,
            "memmove",
            size,
            iterations,
            max_concurrency,
            json,
            throughput_results,
            |desc, src, dst, sz| desc.fill_memmove(src, dst, sz),
        );
    }

    for &size in sizes {
        bench_burst_batch(
            wq,
            size,
            iterations,
            max_concurrency,
            json,
            throughput_results,
        );
    }

    for &size in sizes {
        bench_sliding_window(
            wq,
            "memmove",
            size,
            iterations,
            max_concurrency,
            json,
            throughput_results,
            fill_memmove_desc,
        );
    }

    for &size in sizes {
        bench_sliding_window(
            wq,
            "copy_crc",
            size,
            iterations,
            max_concurrency,
            json,
            throughput_results,
            fill_copy_crc_desc,
        );
    }

    if submit_threads > 1 {
        for &size in sizes {
            bench_multithread_memmove(
                wq,
                size,
                iterations,
                max_concurrency,
                submit_threads,
                json,
                throughput_results,
            );
        }
    }
}

// ============================================================================
// Submit-only burst benchmark
// ============================================================================

fn bench_submit_only_workloads(
    wq: &WqPortal,
    iterations: usize,
    submit_mode: SubmitOnlyMode,
    submit_bursts: &[usize],
    tsc_freq: u64,
    json: bool,
    results: &mut Vec<LatencyResult>,
) {
    bench_submit_only_all_timers(
        wq,
        iterations,
        SUBMIT_ONLY_EMPTY,
        submit_bursts,
        tsc_freq,
        json,
        results,
    );

    for workload in submit_workloads(submit_mode) {
        bench_submit_only_all_timers(
            wq,
            iterations,
            *workload,
            submit_bursts,
            tsc_freq,
            json,
            results,
        );
    }
}

fn bench_submit_only_all_timers(
    wq: &WqPortal,
    iterations: usize,
    workload: SubmitOnlyWorkload,
    submit_bursts: &[usize],
    tsc_freq: u64,
    json: bool,
    results: &mut Vec<LatencyResult>,
) {
    bench_submit_only_workload::<TscTimer>(
        wq,
        iterations,
        workload,
        submit_bursts,
        tsc_freq,
        json,
        results,
    );
    bench_submit_only_workload::<WallTimer>(
        wq,
        iterations,
        workload,
        submit_bursts,
        tsc_freq,
        json,
        results,
    );
    bench_submit_only_workload::<PmuTimer>(
        wq,
        iterations,
        workload,
        submit_bursts,
        tsc_freq,
        json,
        results,
    );
    bench_submit_only_workload::<RdpmcTimer>(
        wq,
        iterations,
        workload,
        submit_bursts,
        tsc_freq,
        json,
        results,
    );
}

#[derive(Copy, Clone)]
struct SubmitOnlyWorkload {
    name: &'static str,
    mode: SubmitOnlyMeasureMode,
}

#[derive(Copy, Clone)]
enum SubmitOnlyMeasureMode {
    Empty,
    Drained,
    Sustained,
    MfenceBetweenSubmits,
}

const SUBMIT_ONLY_EMPTY: SubmitOnlyWorkload = SubmitOnlyWorkload {
    name: "submit_only_empty",
    mode: SubmitOnlyMeasureMode::Empty,
};

const SUBMIT_ONLY_UNLOADED: SubmitOnlyWorkload = SubmitOnlyWorkload {
    name: "submit_only_unloaded",
    mode: SubmitOnlyMeasureMode::Drained,
};

const SUBMIT_ONLY_PRESSURE_RAMP: SubmitOnlyWorkload = SubmitOnlyWorkload {
    name: "submit_only_pressure_ramp",
    mode: SubmitOnlyMeasureMode::Sustained,
};

const SUBMIT_ONLY_MFENCE: SubmitOnlyWorkload = SubmitOnlyWorkload {
    name: "submit_only_mfence",
    mode: SubmitOnlyMeasureMode::MfenceBetweenSubmits,
};

const SUBMIT_ONLY_ALL: &[SubmitOnlyWorkload] = &[
    SUBMIT_ONLY_UNLOADED,
    SUBMIT_ONLY_MFENCE,
    SUBMIT_ONLY_PRESSURE_RAMP,
];
const SUBMIT_ONLY_UNLOADED_ONLY: &[SubmitOnlyWorkload] = &[SUBMIT_ONLY_UNLOADED];
const SUBMIT_ONLY_PRESSURE_RAMP_ONLY: &[SubmitOnlyWorkload] = &[SUBMIT_ONLY_PRESSURE_RAMP];
const SUBMIT_ONLY_MFENCE_ONLY: &[SubmitOnlyWorkload] = &[SUBMIT_ONLY_MFENCE];

fn submit_workloads(submit_mode: SubmitOnlyMode) -> &'static [SubmitOnlyWorkload] {
    match submit_mode {
        SubmitOnlyMode::All => SUBMIT_ONLY_ALL,
        SubmitOnlyMode::Unloaded => SUBMIT_ONLY_UNLOADED_ONLY,
        SubmitOnlyMode::Sustained => SUBMIT_ONLY_PRESSURE_RAMP_ONLY,
        SubmitOnlyMode::Mfence => SUBMIT_ONLY_MFENCE_ONLY,
    }
}

fn bench_submit_only_workload<T: SubmitTimer>(
    wq: &WqPortal,
    iterations: usize,
    workload: SubmitOnlyWorkload,
    submit_bursts: &[usize],
    tsc_freq: u64,
    json: bool,
    results: &mut Vec<LatencyResult>,
) {
    let mut desc = DsaHwDesc::default();
    let mut sentinel_desc = DsaHwDesc::default();
    let mut sentinel_comp = DsaCompletionRecord::default();

    desc.fill_noop(DsaFlags::empty());
    sentinel_desc.fill_noop(completion_flags_no_cache_control());
    sentinel_desc.set_completion(&mut sentinel_comp);

    let mut timers = MeasurementTimers::new();
    T::warn_if_unavailable(&mut timers, json);
    if !T::is_available(&timers) {
        return;
    }

    if !json {
        println!("\n=== {} ({}) ===", workload.name, T::NAME);
        println!(
            "{:>8} {:>14} {:>14}",
            "burst",
            T::BATCH_LABEL,
            T::SUBMIT_LABEL
        );
    }

    for &burst in submit_bursts {
        let mut measurements = Vec::with_capacity(iterations);

        for _ in 0..iterations {
            if workload.needs_sample_drain() {
                reset_completion(&mut sentinel_comp);
            }

            let value = T::measure(&mut timers, || workload.submit_burst(wq, &desc, burst));

            if workload.needs_sample_drain() {
                drain_submit_only_queue(
                    wq,
                    &sentinel_desc,
                    &mut sentinel_comp,
                    workload.name,
                    burst,
                );
            }

            measurements.push(value);
        }

        measurements.sort_unstable();
        let stats = compute_stats(&measurements);

        if !json {
            println!(
                "{:>8} {:>14} {:>14.1}",
                burst,
                stats.median,
                stats.median as f64 / burst as f64
            );
        }

        results.push(T::latency_result(
            workload.name,
            burst,
            stats,
            tsc_freq,
            &measurements,
        ));
    }

    if workload.needs_final_drain() {
        reset_completion(&mut sentinel_comp);
        drain_submit_only_queue(wq, &sentinel_desc, &mut sentinel_comp, workload.name, 0);
    }
}

fn drain_submit_only_queue(
    wq: &WqPortal,
    sentinel_desc: &DsaHwDesc,
    sentinel_comp: &mut DsaCompletionRecord,
    benchmark: &str,
    burst: usize,
) {
    unsafe { wq.submit(sentinel_desc) };
    let status = poll_completion(sentinel_comp);
    if status != DSA_COMP_SUCCESS {
        panic!(
            "DSA submit-only drain sentinel failed: status {:#x} (benchmark={}, burst={})",
            status, benchmark, burst
        );
    }
}

trait SubmitTimer {
    const NAME: &'static str;
    const BATCH_LABEL: &'static str;
    const SUBMIT_LABEL: &'static str;

    fn warn_if_unavailable(_timers: &mut MeasurementTimers, _json: bool) {}

    fn is_available(_timers: &MeasurementTimers) -> bool {
        true
    }

    fn measure(submit_timers: &mut MeasurementTimers, submit: impl FnOnce()) -> u64;

    fn latency_result(
        benchmark: &'static str,
        burst: usize,
        stats: crate::report::LatencyStats,
        tsc_freq: u64,
        measurements: &[u64],
    ) -> LatencyResult;
}

struct TscTimer;
struct WallTimer;
struct PmuTimer;
struct RdpmcTimer;

impl SubmitTimer for TscTimer {
    const NAME: &'static str = "tsc";
    const BATCH_LABEL: &'static str = "tsc/batch";
    const SUBMIT_LABEL: &'static str = "tsc/submit";

    fn measure(timers: &mut MeasurementTimers, submit: impl FnOnce()) -> u64 {
        timers.measure_tsc(submit)
    }

    fn latency_result(
        benchmark: &'static str,
        burst: usize,
        stats: crate::report::LatencyStats,
        tsc_freq: u64,
        measurements: &[u64],
    ) -> LatencyResult {
        let ns_vec: Vec<u64> = measurements
            .iter()
            .map(|&ticks| cycles_to_ns(ticks, tsc_freq))
            .collect();
        LatencyResult::with_tsc_ticks(benchmark, Self::NAME, burst, stats, compute_stats(&ns_vec))
    }
}

impl SubmitTimer for WallTimer {
    const NAME: &'static str = "wall";
    const BATCH_LABEL: &'static str = "ns/batch";
    const SUBMIT_LABEL: &'static str = "ns/submit";

    fn measure(timers: &mut MeasurementTimers, submit: impl FnOnce()) -> u64 {
        timers.measure_wall(submit)
    }

    fn latency_result(
        benchmark: &'static str,
        burst: usize,
        stats: crate::report::LatencyStats,
        _tsc_freq: u64,
        _measurements: &[u64],
    ) -> LatencyResult {
        LatencyResult::with_wall_ns(benchmark, Self::NAME, burst, stats)
    }
}

impl SubmitTimer for PmuTimer {
    const NAME: &'static str = "pmu";
    const BATCH_LABEL: &'static str = "core/batch";
    const SUBMIT_LABEL: &'static str = "core/submit";

    fn warn_if_unavailable(timers: &mut MeasurementTimers, json: bool) {
        timers.warn_if_pmu_unavailable(json);
    }

    fn is_available(timers: &MeasurementTimers) -> bool {
        timers.pmu_available()
    }

    fn measure(timers: &mut MeasurementTimers, submit: impl FnOnce()) -> u64 {
        timers
            .measure_pmu(submit)
            .unwrap_or_else(|| panic!("PMU core-cycle counter unavailable"))
    }

    fn latency_result(
        benchmark: &'static str,
        burst: usize,
        stats: crate::report::LatencyStats,
        _tsc_freq: u64,
        _measurements: &[u64],
    ) -> LatencyResult {
        LatencyResult::with_core_cycles(benchmark, Self::NAME, burst, stats)
    }
}

impl SubmitTimer for RdpmcTimer {
    const NAME: &'static str = "rdpmc";
    const BATCH_LABEL: &'static str = "rdpmc/batch";
    const SUBMIT_LABEL: &'static str = "rdpmc/submit";

    fn warn_if_unavailable(timers: &mut MeasurementTimers, json: bool) {
        timers.warn_if_rdpmc_unavailable(json);
    }

    fn is_available(timers: &MeasurementTimers) -> bool {
        timers.rdpmc_available()
    }

    fn measure(timers: &mut MeasurementTimers, submit: impl FnOnce()) -> u64 {
        timers
            .measure_rdpmc(submit)
            .unwrap_or_else(|| panic!("RDPMC core-cycle counter unavailable"))
    }

    fn latency_result(
        benchmark: &'static str,
        burst: usize,
        stats: crate::report::LatencyStats,
        _tsc_freq: u64,
        _measurements: &[u64],
    ) -> LatencyResult {
        LatencyResult::with_core_cycles(benchmark, Self::NAME, burst, stats)
    }
}

impl SubmitOnlyWorkload {
    fn needs_sample_drain(self) -> bool {
        matches!(
            self.mode,
            SubmitOnlyMeasureMode::Drained | SubmitOnlyMeasureMode::MfenceBetweenSubmits
        )
    }

    fn needs_final_drain(self) -> bool {
        matches!(self.mode, SubmitOnlyMeasureMode::Sustained)
    }

    fn submit_burst(self, wq: &WqPortal, desc: &DsaHwDesc, burst: usize) {
        match self.mode {
            SubmitOnlyMeasureMode::Empty => {}
            SubmitOnlyMeasureMode::Drained | SubmitOnlyMeasureMode::Sustained => {
                for _ in 0..burst {
                    unsafe { wq.submit(desc) };
                }
            }
            SubmitOnlyMeasureMode::MfenceBetweenSubmits => {
                for index in 0..burst {
                    unsafe { wq.submit(desc) };
                    if index + 1 != burst {
                        mfence();
                    }
                }
            }
        }
    }
}

// ============================================================================
// NOOP latency benchmark
// ============================================================================

fn bench_noop_latency(
    wq: &WqPortal,
    iterations: usize,
    tsc_freq: u64,
    json: bool,
    results: &mut Vec<LatencyResult>,
) {
    let mut desc = DsaHwDesc::default();
    let mut comp = DsaCompletionRecord::default();
    let mut latencies = Vec::with_capacity(iterations);

    // Warmup
    for _ in 0..100 {
        reset_completion(&mut comp);
        desc.fill_noop(completion_flags_no_cache_control());
        desc.set_completion(&mut comp);
        unsafe { wq.submit(&desc) };
        poll_completion(&comp);
    }

    // Measure
    for _ in 0..iterations {
        reset_completion(&mut comp);
        desc.fill_noop(completion_flags_no_cache_control());
        desc.set_completion(&mut comp);

        lfence();
        let start = rdtscp().0;
        unsafe { wq.submit(&desc) };
        poll_completion(&comp);
        let end = rdtscp().0;

        latencies.push(end - start);
    }

    latencies.sort_unstable();
    let cyc = compute_stats(&latencies);
    let ns_vec: Vec<u64> = latencies
        .iter()
        .map(|&c| cycles_to_ns(c, tsc_freq))
        .collect();
    // ns_vec is monotonic since latencies is sorted and cycles_to_ns is monotonic
    let ns = compute_stats(&ns_vec);

    if !json {
        println!("\n=== Single-op latency: noop ===");
        println!(
            "{:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>8}",
            "min_cyc", "med_cyc", "mean_cyc", "min_ns", "p99_ns", "p999_ns", "cv"
        );
        println!(
            "{:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>8.3}",
            cyc.min, cyc.median, cyc.mean, ns.min, ns.p99, ns.p999, cyc.cv
        );
    }

    results.push(LatencyResult::basic("noop", None, None, cyc, ns));
}

// ============================================================================
// Single-op latency benchmark (rdtscp)
// ============================================================================

fn bench_single_op_latency(
    wq: &WqPortal,
    op_name: &str,
    sizes: &[usize],
    iterations: usize,
    tsc_freq: u64,
    cold: bool,
    json: bool,
    results: &mut Vec<LatencyResult>,
    fill_fn: impl Fn(&mut DsaHwDesc, *const u8, *mut u8, u32),
) {
    if !json {
        println!("\n=== Single-op latency: {} ===", op_name);
        println!(
            "{:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>8}",
            "size", "min_cyc", "med_cyc", "min_ns", "med_ns", "mean_ns", "p99_ns", "cv"
        );
    }

    for &size in sizes {
        let src = vec![0xABu8; size];
        let mut dst = vec![0u8; size];

        let mut desc = DsaHwDesc::default();
        let mut comp = DsaCompletionRecord::default();

        let mut latencies = Vec::with_capacity(iterations);

        // Warmup
        for _ in 0..100 {
            reset_completion(&mut comp);
            fill_fn(&mut desc, src.as_ptr(), dst.as_mut_ptr(), size as u32);
            desc.set_completion(&mut comp);
            unsafe { wq.submit(&desc) };
            poll_completion(&comp);
        }

        // Measure
        for _ in 0..iterations {
            if cold {
                flush_range(src.as_ptr(), size);
                flush_range(dst.as_ptr(), size);
            }

            reset_completion(&mut comp);
            fill_fn(&mut desc, src.as_ptr(), dst.as_mut_ptr(), size as u32);
            desc.set_completion(&mut comp);

            lfence();
            let start = rdtscp().0;
            unsafe { wq.submit(&desc) };
            poll_completion(&comp);
            let end = rdtscp().0;

            latencies.push(end - start);
        }

        latencies.sort_unstable();
        let cyc = compute_stats(&latencies);
        let ns_vec: Vec<u64> = latencies
            .iter()
            .map(|&c| cycles_to_ns(c, tsc_freq))
            .collect();
        let ns = compute_stats(&ns_vec);

        if !json {
            println!(
                "{:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>8.3}",
                size, cyc.min, cyc.median, ns.min, ns.median, ns.mean, ns.p99, cyc.cv
            );
        }

        results.push(LatencyResult::basic(op_name, Some(size), None, cyc, ns));
    }
}

// ============================================================================
// Batch latency benchmark
// ============================================================================

fn bench_batch_latency(
    wq: &WqPortal,
    size: usize,
    iterations: usize,
    tsc_freq: u64,
    json: bool,
    results: &mut Vec<LatencyResult>,
) {
    if !json {
        println!("\n=== Batch latency: memmove (size={}) ===", size);
        println!(
            "{:>8} {:>10} {:>10} {:>10} {:>12}",
            "batch_n", "med_cyc", "med_ns", "mean_ns", "per_op_ns"
        );
    }

    for &batch_n in &[1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024] {
        let mut sub_descs: Vec<DsaHwDesc> = (0..batch_n).map(|_| DsaHwDesc::default()).collect();
        let mut sub_comps: Vec<DsaCompletionRecord> = (0..batch_n)
            .map(|_| DsaCompletionRecord::default())
            .collect();
        debug_assert!(
            sub_descs.as_ptr() as usize % 64 == 0,
            "descriptor list not 64-byte aligned"
        );

        let src = vec![0xABu8; size];
        let mut dst = vec![0u8; size];

        let mut batch_desc = DsaHwDesc::default();
        let mut batch_comp = DsaCompletionRecord::default();

        let mut latencies = Vec::with_capacity(iterations);

        // Warmup
        for _ in 0..50 {
            for i in 0..batch_n {
                reset_completion(&mut sub_comps[i]);
                sub_descs[i].fill_memmove(src.as_ptr(), dst.as_mut_ptr(), size as u32);
                sub_descs[i].set_completion(&mut sub_comps[i]);
            }
            reset_completion(&mut batch_comp);
            batch_desc.fill_batch(
                sub_descs.as_ptr(),
                batch_n as u32,
                completion_flags_no_cache_control(),
            );
            batch_desc.set_completion(&mut batch_comp);
            unsafe { wq.submit(&batch_desc) };
            poll_completion(&batch_comp);
        }

        // Measure
        for _ in 0..iterations {
            for i in 0..batch_n {
                reset_completion(&mut sub_comps[i]);
                sub_descs[i].fill_memmove(src.as_ptr(), dst.as_mut_ptr(), size as u32);
                sub_descs[i].set_completion(&mut sub_comps[i]);
            }
            reset_completion(&mut batch_comp);
            batch_desc.fill_batch(
                sub_descs.as_ptr(),
                batch_n as u32,
                completion_flags_no_cache_control(),
            );
            batch_desc.set_completion(&mut batch_comp);

            lfence();
            let start = rdtscp().0;
            unsafe { wq.submit(&batch_desc) };
            poll_completion(&batch_comp);
            let end = rdtscp().0;

            latencies.push(end - start);
        }

        latencies.sort_unstable();
        let cyc = compute_stats(&latencies);
        let ns_vec: Vec<u64> = latencies
            .iter()
            .map(|&c| cycles_to_ns(c, tsc_freq))
            .collect();
        let ns = compute_stats(&ns_vec);
        let per_op_ns = ns.median / batch_n as u64;

        if !json {
            println!(
                "{:>8} {:>10} {:>10} {:>10} {:>12}",
                batch_n, cyc.median, ns.median, ns.mean, per_op_ns
            );
        }

        results.push(LatencyResult::basic(
            "batch_memmove",
            Some(size),
            Some(batch_n),
            cyc,
            ns,
        ));
    }
}

// ============================================================================
// Pipelined batch throughput (sliding window of batch descriptors)
// ============================================================================

fn bench_pipelined_batch(
    wq: &WqPortal,
    size: usize,
    iterations: usize,
    max_concurrency: usize,
    json: bool,
    results: &mut Vec<ThroughputResult>,
) {
    if !json {
        println!(
            "\n=== Pipelined batch throughput: memmove (size={}) ===",
            size
        );
        println!(
            "{:>6} {:>8} {:>10} {:>14} {:>14}",
            "conc", "batch_n", "total_fly", "ops/sec", "bandwidth_MB/s"
        );
    }

    // Sweep batch sizes × concurrency levels
    for &batch_n in &[4, 8, 16, 32, 64, 128, 256] {
        for concurrency in [1, 2, 4, 8, 16, 32]
            .iter()
            .copied()
            .filter(|&c| c <= max_concurrency)
        {
            let total_inflight = concurrency * batch_n;

            // Per-slot: each slot owns a batch descriptor + sub-descriptors + sub-completions + buffers
            struct BatchSlot {
                batch_desc: DsaHwDesc,
                batch_comp: DsaCompletionRecord,
                sub_descs: Vec<DsaHwDesc>,
                sub_comps: Vec<DsaCompletionRecord>,
                src: Vec<u8>,
                dst: Vec<u8>,
            }

            let mut slots: Vec<BatchSlot> = (0..concurrency)
                .map(|_| {
                    let mut dst = vec![0u8; size];
                    // Touch pages
                    for offset in (0..size).step_by(4096) {
                        dst[offset] = 0xFF;
                    }
                    BatchSlot {
                        batch_desc: DsaHwDesc::default(),
                        batch_comp: DsaCompletionRecord::default(),
                        sub_descs: (0..batch_n).map(|_| DsaHwDesc::default()).collect(),
                        sub_comps: (0..batch_n)
                            .map(|_| DsaCompletionRecord::default())
                            .collect(),
                        src: vec![0xABu8; size],
                        dst,
                    }
                })
                .collect();

            // Helper to fill and submit a batch slot
            let fill_and_submit = |slot: &mut BatchSlot, wq: &WqPortal| {
                for i in 0..batch_n {
                    reset_completion(&mut slot.sub_comps[i]);
                    slot.sub_descs[i].fill_memmove(
                        slot.src.as_ptr(),
                        slot.dst.as_mut_ptr(),
                        size as u32,
                    );
                    slot.sub_descs[i].set_completion(&mut slot.sub_comps[i]);
                }
                reset_completion(&mut slot.batch_comp);
                slot.batch_desc.fill_batch(
                    slot.sub_descs.as_ptr(),
                    batch_n as u32,
                    completion_flags_no_cache_control(),
                );
                slot.batch_desc.set_completion(&mut slot.batch_comp);
                unsafe { wq.submit(&slot.batch_desc) };
            };

            // Warmup
            for s in slots.iter_mut() {
                fill_and_submit(s, wq);
            }
            for s in slots.iter() {
                poll_completion(&s.batch_comp);
            }

            let total_batches = iterations; // iterations = number of batch completions
            let window = concurrency.min(total_batches);

            // Submit the initial window inside the timed interval. Counting
            // completions for descriptors submitted before `start` inflates
            // short high-concurrency runs.
            let start = Instant::now();
            for s in slots.iter_mut().take(window) {
                fill_and_submit(s, wq);
            }

            let mut issued_batches = window;
            let mut completed_batches = 0usize;
            let mut idx = 0usize;

            while completed_batches < total_batches {
                let status = poll_completion(&slots[idx].batch_comp);
                if status == DSA_COMP_PAGE_FAULT_NOBOF {
                    touch_fault_page(&slots[idx].batch_comp);
                    fill_and_submit(&mut slots[idx], wq);
                    continue;
                }
                if status != DSA_COMP_SUCCESS && status != 0x05 {
                    // Drain all in-flight batch descriptors before panic
                    for s in &slots[..window] {
                        let st = s.batch_comp.status();
                        if st == DSA_COMP_NONE {
                            poll_completion(&s.batch_comp);
                        }
                    }
                    panic!(
                        "Pipelined batch failed: status {:#x} (size={}, conc={})",
                        status, size, concurrency
                    );
                }
                completed_batches += 1;

                if issued_batches < total_batches {
                    fill_and_submit(&mut slots[idx], wq);
                    issued_batches += 1;
                }
                idx = (idx + 1) % window;
            }

            // Drain remaining in-flight slots.
            for s in &slots[..window] {
                let status = s.batch_comp.status();
                if status == DSA_COMP_NONE {
                    poll_completion(&s.batch_comp);
                }
            }

            let elapsed = start.elapsed();
            let total_ops = total_batches * batch_n;
            let ops_per_sec = total_ops as f64 / elapsed.as_secs_f64();
            let bw_mb = (total_ops * size) as f64 / elapsed.as_secs_f64() / 1e6;

            if !json {
                println!(
                    "{:>6} {:>8} {:>10} {:>14.0} {:>14.1}",
                    concurrency, batch_n, total_inflight, ops_per_sec, bw_mb
                );
            }

            results.push(ThroughputResult {
                benchmark: format!("pipelined_batch_b{}", batch_n),
                size,
                concurrency,
                ops_per_sec,
                bandwidth_mb_s: bw_mb,
            });
        }
    }
}

// ============================================================================
// Burst throughput (submit N, wait all, repeat — no overlap)
// ============================================================================

fn bench_burst(
    wq: &WqPortal,
    op_name: &str,
    size: usize,
    iterations: usize,
    max_burst: usize,
    json: bool,
    results: &mut Vec<ThroughputResult>,
    fill_fn: impl Fn(&mut DsaHwDesc, *const u8, *mut u8, u32),
) {
    if !json {
        println!("\n=== Burst throughput: {} (size={}) ===", op_name, size);
        println!("{:>6} {:>14} {:>14}", "burst", "ops/sec", "bandwidth_MB/s");
    }

    for burst_size in [1, 2, 4, 8, 16, 32, 64, 128]
        .iter()
        .copied()
        .filter(|&b| b <= max_burst)
    {
        let mut descs: Vec<DsaHwDesc> = (0..burst_size).map(|_| DsaHwDesc::default()).collect();
        let mut comps: Vec<DsaCompletionRecord> = (0..burst_size)
            .map(|_| DsaCompletionRecord::default())
            .collect();

        // Per-op buffers
        let srcs: Vec<Vec<u8>> = (0..burst_size).map(|_| vec![0xABu8; size]).collect();
        let mut dsts: Vec<Vec<u8>> = (0..burst_size)
            .map(|_| {
                let mut v = vec![0u8; size];
                for offset in (0..size).step_by(4096) {
                    v[offset] = 0xFF;
                }
                v
            })
            .collect();

        let num_bursts = iterations;
        let start = Instant::now();

        for _ in 0..num_bursts {
            // Submit all
            for i in 0..burst_size {
                reset_completion(&mut comps[i]);
                fill_fn(
                    &mut descs[i],
                    srcs[i].as_ptr(),
                    dsts[i].as_mut_ptr(),
                    size as u32,
                );
                descs[i].set_completion(&mut comps[i]);
                unsafe { wq.submit(&descs[i]) };
            }
            // Wait all
            for i in 0..burst_size {
                let status = poll_completion(&comps[i]);
                if status == DSA_COMP_PAGE_FAULT_NOBOF {
                    touch_fault_page(&comps[i]);
                    // Drain remaining, then retry whole burst
                    drain_completions(&comps[i + 1..]);
                    // Resubmit this one
                    reset_completion(&mut comps[i]);
                    fill_fn(
                        &mut descs[i],
                        srcs[i].as_ptr(),
                        dsts[i].as_mut_ptr(),
                        size as u32,
                    );
                    descs[i].set_completion(&mut comps[i]);
                    unsafe { wq.submit(&descs[i]) };
                    // Re-poll from this slot
                    let retry_status = poll_completion(&comps[i]);
                    if retry_status != DSA_COMP_SUCCESS {
                        drain_completions(&comps);
                        panic!(
                            "DSA burst {} failed after page fault retry: status {:#x}",
                            op_name, retry_status
                        );
                    }
                    continue;
                }
                if status != DSA_COMP_SUCCESS {
                    drain_completions(&comps[i + 1..]);
                    panic!(
                        "DSA burst {} failed: status {:#x} (size={}, burst={})",
                        op_name, status, size, burst_size
                    );
                }
            }
        }

        let elapsed = start.elapsed();
        let total_ops = num_bursts * burst_size;
        let ops_per_sec = total_ops as f64 / elapsed.as_secs_f64();
        let bw_mb = (total_ops * size) as f64 / elapsed.as_secs_f64() / 1e6;

        if !json {
            println!("{:>6} {:>14.0} {:>14.1}", burst_size, ops_per_sec, bw_mb);
        }

        results.push(ThroughputResult {
            benchmark: format!("burst_{}", op_name),
            size,
            concurrency: burst_size,
            ops_per_sec,
            bandwidth_mb_s: bw_mb,
        });
    }
}

// ============================================================================
// Burst-batch throughput (submit B batch descriptors, wait all, repeat)
// ============================================================================

fn bench_burst_batch(
    wq: &WqPortal,
    size: usize,
    iterations: usize,
    max_burst: usize,
    json: bool,
    results: &mut Vec<ThroughputResult>,
) {
    if !json {
        println!("\n=== Burst-batch throughput: memmove (size={}) ===", size);
        println!(
            "{:>6} {:>8} {:>10} {:>14} {:>14}",
            "burst", "batch_n", "total_ops", "ops/sec", "bandwidth_MB/s"
        );
    }

    // Sweep batch_n × burst_size
    for &batch_n in &[4, 8, 16, 32, 64, 128, 256] {
        for burst_size in [1, 2, 4, 8, 16, 32]
            .iter()
            .copied()
            .filter(|&b| b <= max_burst)
        {
            struct BatchSlot {
                batch_desc: DsaHwDesc,
                batch_comp: DsaCompletionRecord,
                sub_descs: Vec<DsaHwDesc>,
                sub_comps: Vec<DsaCompletionRecord>,
                src: Vec<u8>,
                dst: Vec<u8>,
            }

            let mut slots: Vec<BatchSlot> = (0..burst_size)
                .map(|_| {
                    let mut dst = vec![0u8; size];
                    for offset in (0..size).step_by(4096) {
                        dst[offset] = 0xFF;
                    }
                    BatchSlot {
                        batch_desc: DsaHwDesc::default(),
                        batch_comp: DsaCompletionRecord::default(),
                        sub_descs: (0..batch_n).map(|_| DsaHwDesc::default()).collect(),
                        sub_comps: (0..batch_n)
                            .map(|_| DsaCompletionRecord::default())
                            .collect(),
                        src: vec![0xABu8; size],
                        dst,
                    }
                })
                .collect();

            let fill_and_submit = |slot: &mut BatchSlot, wq: &WqPortal| {
                for i in 0..batch_n {
                    reset_completion(&mut slot.sub_comps[i]);
                    slot.sub_descs[i].fill_memmove(
                        slot.src.as_ptr(),
                        slot.dst.as_mut_ptr(),
                        size as u32,
                    );
                    slot.sub_descs[i].set_completion(&mut slot.sub_comps[i]);
                }
                reset_completion(&mut slot.batch_comp);
                slot.batch_desc.fill_batch(
                    slot.sub_descs.as_ptr(),
                    batch_n as u32,
                    completion_flags_no_cache_control(),
                );
                slot.batch_desc.set_completion(&mut slot.batch_comp);
                unsafe { wq.submit(&slot.batch_desc) };
            };

            // Warmup
            for s in slots.iter_mut() {
                fill_and_submit(s, wq);
            }
            for s in &slots {
                poll_completion(&s.batch_comp);
            }

            let num_rounds = iterations;
            let start = Instant::now();

            for _ in 0..num_rounds {
                // Submit all batch descriptors
                for s in slots.iter_mut() {
                    fill_and_submit(s, wq);
                }
                // Wait all batch descriptors
                for s in &slots {
                    let status = poll_completion(&s.batch_comp);
                    if status == DSA_COMP_PAGE_FAULT_NOBOF {
                        touch_fault_page(&s.batch_comp);
                        // Remaining slots will be drained at next round or below
                        continue;
                    }
                    if status != DSA_COMP_SUCCESS && status != 0x05 {
                        // Drain all in-flight before panic
                        for s2 in &slots {
                            let st = s2.batch_comp.status();
                            if st == DSA_COMP_NONE {
                                poll_completion(&s2.batch_comp);
                            }
                        }
                        panic!(
                            "Burst-batch failed: status {:#x} (size={}, burst={})",
                            status, size, burst_size
                        );
                    }
                }
            }

            let elapsed = start.elapsed();
            let total_ops = num_rounds * burst_size * batch_n;
            let ops_per_sec = total_ops as f64 / elapsed.as_secs_f64();
            let bw_mb = (total_ops * size) as f64 / elapsed.as_secs_f64() / 1e6;

            if !json {
                println!(
                    "{:>6} {:>8} {:>10} {:>14.0} {:>14.1}",
                    burst_size,
                    batch_n,
                    burst_size * batch_n,
                    ops_per_sec,
                    bw_mb
                );
            }

            results.push(ThroughputResult {
                benchmark: format!("burst_batch_b{}", batch_n),
                size,
                concurrency: burst_size,
                ops_per_sec,
                bandwidth_mb_s: bw_mb,
            });
        }
    }
}

// ============================================================================
// Sliding window throughput benchmark (per-op buffers)
// ============================================================================

type DsaFillFn = fn(&mut DsaHwDesc, *const u8, *mut u8, u32);

fn fill_memmove_desc(desc: &mut DsaHwDesc, src: *const u8, dst: *mut u8, size: u32) {
    desc.fill_memmove(src, dst, size);
}

fn fill_copy_crc_desc(desc: &mut DsaHwDesc, src: *const u8, dst: *mut u8, size: u32) {
    desc.fill_copy_crc(src, dst, size, 0, 0);
}

fn bench_sliding_window(
    wq: &WqPortal,
    op_name: &str,
    size: usize,
    iterations: usize,
    max_concurrency: usize,
    json: bool,
    results: &mut Vec<ThroughputResult>,
    fill_fn: DsaFillFn,
) {
    if !json {
        println!(
            "\n=== Sliding window throughput: {} (size={}) ===",
            op_name, size
        );
        println!("{:>6} {:>14} {:>14}", "conc", "ops/sec", "bandwidth_MB/s");
    }

    for concurrency in [1, 2, 4, 8, 16, 32, 64, 128]
        .iter()
        .copied()
        .filter(|&c| c <= max_concurrency)
    {
        let window = concurrency.min(iterations);
        let mut state = DirectWindow::new(size, window);

        let start = Instant::now();
        let total_ops = state.run(wq, size, iterations, op_name, fill_fn);
        let elapsed = start.elapsed();

        let ops_per_sec = total_ops as f64 / elapsed.as_secs_f64();
        let bw_mb = (total_ops * size) as f64 / elapsed.as_secs_f64() / 1e6;

        if !json {
            println!("{:>6} {:>14.0} {:>14.1}", concurrency, ops_per_sec, bw_mb);
        }

        results.push(ThroughputResult {
            benchmark: op_name.into(),
            size,
            concurrency,
            ops_per_sec,
            bandwidth_mb_s: bw_mb,
        });
    }
}

// ============================================================================
// Multi-thread direct memmove throughput (batch_n = 1, concrete DSA path)
// ============================================================================

fn bench_multithread_memmove(
    wq: &WqPortal,
    size: usize,
    iterations: usize,
    max_concurrency: usize,
    submit_threads: usize,
    json: bool,
    results: &mut Vec<ThroughputResult>,
) {
    if !json {
        println!(
            "\n=== Multi-thread direct memmove throughput: size={} threads={} ===",
            size, submit_threads
        );
        println!(
            "{:>6} {:>8} {:>14} {:>14}",
            "conc", "threads", "ops/sec", "bandwidth_MB/s"
        );
    }

    for concurrency in [1, 2, 4, 8, 16, 32, 64, 128]
        .iter()
        .copied()
        .filter(|&c| c <= max_concurrency && c >= submit_threads)
    {
        let active_threads = submit_threads;
        let base_window = concurrency / active_threads;
        let extra_windows = concurrency % active_threads;
        let ready_barrier = Barrier::new(active_threads + 1);
        let start_barrier = Barrier::new(active_threads + 1);

        let (elapsed, total_ops) = thread::scope(|scope| {
            let mut handles = Vec::with_capacity(active_threads);

            for thread_idx in 0..active_threads {
                let thread_window = base_window + usize::from(thread_idx < extra_windows);
                let ready_barrier = &ready_barrier;
                let start_barrier = &start_barrier;

                handles.push(scope.spawn(move || {
                    let mut state = DirectWindow::new(size, thread_window);
                    ready_barrier.wait();
                    start_barrier.wait();
                    state.run(wq, size, iterations, "memmove_mt", fill_memmove_desc)
                }));
            }

            ready_barrier.wait();
            let start = Instant::now();
            start_barrier.wait();

            let mut total_ops = 0usize;
            for handle in handles {
                total_ops += handle
                    .join()
                    .expect("multi-thread DSA memmove worker panicked");
            }

            (start.elapsed(), total_ops)
        });

        let ops_per_sec = total_ops as f64 / elapsed.as_secs_f64();
        let bw_mb = (total_ops * size) as f64 / elapsed.as_secs_f64() / 1e6;

        if !json {
            println!(
                "{:>6} {:>8} {:>14.0} {:>14.1}",
                concurrency, active_threads, ops_per_sec, bw_mb
            );
        }

        results.push(ThroughputResult {
            benchmark: format!("memmove_mt_t{}", active_threads),
            size,
            concurrency,
            ops_per_sec,
            bandwidth_mb_s: bw_mb,
        });
    }
}

struct DirectWindow {
    descs: Vec<DsaHwDesc>,
    comps: Vec<DsaCompletionRecord>,
    srcs: Vec<Vec<u8>>,
    dsts: Vec<Vec<u8>>,
}

impl DirectWindow {
    fn new(size: usize, window: usize) -> Self {
        let descs = (0..window).map(|_| DsaHwDesc::default()).collect();
        let comps = (0..window)
            .map(|_| DsaCompletionRecord::default())
            .collect();
        let srcs = (0..window).map(|_| vec![0xABu8; size]).collect();
        let dsts = (0..window)
            .map(|_| {
                let mut v = vec![0u8; size];
                for offset in (0..size).step_by(4096) {
                    v[offset] = 0xFF;
                }
                v
            })
            .collect();

        Self {
            descs,
            comps,
            srcs,
            dsts,
        }
    }

    fn run(
        &mut self,
        wq: &WqPortal,
        size: usize,
        iterations: usize,
        op_name: &str,
        fill_fn: DsaFillFn,
    ) -> usize {
        let window = self.descs.len();

        for slot in 0..window {
            self.submit_slot(wq, size, fill_fn, slot);
        }

        let mut issued = window;
        let mut completed = 0usize;
        let mut slot = 0usize;

        while completed < iterations {
            let status = poll_completion(&self.comps[slot]);
            if status == DSA_COMP_PAGE_FAULT_NOBOF {
                touch_fault_page(&self.comps[slot]);
                self.submit_slot(wq, size, fill_fn, slot);
                continue;
            }
            if status != DSA_COMP_SUCCESS {
                drain_completions(&self.comps);
                panic!(
                    "DSA {} failed: status {:#x} (size={}, window={})",
                    op_name, status, size, window
                );
            }
            completed += 1;

            if issued < iterations {
                self.submit_slot(wq, size, fill_fn, slot);
                issued += 1;
            }

            slot = (slot + 1) % window;
        }

        drain_completions(&self.comps[..window]);
        completed
    }

    fn submit_slot(&mut self, wq: &WqPortal, size: usize, fill_fn: DsaFillFn, slot: usize) {
        reset_completion(&mut self.comps[slot]);
        fill_fn(
            &mut self.descs[slot],
            self.srcs[slot].as_ptr(),
            self.dsts[slot].as_mut_ptr(),
            size as u32,
        );
        self.descs[slot].set_completion(&mut self.comps[slot]);
        unsafe { wq.submit(&self.descs[slot]) };
    }
}
