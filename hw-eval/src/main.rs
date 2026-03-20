//! Raw DSA hardware evaluation — measures true hardware performance
//! with zero framework overhead.
//!
//! Measures:
//! - NOOP latency: pure submission + completion overhead
//! - Single-op latency: submit one descriptor, poll, measure (rdtscp)
//! - Batch latency: submit N descriptors as hardware batch
//! - Throughput: sliding window of N in-flight ops (per-op buffers)
//! - Software baselines: memcpy, CRC-32C (SSE4.2)

use clap::Parser;
use hw_eval::dsa::*;
use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
#[command(
    name = "hw-eval",
    about = "Raw DSA/IAX hardware performance evaluation"
)]
struct Args {
    /// WQ device path (e.g., /dev/dsa/wq0.0)
    #[arg(short, long, default_value = "/dev/dsa/wq0.0")]
    device: PathBuf,

    /// Message sizes to test (bytes, comma-separated)
    #[arg(
        short,
        long,
        default_value = "64,256,1024,4096,16384,65536,262144,1048576"
    )]
    sizes: String,

    /// Number of iterations per measurement
    #[arg(short, long, default_value = "10000")]
    iterations: usize,

    /// Maximum concurrency for sliding window test
    #[arg(short, long, default_value = "128")]
    max_concurrency: usize,

    /// Run software baselines only (no hardware required)
    #[arg(long)]
    sw_only: bool,

    /// Pin benchmark thread to this CPU core
    #[arg(long)]
    pin_core: Option<usize>,

    /// Flush caches between iterations (cold-cache measurement)
    #[arg(long)]
    cold: bool,

    /// Output results as JSON
    #[arg(long)]
    json: bool,
}

fn parse_sizes(s: &str) -> Vec<usize> {
    s.split(',')
        .map(|s| s.trim().parse().expect("invalid size"))
        .collect()
}

// ============================================================================
// Statistics
// ============================================================================

#[derive(Serialize, Clone)]
struct LatencyStats {
    min: u64,
    median: u64,
    mean: u64,
    p99: u64,
    p999: u64,
    cv: f64,
}

fn compute_stats(sorted: &[u64]) -> LatencyStats {
    let n = sorted.len();
    let min = sorted[0];
    let median = sorted[n / 2];
    let sum: u64 = sorted.iter().sum();
    let mean = sum / n as u64;
    let p99 = sorted[(n as f64 * 0.99) as usize];
    let p999 = sorted[((n as f64 * 0.999) as usize).min(n - 1)];

    let mean_f = sum as f64 / n as f64;
    let variance: f64 = sorted
        .iter()
        .map(|&v| {
            let d = v as f64 - mean_f;
            d * d
        })
        .sum::<f64>()
        / n as f64;
    let cv = if mean_f > 0.0 {
        variance.sqrt() / mean_f
    } else {
        0.0
    };

    LatencyStats {
        min,
        median,
        mean,
        p99,
        p999,
        cv,
    }
}

// ============================================================================
// JSON output structures
// ============================================================================

#[derive(Serialize)]
struct FullReport {
    metadata: Metadata,
    latency: Vec<LatencyResult>,
    throughput: Vec<ThroughputResult>,
}

#[derive(Serialize)]
struct Metadata {
    tsc_freq_hz: u64,
    pinned_core: usize,
    cpu_numa_node: Option<usize>,
    device_numa_node: Option<i32>,
    device: String,
    wq_dedicated: Option<bool>,
    iterations: usize,
    cold_cache: bool,
}

#[derive(Serialize)]
struct LatencyResult {
    benchmark: String,
    size: Option<usize>,
    batch_size: Option<usize>,
    cycles: LatencyStats,
    ns: LatencyStats,
}

#[derive(Serialize)]
struct ThroughputResult {
    benchmark: String,
    size: usize,
    concurrency: usize,
    ops_per_sec: f64,
    bandwidth_mb_s: f64,
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
                        let st = unsafe { std::ptr::read_volatile(&s.batch_comp.status) };
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
                let status = unsafe { std::ptr::read_volatile(&s.batch_comp.status) };
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
                            let st = unsafe { std::ptr::read_volatile(&s2.batch_comp.status) };
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
        for i in 0..concurrency {
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

            if completed + concurrency <= iterations + concurrency {
                reset_completion(&mut comps[slot]);
                fill_fn(
                    &mut descs[slot],
                    srcs[slot].as_ptr(),
                    dsts[slot].as_mut_ptr(),
                    size as u32,
                );
                descs[slot].set_completion(&mut comps[slot]);
                unsafe { wq.submit(&descs[slot]) };
            }

            slot = (slot + 1) % concurrency;
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
    let args = Args::parse();
    let sizes = parse_sizes(&args.sizes);

    // Thread pinning
    let core = args.pin_core.unwrap_or_else(|| current_core());
    match pin_to_core(core) {
        Ok(c) => {
            if !args.json {
                println!("Pinned to core {}", c)
            }
        }
        Err(e) => eprintln!("WARNING: failed to pin to core {}: {}", core, e),
    }

    // TSC frequency
    let tsc_freq = tsc_frequency_hz();

    if !args.json {
        println!("hw-eval: Raw DSA Hardware Performance Evaluation");
        println!("================================================");
        println!("TSC frequency: {:.3} GHz", tsc_freq as f64 / 1e9);
        println!("Sizes: {:?}", sizes);
        println!("Iterations: {}", args.iterations);
        if args.cold {
            println!("Mode: cold-cache (clflush between iterations)");
        }
        if let Some(node) = cpu_numa_node(core) {
            println!("CPU NUMA node: {}", node);
        }
    }

    let mut latency_results: Vec<LatencyResult> = Vec::new();
    let mut throughput_results: Vec<ThroughputResult> = Vec::new();

    // Software baselines
    bench_software_baselines(&sizes, args.iterations, args.json, &mut latency_results);

    if args.sw_only {
        if args.json {
            let report = FullReport {
                metadata: Metadata {
                    tsc_freq_hz: tsc_freq,
                    pinned_core: core,
                    cpu_numa_node: cpu_numa_node(core),
                    device_numa_node: None,
                    device: args.device.display().to_string(),
                    wq_dedicated: None,
                    iterations: args.iterations,
                    cold_cache: args.cold,
                },
                latency: latency_results,
                throughput: throughput_results,
            };
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
        }
        return;
    }

    // Open DSA WQ
    let wq = match WqPortal::open(&args.device) {
        Ok(wq) => {
            if !args.json {
                println!(
                    "\nOpened WQ: {} ({})",
                    args.device.display(),
                    if wq.is_dedicated() {
                        "dedicated"
                    } else {
                        "shared"
                    }
                );
                if let Some(node) = device_numa_node(&args.device) {
                    println!("DSA NUMA node: {}", node);
                }
            }
            wq
        }
        Err(e) => {
            eprintln!(
                "\nFailed to open {}: {} (need CAP_SYS_RAWIO or run via dsa_launcher)",
                args.device.display(),
                e
            );
            return;
        }
    };

    // NOOP — pure submission overhead
    bench_noop_latency(
        &wq,
        args.iterations,
        tsc_freq,
        args.json,
        &mut latency_results,
    );

    // Single-op latency: memmove
    bench_single_op_latency(
        &wq,
        "memmove",
        &sizes,
        args.iterations,
        tsc_freq,
        args.cold,
        args.json,
        &mut latency_results,
        |desc, src, dst, size| {
            desc.fill_memmove(src, dst, size);
        },
    );

    // Single-op latency: crc_gen
    bench_single_op_latency(
        &wq,
        "crc_gen",
        &sizes,
        args.iterations,
        tsc_freq,
        args.cold,
        args.json,
        &mut latency_results,
        |desc, src, _dst, size| {
            desc.fill_crc_gen(src, size, 0);
        },
    );

    // Single-op latency: copy_crc
    bench_single_op_latency(
        &wq,
        "copy_crc",
        &sizes,
        args.iterations,
        tsc_freq,
        args.cold,
        args.json,
        &mut latency_results,
        |desc, src, dst, size| {
            desc.fill_copy_crc(src, dst, size, 0);
        },
    );

    // Batch latency
    bench_batch_latency(
        &wq,
        4096,
        args.iterations,
        tsc_freq,
        args.json,
        &mut latency_results,
    );

    // Pipelined batch: memmove (representative sizes)
    for &size in sizes.iter() {
        bench_pipelined_batch(
            &wq,
            size,
            args.iterations,
            args.max_concurrency,
            args.json,
            &mut throughput_results,
        );
    }

    // Burst: memmove (all requested sizes)
    for &size in &sizes {
        bench_burst(
            &wq,
            "memmove",
            size,
            args.iterations,
            args.max_concurrency,
            args.json,
            &mut throughput_results,
            |desc, src, dst, sz| desc.fill_memmove(src, dst, sz),
        );
    }

    // Burst-batch: memmove (all requested sizes)
    for &size in &sizes {
        bench_burst_batch(
            &wq,
            size,
            args.iterations,
            args.max_concurrency,
            args.json,
            &mut throughput_results,
        );
    }

    // Sliding window: memmove (all requested sizes)
    for &size in &sizes {
        bench_sliding_window(
            &wq,
            "memmove",
            size,
            args.iterations,
            args.max_concurrency,
            args.json,
            &mut throughput_results,
            |desc, src, dst, sz| desc.fill_memmove(src, dst, sz),
        );
    }

    // Sliding window: copy_crc (all requested sizes)
    for &size in &sizes {
        bench_sliding_window(
            &wq,
            "copy_crc",
            size,
            args.iterations,
            args.max_concurrency,
            args.json,
            &mut throughput_results,
            |desc, src, dst, sz| desc.fill_copy_crc(src, dst, sz, 0),
        );
    }

    if args.json {
        let report = FullReport {
            metadata: Metadata {
                tsc_freq_hz: tsc_freq,
                pinned_core: core,
                cpu_numa_node: cpu_numa_node(core),
                device_numa_node: device_numa_node(&args.device),
                device: args.device.display().to_string(),
                wq_dedicated: Some(wq.is_dedicated()),
                iterations: args.iterations,
                cold_cache: args.cold,
            },
            latency: latency_results,
            throughput: throughput_results,
        };
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        println!("\nDone.");
    }
}
