//! Raw DSA hardware evaluation — measures true hardware performance
//! with zero framework overhead.
//!
//! Measures:
//! - NOOP latency: pure submission + completion overhead
//! - Single-op latency: submit one descriptor, poll, measure (rdtscp)
//! - Batch latency: submit N descriptors as hardware batch
//! - Throughput: sliding window of N in-flight ops (per-op buffers)
//! - Software baselines: memcpy, CRC-32C (SSE4.2)

mod config;
mod report;

use clap::Parser;
use config::{AccelKind, Args, BenchmarkConfig, BenchmarkConfigError};
use hw_eval::dsa::*;
use hw_eval::iax;
use hw_eval::submit::*;
use hw_eval::sw::*;
use report::{
    compute_stats, print_json_report, FullReport, LatencyResult, Metadata, ThroughputResult,
};
use snafu::{ResultExt, Snafu};
use std::time::Instant;

#[derive(Debug, Snafu)]
pub(crate) enum HwEvalError {
    #[snafu(display("invalid hw-eval configuration: {source}"))]
    Config { source: BenchmarkConfigError },
    #[snafu(display(
        "failed to {operation} for accelerator {accelerator} at {device}: {source} ({hint})"
    ))]
    OpenWq {
        accelerator: &'static str,
        device: String,
        operation: &'static str,
        hint: &'static str,
        source: std::io::Error,
    },
    #[snafu(display("failed to serialize hw-eval JSON report: {source}"))]
    SerializeReport { source: serde_json::Error },
}

#[derive(Debug, Snafu)]
enum PinWarning {
    #[snafu(display(
        "warning: failed to pin benchmark thread to core {requested_core}: {source}"
    ))]
    Affinity {
        requested_core: usize,
        source: std::io::Error,
    },
}

fn pin_benchmark_thread(core: usize) -> Result<usize, PinWarning> {
    pin_to_core(core).map_err(|source| PinWarning::Affinity {
        requested_core: core,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use snafu::ResultExt;

    #[test]
    fn hw_eval_config_error_preserves_source_chain() {
        let error = BenchmarkConfig::builder()
            .sizes("64,abc".to_string())
            .build()
            .context(ConfigSnafu)
            .unwrap_err();

        let config_source = std::error::Error::source(&error)
            .expect("HwEvalError::Config should expose BenchmarkConfigError as source");
        assert!(
            std::error::Error::source(config_source).is_some(),
            "BenchmarkConfigError::InvalidSize should expose ParseIntError as source"
        );
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
        desc.fill_noop();
        desc.set_completion(&mut comp);
        unsafe { wq.submit(&desc) };
        poll_completion(&comp);
    }

    // Measure
    for _ in 0..iterations {
        reset_completion(&mut comp);
        desc.fill_noop();
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

    results.push(LatencyResult {
        benchmark: "noop".into(),
        size: None,
        batch_size: None,
        cycles: cyc,
        ns,
    });
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

        results.push(LatencyResult {
            benchmark: op_name.into(),
            size: Some(size),
            batch_size: None,
            cycles: cyc,
            ns,
        });
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
            batch_desc.fill_batch(sub_descs.as_ptr(), batch_n as u32);
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
            batch_desc.fill_batch(sub_descs.as_ptr(), batch_n as u32);
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

        results.push(LatencyResult {
            benchmark: "batch_memmove".into(),
            size: Some(size),
            batch_size: Some(batch_n),
            cycles: cyc,
            ns,
        });
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
                slot.batch_desc
                    .fill_batch(slot.sub_descs.as_ptr(), batch_n as u32);
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

            // Submit initial window
            for s in slots.iter_mut() {
                fill_and_submit(s, wq);
            }

            let total_batches = iterations; // iterations = number of batch completions
            let start = Instant::now();
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
                    for s in &slots {
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

                if completed_batches + concurrency <= total_batches {
                    fill_and_submit(&mut slots[idx], wq);
                }
                idx = (idx + 1) % concurrency;
            }

            // Drain remaining
            for s in &slots {
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
                slot.batch_desc
                    .fill_batch(slot.sub_descs.as_ptr(), batch_n as u32);
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

fn bench_sliding_window(
    wq: &WqPortal,
    op_name: &str,
    size: usize,
    iterations: usize,
    max_concurrency: usize,
    json: bool,
    results: &mut Vec<ThroughputResult>,
    fill_fn: impl Fn(&mut DsaHwDesc, *const u8, *mut u8, u32),
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
        let mut descs: Vec<DsaHwDesc> = (0..concurrency).map(|_| DsaHwDesc::default()).collect();
        let mut comps: Vec<DsaCompletionRecord> = (0..concurrency)
            .map(|_| DsaCompletionRecord::default())
            .collect();

        // Per-op buffers — each slot has its own src/dst.
        // Touch every page to avoid DSA page faults (DSA_COMP_PAGE_FAULT_NOBOF).
        let srcs: Vec<Vec<u8>> = (0..concurrency).map(|_| vec![0xABu8; size]).collect();
        let mut dsts: Vec<Vec<u8>> = (0..concurrency)
            .map(|_| {
                let mut v = vec![0u8; size];
                // Force page mapping by writing every page
                for offset in (0..size).step_by(4096) {
                    v[offset] = 0xFF;
                }
                v
            })
            .collect();

        // Pre-fill and submit initial window
        for i in 0..window {
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

        let start = Instant::now();
        let mut issued = window;
        let mut completed = 0usize;
        let mut slot = 0usize;

        while completed < iterations {
            let status = poll_completion(&comps[slot]);
            if status == DSA_COMP_PAGE_FAULT_NOBOF {
                touch_fault_page(&comps[slot]);
                reset_completion(&mut comps[slot]);
                fill_fn(
                    &mut descs[slot],
                    srcs[slot].as_ptr(),
                    dsts[slot].as_mut_ptr(),
                    size as u32,
                );
                descs[slot].set_completion(&mut comps[slot]);
                unsafe { wq.submit(&descs[slot]) };
                continue;
            }
            if status != DSA_COMP_SUCCESS {
                drain_completions(&comps);
                panic!(
                    "DSA {} failed: status {:#x} (size={}, conc={})",
                    op_name, status, size, concurrency
                );
            }
            completed += 1;

            if issued < iterations {
                reset_completion(&mut comps[slot]);
                fill_fn(
                    &mut descs[slot],
                    srcs[slot].as_ptr(),
                    dsts[slot].as_mut_ptr(),
                    size as u32,
                );
                descs[slot].set_completion(&mut comps[slot]);
                unsafe { wq.submit(&descs[slot]) };
                issued += 1;
            }

            slot = (slot + 1) % window;
        }

        // Drain remaining
        drain_completions(&comps);

        let elapsed = start.elapsed();
        let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();
        let bw_mb = (iterations * size) as f64 / elapsed.as_secs_f64() / 1e6;

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
// IAX benchmarks
// ============================================================================

fn bench_noop_latency_iax(
    wq: &WqPortal,
    iterations: usize,
    tsc_freq: u64,
    json: bool,
    results: &mut Vec<LatencyResult>,
) {
    let mut desc = iax::IaxHwDesc::default();
    let mut comp = iax::IaxCompletionRecord::default();
    let mut latencies = Vec::with_capacity(iterations);

    for _ in 0..100 {
        iax::reset_completion(&mut comp);
        desc.fill_noop();
        desc.set_completion(&mut comp);
        unsafe { wq.submit_iax(&desc) };
        iax::poll_completion(&comp);
    }

    for _ in 0..iterations {
        iax::reset_completion(&mut comp);
        desc.fill_noop();
        desc.set_completion(&mut comp);

        lfence();
        let start = rdtscp().0;
        unsafe { wq.submit_iax(&desc) };
        iax::poll_completion(&comp);
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
        println!("\n=== Single-op latency: noop (iax) ===");
        println!(
            "{:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>8}",
            "min_cyc", "med_cyc", "mean_cyc", "min_ns", "p99_ns", "p999_ns", "cv"
        );
        println!(
            "{:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>8.3}",
            cyc.min, cyc.median, cyc.mean, ns.min, ns.p99, ns.p999, cyc.cv
        );
    }

    results.push(LatencyResult {
        benchmark: "noop".into(),
        size: None,
        batch_size: None,
        cycles: cyc,
        ns,
    });
}

fn panic_iax_failure(
    op_name: &str,
    comp: &iax::IaxCompletionRecord,
    status: u8,
    size: usize,
    context: &str,
) -> ! {
    panic!(
        "IAX {} failed: status={:#x} error={:#x} invalid_flags={:#x} size={} {}",
        op_name,
        status,
        iax::completion_error_code(comp),
        iax::completion_invalid_flags(comp),
        size,
        context
    );
}

fn bench_single_op_latency_iax_crc64(
    wq: &WqPortal,
    sizes: &[usize],
    iterations: usize,
    tsc_freq: u64,
    cold: bool,
    json: bool,
    results: &mut Vec<LatencyResult>,
) {
    if !json {
        println!("\n=== Single-op latency: crc64 (iax) ===");
        println!(
            "{:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>8}",
            "size", "min_cyc", "med_cyc", "min_ns", "med_ns", "mean_ns", "p99_ns", "cv"
        );
    }

    for &size in sizes {
        let src = vec![0xABu8; size];
        let mut desc = iax::IaxHwDesc::default();
        let mut comp = iax::IaxCompletionRecord::default();
        let mut latencies = Vec::with_capacity(iterations);

        for _ in 0..100 {
            iax::reset_completion(&mut comp);
            desc.fill_crc64(src.as_ptr(), size as u32);
            desc.set_completion(&mut comp);
            unsafe { wq.submit_iax(&desc) };
            iax::poll_completion(&comp);
        }

        for _ in 0..iterations {
            if cold {
                flush_range(src.as_ptr(), size);
            }

            iax::reset_completion(&mut comp);
            desc.fill_crc64(src.as_ptr(), size as u32);
            desc.set_completion(&mut comp);

            lfence();
            let start = rdtscp().0;
            unsafe { wq.submit_iax(&desc) };
            let status = iax::poll_completion(&comp);
            if status == iax::IAX_COMP_PAGE_FAULT_IR {
                iax::touch_fault_page(&comp);
                iax::reset_completion(&mut comp);
                desc.fill_crc64(src.as_ptr(), size as u32);
                desc.set_completion(&mut comp);
                unsafe { wq.submit_iax(&desc) };
                let retry = iax::poll_completion(&comp);
                if retry != iax::IAX_COMP_SUCCESS {
                    panic_iax_failure("crc64", &comp, retry, size, "after page-fault retry");
                }
            } else if status != iax::IAX_COMP_SUCCESS {
                panic_iax_failure("crc64", &comp, status, size, "");
            }
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

        results.push(LatencyResult {
            benchmark: "crc64".into(),
            size: Some(size),
            batch_size: None,
            cycles: cyc,
            ns,
        });
    }
}

fn bench_burst_iax_crc64(
    wq: &WqPortal,
    size: usize,
    iterations: usize,
    max_burst: usize,
    json: bool,
    results: &mut Vec<ThroughputResult>,
) {
    if !json {
        println!("\n=== Burst throughput: crc64 (iax, size={}) ===", size);
        println!("{:>6} {:>14} {:>14}", "burst", "ops/sec", "bandwidth_MB/s");
    }

    for burst_size in [1, 2, 4, 8, 16, 32, 64, 128]
        .iter()
        .copied()
        .filter(|&b| b <= max_burst)
    {
        let mut descs: Vec<iax::IaxHwDesc> =
            (0..burst_size).map(|_| iax::IaxHwDesc::default()).collect();
        let mut comps: Vec<iax::IaxCompletionRecord> = (0..burst_size)
            .map(|_| iax::IaxCompletionRecord::default())
            .collect();
        let srcs: Vec<Vec<u8>> = (0..burst_size).map(|_| vec![0xABu8; size]).collect();

        let start = Instant::now();
        for _ in 0..iterations {
            for i in 0..burst_size {
                iax::reset_completion(&mut comps[i]);
                descs[i].fill_crc64(srcs[i].as_ptr(), size as u32);
                descs[i].set_completion(&mut comps[i]);
                unsafe { wq.submit_iax(&descs[i]) };
            }

            for i in 0..burst_size {
                let status = iax::poll_completion(&comps[i]);
                if status == iax::IAX_COMP_PAGE_FAULT_IR {
                    iax::touch_fault_page(&comps[i]);
                    iax::drain_completions(&comps[i + 1..]);
                    iax::reset_completion(&mut comps[i]);
                    descs[i].fill_crc64(srcs[i].as_ptr(), size as u32);
                    descs[i].set_completion(&mut comps[i]);
                    unsafe { wq.submit_iax(&descs[i]) };
                    let retry = iax::poll_completion(&comps[i]);
                    if retry != iax::IAX_COMP_SUCCESS {
                        iax::drain_completions(&comps);
                        panic!(
                            "IAX crc64 failed: status={:#x} error={:#x} invalid_flags={:#x} size={} burst={} after page-fault retry",
                            retry,
                            iax::completion_error_code(&comps[i]),
                            iax::completion_invalid_flags(&comps[i]),
                            size,
                            burst_size
                        );
                    }
                    continue;
                }
                if status != iax::IAX_COMP_SUCCESS {
                    iax::drain_completions(&comps[i + 1..]);
                    panic!(
                        "IAX crc64 failed: status={:#x} error={:#x} invalid_flags={:#x} size={} burst={}",
                        status,
                        iax::completion_error_code(&comps[i]),
                        iax::completion_invalid_flags(&comps[i]),
                        size,
                        burst_size
                    );
                }
            }
        }

        let elapsed = start.elapsed();
        let total_ops = iterations * burst_size;
        let ops_per_sec = total_ops as f64 / elapsed.as_secs_f64();
        let bw_mb = (total_ops * size) as f64 / elapsed.as_secs_f64() / 1e6;

        if !json {
            println!("{:>6} {:>14.0} {:>14.1}", burst_size, ops_per_sec, bw_mb);
        }

        results.push(ThroughputResult {
            benchmark: "burst_crc64".into(),
            size,
            concurrency: burst_size,
            ops_per_sec,
            bandwidth_mb_s: bw_mb,
        });
    }
}

fn bench_sliding_window_iax_crc64(
    wq: &WqPortal,
    size: usize,
    iterations: usize,
    max_concurrency: usize,
    json: bool,
    results: &mut Vec<ThroughputResult>,
) {
    if !json {
        println!(
            "\n=== Sliding window throughput: crc64 (iax, size={}) ===",
            size
        );
        println!("{:>6} {:>14} {:>14}", "conc", "ops/sec", "bandwidth_MB/s");
    }

    for concurrency in [1, 2, 4, 8, 16, 32, 64, 128]
        .iter()
        .copied()
        .filter(|&c| c <= max_concurrency)
    {
        let window = concurrency.min(iterations);
        let mut descs: Vec<iax::IaxHwDesc> = (0..concurrency)
            .map(|_| iax::IaxHwDesc::default())
            .collect();
        let mut comps: Vec<iax::IaxCompletionRecord> = (0..concurrency)
            .map(|_| iax::IaxCompletionRecord::default())
            .collect();
        let srcs: Vec<Vec<u8>> = (0..concurrency).map(|_| vec![0xABu8; size]).collect();

        for i in 0..window {
            iax::reset_completion(&mut comps[i]);
            descs[i].fill_crc64(srcs[i].as_ptr(), size as u32);
            descs[i].set_completion(&mut comps[i]);
            unsafe { wq.submit_iax(&descs[i]) };
        }

        let start = Instant::now();
        let mut issued = window;
        let mut completed = 0usize;
        let mut slot = 0usize;

        while completed < iterations {
            let status = iax::poll_completion(&comps[slot]);
            if status == iax::IAX_COMP_PAGE_FAULT_IR {
                iax::touch_fault_page(&comps[slot]);
                iax::reset_completion(&mut comps[slot]);
                descs[slot].fill_crc64(srcs[slot].as_ptr(), size as u32);
                descs[slot].set_completion(&mut comps[slot]);
                unsafe { wq.submit_iax(&descs[slot]) };
                continue;
            }
            if status != iax::IAX_COMP_SUCCESS {
                iax::drain_completions(&comps);
                panic!(
                    "IAX crc64 failed: status={:#x} error={:#x} invalid_flags={:#x} size={} conc={}",
                    status,
                    iax::completion_error_code(&comps[slot]),
                    iax::completion_invalid_flags(&comps[slot]),
                    size,
                    concurrency
                );
            }
            completed += 1;

            if issued < iterations {
                iax::reset_completion(&mut comps[slot]);
                descs[slot].fill_crc64(srcs[slot].as_ptr(), size as u32);
                descs[slot].set_completion(&mut comps[slot]);
                unsafe { wq.submit_iax(&descs[slot]) };
                issued += 1;
            }

            slot = (slot + 1) % window;
        }

        iax::drain_completions(&comps);

        let elapsed = start.elapsed();
        let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();
        let bw_mb = (iterations * size) as f64 / elapsed.as_secs_f64() / 1e6;

        if !json {
            println!("{:>6} {:>14.0} {:>14.1}", concurrency, ops_per_sec, bw_mb);
        }

        results.push(ThroughputResult {
            benchmark: "crc64".into(),
            size,
            concurrency,
            ops_per_sec,
            bandwidth_mb_s: bw_mb,
        });
    }
}

// ============================================================================
// Software baselines
// ============================================================================

fn bench_software_baselines(
    sizes: &[usize],
    iterations: usize,
    json: bool,
    results: &mut Vec<LatencyResult>,
) {
    if !json {
        println!("\n=== Software baselines ===");
    }

    // memcpy
    if !json {
        println!("\n--- memcpy (software) ---");
        println!(
            "{:>10} {:>10} {:>10} {:>14}",
            "size", "med_ns", "p99_ns", "bandwidth_MB/s"
        );
    }
    for &size in sizes {
        let src = vec![0xABu8; size];
        let mut dst = vec![0u8; size];
        let mut latencies = Vec::with_capacity(iterations);

        for _ in 0..iterations {
            let start = Instant::now();
            sw_memcpy(&mut dst, &src);
            std::hint::black_box(&dst);
            latencies.push(start.elapsed().as_nanos() as u64);
        }

        latencies.sort_unstable();
        let stats = compute_stats(&latencies);
        let bw = size as f64 / (stats.median as f64) * 1000.0;

        if !json {
            println!(
                "{:>10} {:>10} {:>10} {:>14.1}",
                size, stats.median, stats.p99, bw
            );
        }

        results.push(LatencyResult {
            benchmark: "sw_memcpy".into(),
            size: Some(size),
            batch_size: None,
            cycles: stats.clone(), // SW uses ns directly, cycles field holds ns
            ns: stats,
        });
    }

    // CRC-32C (SSE4.2)
    if !json {
        println!("\n--- CRC-32C (SSE4.2) ---");
        println!(
            "{:>10} {:>10} {:>10} {:>14}",
            "size", "med_ns", "p99_ns", "bandwidth_MB/s"
        );
    }
    for &size in sizes {
        let data = vec![0xABu8; size];
        let mut latencies = Vec::with_capacity(iterations);

        for _ in 0..iterations {
            let start = Instant::now();
            let crc = sw_crc32c(&data, 0);
            std::hint::black_box(crc);
            latencies.push(start.elapsed().as_nanos() as u64);
        }

        latencies.sort_unstable();
        let stats = compute_stats(&latencies);
        let bw = size as f64 / (stats.median as f64) * 1000.0;

        if !json {
            println!(
                "{:>10} {:>10} {:>10} {:>14.1}",
                size, stats.median, stats.p99, bw
            );
        }

        results.push(LatencyResult {
            benchmark: "sw_crc32c".into(),
            size: Some(size),
            batch_size: None,
            cycles: stats.clone(),
            ns: stats,
        });
    }
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), HwEvalError> {
    let args = Args::parse();
    let config = BenchmarkConfig::from_args(args).context(ConfigSnafu)?;

    // Thread pinning
    let core = config.pin_core.unwrap_or_else(|| current_core());
    match pin_benchmark_thread(core) {
        Ok(c) => {
            if !config.json {
                println!("Pinned to core {}", c)
            }
        }
        Err(warning) => eprintln!("{warning}"),
    }

    // TSC frequency
    let tsc_freq = tsc_frequency_hz();

    if !config.json {
        println!(
            "hw-eval: Raw {} Hardware Performance Evaluation",
            config.accel.as_str().to_uppercase()
        );
        println!("================================================");
        println!("TSC frequency: {:.3} GHz", tsc_freq as f64 / 1e9);
        println!("Accelerator: {}", config.accel.as_str());
        println!("Sizes: {:?}", config.sizes);
        println!("Iterations: {}", config.iterations);
        if config.cold {
            println!("Mode: cold-cache (clflush between iterations)");
        }
        if let Some(node) = cpu_numa_node(core) {
            println!("CPU NUMA node: {}", node);
        }
    }

    let mut latency_results: Vec<LatencyResult> = Vec::new();
    let mut throughput_results: Vec<ThroughputResult> = Vec::new();

    // Software baselines
    bench_software_baselines(
        &config.sizes,
        config.iterations,
        config.json,
        &mut latency_results,
    );

    if config.sw_only {
        if config.json {
            let report = FullReport {
                metadata: Metadata {
                    accelerator: config.accel.as_str().to_string(),
                    tsc_freq_hz: tsc_freq,
                    pinned_core: core,
                    cpu_numa_node: cpu_numa_node(core),
                    device_numa_node: None,
                    device: config.device.display().to_string(),
                    wq_dedicated: None,
                    iterations: config.iterations,
                    cold_cache: config.cold,
                },
                latency: latency_results,
                throughput: throughput_results,
            };
            print_json_report(&report)?;
        }
        return Ok(());
    }

    // Open WQ
    let wq = WqPortal::open(&config.device).context(OpenWqSnafu {
        accelerator: config.accel.as_str(),
        device: config.device.display().to_string(),
        operation: "open_wq",
        hint: "need CAP_SYS_RAWIO or run via dsa_launcher",
    })?;

    if !config.json {
        println!(
            "\nOpened WQ: {} ({})",
            config.device.display(),
            if wq.is_dedicated() {
                "dedicated"
            } else {
                "shared"
            }
        );
        if let Some(node) = device_numa_node(&config.device) {
            println!(
                "{} NUMA node: {}",
                config.accel.as_str().to_uppercase(),
                node
            );
        }
    }

    match config.accel {
        AccelKind::Dsa => {
            bench_noop_latency(
                &wq,
                config.iterations,
                tsc_freq,
                config.json,
                &mut latency_results,
            );

            bench_single_op_latency(
                &wq,
                "memmove",
                &config.sizes,
                config.iterations,
                tsc_freq,
                config.cold,
                config.json,
                &mut latency_results,
                |desc, src, dst, size| {
                    desc.fill_memmove(src, dst, size);
                },
            );

            bench_single_op_latency(
                &wq,
                "crc_gen",
                &config.sizes,
                config.iterations,
                tsc_freq,
                config.cold,
                config.json,
                &mut latency_results,
                |desc, src, _dst, size| {
                    desc.fill_crc_gen(src, size, 0);
                },
            );

            bench_single_op_latency(
                &wq,
                "copy_crc",
                &config.sizes,
                config.iterations,
                tsc_freq,
                config.cold,
                config.json,
                &mut latency_results,
                |desc, src, dst, size| {
                    desc.fill_copy_crc(src, dst, size, 0);
                },
            );

            bench_batch_latency(
                &wq,
                4096,
                config.iterations,
                tsc_freq,
                config.json,
                &mut latency_results,
            );

            for &size in config.sizes.iter() {
                bench_pipelined_batch(
                    &wq,
                    size,
                    config.iterations,
                    config.max_concurrency,
                    config.json,
                    &mut throughput_results,
                );
            }

            for &size in &config.sizes {
                bench_burst(
                    &wq,
                    "memmove",
                    size,
                    config.iterations,
                    config.max_concurrency,
                    config.json,
                    &mut throughput_results,
                    |desc, src, dst, sz| desc.fill_memmove(src, dst, sz),
                );
            }

            for &size in &config.sizes {
                bench_burst_batch(
                    &wq,
                    size,
                    config.iterations,
                    config.max_concurrency,
                    config.json,
                    &mut throughput_results,
                );
            }

            for &size in &config.sizes {
                bench_sliding_window(
                    &wq,
                    "memmove",
                    size,
                    config.iterations,
                    config.max_concurrency,
                    config.json,
                    &mut throughput_results,
                    |desc, src, dst, sz| desc.fill_memmove(src, dst, sz),
                );
            }

            for &size in &config.sizes {
                bench_sliding_window(
                    &wq,
                    "copy_crc",
                    size,
                    config.iterations,
                    config.max_concurrency,
                    config.json,
                    &mut throughput_results,
                    |desc, src, dst, sz| desc.fill_copy_crc(src, dst, sz, 0),
                );
            }
        }
        AccelKind::Iax => {
            bench_noop_latency_iax(
                &wq,
                config.iterations,
                tsc_freq,
                config.json,
                &mut latency_results,
            );

            bench_single_op_latency_iax_crc64(
                &wq,
                &config.sizes,
                config.iterations,
                tsc_freq,
                config.cold,
                config.json,
                &mut latency_results,
            );

            for &size in &config.sizes {
                bench_burst_iax_crc64(
                    &wq,
                    size,
                    config.iterations,
                    config.max_concurrency,
                    config.json,
                    &mut throughput_results,
                );
            }

            for &size in &config.sizes {
                bench_sliding_window_iax_crc64(
                    &wq,
                    size,
                    config.iterations,
                    config.max_concurrency,
                    config.json,
                    &mut throughput_results,
                );
            }
        }
    }

    if config.json {
        let report = FullReport {
            metadata: Metadata {
                accelerator: config.accel.as_str().to_string(),
                tsc_freq_hz: tsc_freq,
                pinned_core: core,
                cpu_numa_node: cpu_numa_node(core),
                device_numa_node: device_numa_node(&config.device),
                device: config.device.display().to_string(),
                wq_dedicated: Some(wq.is_dedicated()),
                iterations: config.iterations,
                cold_cache: config.cold,
            },
            latency: latency_results,
            throughput: throughput_results,
        };
        print_json_report(&report)?;
    } else {
        println!("\nDone.");
    }

    Ok(())
}
