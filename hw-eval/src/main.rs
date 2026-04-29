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
mod methodology;
mod report;

use clap::Parser;
use config::{AccelKind, Args, BenchmarkConfig, BenchmarkConfigError};
use hw_eval::iax;
use hw_eval::submit::*;
use methodology::dsa::run_dsa_benchmarks;
use methodology::software::bench_software_baselines;
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
            run_dsa_benchmarks(
                &wq,
                &config.sizes,
                config.iterations,
                config.max_concurrency,
                tsc_freq,
                config.cold,
                config.json,
                &mut latency_results,
                &mut throughput_results,
            );
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
