//! Raw DSA hardware evaluation — measures true hardware performance
//! with zero framework overhead.
//!
//! Measures:
//! - Single-op latency: submit one descriptor, poll, measure
//! - Throughput: sliding window of N in-flight ops
//! - Batch: submit N descriptors as hardware batch
//! - Software baselines: memcpy, CRC-32C (SSE4.2)
//! - Crossover points: message size where DSA beats software

use clap::Parser;
use hw_eval::dsa::*;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "hw-eval", about = "Raw DSA/IAX hardware performance evaluation")]
struct Args {
    /// WQ device path (e.g., /dev/dsa/wq0.0)
    #[arg(short, long, default_value = "/dev/dsa/wq0.0")]
    device: PathBuf,

    /// Message sizes to test (bytes, comma-separated)
    #[arg(short, long, default_value = "64,256,1024,4096,16384,65536,262144,1048576")]
    sizes: String,

    /// Number of iterations per measurement
    #[arg(short, long, default_value = "10000")]
    iterations: usize,

    /// Maximum concurrency for sliding window test
    #[arg(short, long, default_value = "32")]
    max_concurrency: usize,

    /// Run software baselines only (no hardware required)
    #[arg(long)]
    sw_only: bool,
}

fn parse_sizes(s: &str) -> Vec<usize> {
    s.split(',')
        .map(|s| s.trim().parse().expect("invalid size"))
        .collect()
}

// ============================================================================
// Single-op latency benchmark
// ============================================================================

fn bench_single_op_latency(
    wq: &WqPortal,
    op_name: &str,
    sizes: &[usize],
    iterations: usize,
    fill_fn: impl Fn(&mut DsaHwDesc, *const u8, *mut u8, u32),
) {
    println!("\n=== Single-op latency: {} ===", op_name);
    println!("{:>10} {:>12} {:>12} {:>12}", "size", "min_ns", "median_ns", "mean_ns");

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
            reset_completion(&mut comp);
            fill_fn(&mut desc, src.as_ptr(), dst.as_mut_ptr(), size as u32);
            desc.set_completion(&mut comp);

            let start = Instant::now();
            unsafe { wq.submit(&desc) };
            poll_completion(&comp);
            let elapsed = start.elapsed().as_nanos() as u64;

            latencies.push(elapsed);
        }

        latencies.sort_unstable();
        let min = latencies[0];
        let median = latencies[latencies.len() / 2];
        let mean = latencies.iter().sum::<u64>() / latencies.len() as u64;

        println!("{:>10} {:>12} {:>12} {:>12}", size, min, median, mean);
    }
}

// ============================================================================
// Sliding window throughput benchmark
// ============================================================================

fn bench_sliding_window(
    wq: &WqPortal,
    op_name: &str,
    size: usize,
    iterations: usize,
    max_concurrency: usize,
    fill_fn: impl Fn(&mut DsaHwDesc, *const u8, *mut u8, u32),
) {
    println!("\n=== Sliding window throughput: {} (size={}) ===", op_name, size);
    println!("{:>6} {:>14} {:>14}", "conc", "ops/sec", "bandwidth_MB/s");

    let src = vec![0xABu8; size];
    let mut dst = vec![0u8; size];

    for concurrency in [1, 2, 4, 8, 16, 32].iter().copied().filter(|&c| c <= max_concurrency) {
        let mut descs: Vec<DsaHwDesc> = (0..concurrency).map(|_| DsaHwDesc::default()).collect();
        let mut comps: Vec<DsaCompletionRecord> =
            (0..concurrency).map(|_| DsaCompletionRecord::default()).collect();

        // Pre-fill and submit initial window
        for i in 0..concurrency {
            reset_completion(&mut comps[i]);
            fill_fn(&mut descs[i], src.as_ptr(), dst.as_mut_ptr(), size as u32);
            descs[i].set_completion(&mut comps[i]);
            unsafe { wq.submit(&descs[i]) };
        }

        let start = Instant::now();
        let mut completed = 0usize;
        let mut slot = 0usize;

        while completed < iterations {
            // Poll current slot
            let status = poll_completion(&comps[slot]);
            assert_eq!(
                status, DSA_COMP_SUCCESS,
                "DSA operation failed with status {:#x}",
                status
            );
            completed += 1;

            // Resubmit
            if completed + concurrency <= iterations + concurrency {
                reset_completion(&mut comps[slot]);
                fill_fn(&mut descs[slot], src.as_ptr(), dst.as_mut_ptr(), size as u32);
                descs[slot].set_completion(&mut comps[slot]);
                unsafe { wq.submit(&descs[slot]) };
            }

            slot = (slot + 1) % concurrency;
        }

        // Drain remaining
        for i in 0..concurrency {
            let status = unsafe { std::ptr::read_volatile(&comps[i].status) };
            if status == DSA_COMP_NONE {
                poll_completion(&comps[i]);
            }
        }

        let elapsed = start.elapsed();
        let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();
        let bw_mb = (iterations * size) as f64 / elapsed.as_secs_f64() / 1e6;

        println!("{:>6} {:>14.0} {:>14.1}", concurrency, ops_per_sec, bw_mb);
    }
}

// ============================================================================
// Software baselines
// ============================================================================

fn bench_software_baselines(sizes: &[usize], iterations: usize) {
    println!("\n=== Software baselines ===");

    // memcpy
    println!("\n--- memcpy (software) ---");
    println!("{:>10} {:>12} {:>14}", "size", "median_ns", "bandwidth_MB/s");
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
        let median = latencies[latencies.len() / 2];
        let bw = size as f64 / (median as f64) * 1000.0; // MB/s

        println!("{:>10} {:>12} {:>14.1}", size, median, bw);
    }

    // CRC-32C (SSE4.2)
    println!("\n--- CRC-32C (SSE4.2) ---");
    println!("{:>10} {:>12} {:>14}", "size", "median_ns", "bandwidth_MB/s");
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
        let median = latencies[latencies.len() / 2];
        let bw = size as f64 / (median as f64) * 1000.0;

        println!("{:>10} {:>12} {:>14.1}", size, median, bw);
    }
}

fn main() {
    let args = Args::parse();
    let sizes = parse_sizes(&args.sizes);

    println!("hw-eval: Raw DSA Hardware Performance Evaluation");
    println!("================================================");
    println!("Sizes: {:?}", sizes);
    println!("Iterations: {}", args.iterations);

    // Always run software baselines
    bench_software_baselines(&sizes, args.iterations);

    if args.sw_only {
        return;
    }

    // Open DSA WQ
    let wq = match WqPortal::open(&args.device) {
        Ok(wq) => {
            println!("\nOpened WQ: {}", args.device.display());
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

    // Single-op latency: memmove
    bench_single_op_latency(&wq, "memmove", &sizes, args.iterations, |desc, src, dst, size| {
        desc.fill_memmove(src, dst, size);
    });

    // Single-op latency: crc_gen
    bench_single_op_latency(&wq, "crc_gen", &sizes, args.iterations, |desc, src, _dst, size| {
        desc.fill_crc_gen(src, size, 0);
    });

    // Single-op latency: copy_crc
    bench_single_op_latency(&wq, "copy_crc", &sizes, args.iterations, |desc, src, dst, size| {
        desc.fill_copy_crc(src, dst, size, 0);
    });

    // Sliding window: memmove
    for &size in &[256, 4096, 65536] {
        if sizes.contains(&size) {
            bench_sliding_window(
                &wq,
                "memmove",
                size,
                args.iterations,
                args.max_concurrency,
                |desc, src, dst, sz| desc.fill_memmove(src, dst, sz),
            );
        }
    }

    // Sliding window: copy_crc
    for &size in &[256, 4096, 65536] {
        if sizes.contains(&size) {
            bench_sliding_window(
                &wq,
                "copy_crc",
                size,
                args.iterations,
                args.max_concurrency,
                |desc, src, dst, sz| desc.fill_copy_crc(src, dst, sz, 0),
            );
        }
    }

    println!("\nDone.");
}
