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
use hw_eval::submit::*;
use methodology::dsa::run_dsa_benchmarks;
use methodology::iax::run_iax_benchmarks;
use methodology::software::bench_software_baselines;
use report::{print_json_report, FullReport, LatencyResult, Metadata, ThroughputResult};
use snafu::{ResultExt, Snafu};

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

fn open_work_queue(config: &BenchmarkConfig) -> Result<WqPortal, HwEvalError> {
    WqPortal::open(&config.device).context(OpenWqSnafu {
        accelerator: config.accel.as_str(),
        device: config.device.display().to_string(),
        operation: "open_wq",
        hint: "need CAP_SYS_RAWIO or run via dsa_launcher",
    })
}

fn build_report(
    config: &BenchmarkConfig,
    tsc_freq: u64,
    core: usize,
    wq: Option<&WqPortal>,
    latency_results: Vec<LatencyResult>,
    throughput_results: Vec<ThroughputResult>,
) -> FullReport {
    FullReport {
        metadata: Metadata {
            accelerator: config.accel.as_str().to_string(),
            tsc_freq_hz: tsc_freq,
            pinned_core: core,
            cpu_numa_node: cpu_numa_node(core),
            device_numa_node: wq.and_then(|_| device_numa_node(&config.device)),
            device: config.device.display().to_string(),
            wq_dedicated: wq.map(WqPortal::is_dedicated),
            iterations: config.iterations,
            cold_cache: config.cold,
        },
        latency: latency_results,
        throughput: throughput_results,
    }
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
            let report = build_report(
                &config,
                tsc_freq,
                core,
                None,
                latency_results,
                throughput_results,
            );
            print_json_report(&report)?;
        }
        return Ok(());
    }

    // Open WQ
    let wq = open_work_queue(&config)?;

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
            run_iax_benchmarks(
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
    }

    if config.json {
        let report = build_report(
            &config,
            tsc_freq,
            core,
            Some(&wq),
            latency_results,
            throughput_results,
        );
        print_json_report(&report)?;
    } else {
        println!("\nDone.");
    }

    Ok(())
}
