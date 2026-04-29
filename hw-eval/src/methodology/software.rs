use std::time::Instant;

use hw_eval::sw::{sw_crc32c, sw_memcpy};

use crate::report::{compute_stats, LatencyResult};

pub(crate) fn bench_software_baselines(
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
