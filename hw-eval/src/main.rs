//! Raw DSA hardware evaluation — measures true hardware performance
//! with zero framework overhead.
//!
//! Measures:
//! - NOOP latency: pure submission + completion overhead
//! - Single-op latency: submit one descriptor, poll, measure (rdtscp)
//! - Batch latency: submit N descriptors as hardware batch
//! - Throughput: sliding window of N in-flight ops (per-op buffers)
//! - Software baselines: memcpy, CRC-32C (SSE4.2)

mod benchmarks;
mod config;
mod report;
mod timing;

use benchmarks::dsa::{run_dsa_benchmarks, DsaBenchmarkRequest, DsaBenchmarkResults};
use benchmarks::iax::run_iax_benchmarks;
use benchmarks::software::bench_software_baselines;
use clap::Parser;
use config::{
    AccelKind, Args, BenchmarkConfig, BenchmarkConfigError, BenchmarkKind, SubmitOnlyMode,
};
use hw_eval::submit::*;
use report::{
    print_json_report, FullReport, LatencyResult, Metadata, SubmissionBottleneckResults,
    ThroughputResult,
};
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
    submission_bottleneck_results: SubmissionBottleneckResults,
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
            threads: config.threads,
            cold_cache: config.cold,
        },
        latency: latency_results,
        throughput: throughput_results,
        admission: submission_bottleneck_results.admission,
        submit_occupancy: submission_bottleneck_results.submit_occupancy,
        submit_marker_overlap: submission_bottleneck_results.submit_marker_overlap,
        submit_marker_mechanism: submission_bottleneck_results.submit_marker_mechanism,
        traffic_class_ladder: submission_bottleneck_results.traffic_class_ladder,
        completion_reuse_policy: submission_bottleneck_results.completion_reuse_policy,
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

    // Thread pinning. Multi-submitter runs should not inherit one-core
    // affinity unless the operator explicitly asks for it with --pin-core.
    let core = config.pin_core.unwrap_or_else(|| current_core());
    if config.pin_core.is_some() || config.threads == 1 {
        match pin_benchmark_thread(core) {
            Ok(c) => {
                if !config.json {
                    println!("Pinned to core {}", c)
                }
            }
            Err(warning) => eprintln!("{warning}"),
        }
    } else if !config.json {
        println!(
            "Not pinning benchmark thread; --threads={} needs scheduler-visible CPUs",
            config.threads
        );
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
        if matches!(
            config.benchmark,
            BenchmarkKind::SubmitOnly | BenchmarkKind::SubmitAdmission
        ) {
            println!("Submit bursts: {:?}", config.submit_bursts);
            if config.benchmark == BenchmarkKind::SubmitOnly {
                println!("Submit mode: {:?}", config.submit_mode);
            }
        } else if matches!(
            config.benchmark,
            BenchmarkKind::SubmitOccupancy
                | BenchmarkKind::SubmitMarkerOverlap
                | BenchmarkKind::TrafficClassLadder
                | BenchmarkKind::CompletionReusePolicy
        ) {
            let bottleneck = &config.submission_bottleneck;
            match config.benchmark {
                BenchmarkKind::SubmitOccupancy => {
                    println!("Submit occupancies: {:?}", bottleneck.submit_occupancies);
                    println!("DSA operation: {}", bottleneck.dsa_operation.as_str());
                }
                BenchmarkKind::SubmitMarkerOverlap => {
                    println!("Marker bursts: {:?}", bottleneck.marker_bursts);
                    println!("Marker positions: {:?}", bottleneck.marker_positions);
                    println!(
                        "Marker poll cadences: {:?}",
                        bottleneck.marker_poll_cadences
                    );
                    println!("DSA operation: {}", bottleneck.dsa_operation.as_str());
                }
                BenchmarkKind::TrafficClassLadder => {
                    println!("Traffic windows: {:?}", bottleneck.traffic_windows);
                    println!("Traffic classes: {:?}", bottleneck.traffic_classes);
                }
                BenchmarkKind::CompletionReusePolicy => {
                    println!(
                        "Completion reuse policies: {:?}",
                        bottleneck.completion_reuse_policies
                    );
                    println!(
                        "Completion reuse window: {}",
                        bottleneck.completion_reuse_window
                    );
                    println!("DSA operation: {}", bottleneck.dsa_operation.as_str());
                }
                _ => {}
            }
        }
        println!("Submit threads: {}", config.threads);
        if config.cold {
            println!("Mode: cold-cache (clflush between iterations)");
        }
        if let Some(node) = cpu_numa_node(core) {
            println!("CPU NUMA node: {}", node);
        }
    }

    let mut latency_results: Vec<LatencyResult> = Vec::new();
    let mut throughput_results: Vec<ThroughputResult> = Vec::new();
    let mut submission_bottleneck_results = SubmissionBottleneckResults::default();
    // Software baselines
    if config.sw_only || config.benchmark == BenchmarkKind::All {
        bench_software_baselines(
            &config.sizes,
            config.iterations,
            config.json,
            &mut latency_results,
        );
    }

    if config.sw_only {
        if config.json {
            let report = build_report(
                &config,
                tsc_freq,
                core,
                None,
                latency_results,
                throughput_results,
                submission_bottleneck_results,
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
                DsaBenchmarkRequest {
                    wq: &wq,
                    sizes: &config.sizes,
                    iterations: config.iterations,
                    benchmark: config.benchmark,
                    submit_mode: if config.benchmark == BenchmarkKind::SubmitOnly {
                        config.submit_mode
                    } else {
                        SubmitOnlyMode::Unloaded
                    },
                    submit_bursts: &config.submit_bursts,
                    submission_bottleneck: &config.submission_bottleneck,
                    max_concurrency: config.max_concurrency,
                    submit_threads: config.threads,
                    tsc_freq,
                    cold: config.cold,
                    json: config.json,
                },
                DsaBenchmarkResults {
                    latency: &mut latency_results,
                    throughput: &mut throughput_results,
                    submission_bottleneck: &mut submission_bottleneck_results,
                },
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
            submission_bottleneck_results,
        );
        print_json_report(&report)?;
    } else {
        println!("\nDone.");
    }

    Ok(())
}
