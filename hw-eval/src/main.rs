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
mod timing;

use clap::Parser;
use config::{
    AccelKind, Args, BenchmarkConfig, BenchmarkConfigError, BenchmarkKind, SubmitOnlyMode,
};
use hw_eval::submit::*;
use methodology::dsa::run_dsa_benchmarks;
use methodology::iax::run_iax_benchmarks;
use methodology::software::bench_software_baselines;
use report::{
    print_json_report, AdmissionResult, CompletionReusePolicyResult, FullReport, LatencyResult,
    Metadata, SubmitMarkerOverlapResult, SubmitOccupancyResult, ThroughputResult,
    TrafficClassLadderResult,
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
    admission_results: Vec<AdmissionResult>,
    submit_occupancy_results: Vec<SubmitOccupancyResult>,
    submit_marker_overlap_results: Vec<SubmitMarkerOverlapResult>,
    traffic_class_ladder_results: Vec<TrafficClassLadderResult>,
    completion_reuse_policy_results: Vec<CompletionReusePolicyResult>,
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
        admission: admission_results,
        submit_occupancy: submit_occupancy_results,
        submit_marker_overlap: submit_marker_overlap_results,
        traffic_class_ladder: traffic_class_ladder_results,
        completion_reuse_policy: completion_reuse_policy_results,
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
            match config.benchmark {
                BenchmarkKind::SubmitOccupancy => {
                    println!("Submit occupancies: {:?}", config.submit_occupancies);
                    println!("DSA operation: {}", config.dsa_op.as_str());
                }
                BenchmarkKind::SubmitMarkerOverlap => {
                    println!("Marker bursts: {:?}", config.marker_bursts);
                    println!("Marker positions: {:?}", config.marker_positions);
                    println!("Marker poll cadences: {:?}", config.marker_poll_cadences);
                    println!("DSA operation: {}", config.dsa_op.as_str());
                }
                BenchmarkKind::TrafficClassLadder => {
                    println!("Traffic windows: {:?}", config.traffic_windows);
                    println!("Traffic classes: {:?}", config.traffic_classes);
                }
                BenchmarkKind::CompletionReusePolicy => {
                    println!(
                        "Completion reuse policies: {:?}",
                        config.completion_reuse_policies
                    );
                    println!(
                        "Completion reuse window: {}",
                        config.completion_reuse_window
                    );
                    println!("DSA operation: {}", config.dsa_op.as_str());
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
    let mut admission_results: Vec<AdmissionResult> = Vec::new();

    let mut submit_occupancy_results: Vec<SubmitOccupancyResult> = Vec::new();
    let mut submit_marker_overlap_results: Vec<SubmitMarkerOverlapResult> = Vec::new();
    let mut traffic_class_ladder_results: Vec<TrafficClassLadderResult> = Vec::new();
    let mut completion_reuse_policy_results: Vec<CompletionReusePolicyResult> = Vec::new();
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
                admission_results,
                submit_occupancy_results,
                submit_marker_overlap_results,
                traffic_class_ladder_results,
                completion_reuse_policy_results,
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
                config.benchmark,
                if config.benchmark == BenchmarkKind::SubmitOnly {
                    config.submit_mode
                } else {
                    SubmitOnlyMode::Unloaded
                },
                &config.submit_bursts,
                &config.submit_occupancies,
                &config.marker_bursts,
                &config.marker_positions,
                &config.marker_poll_cadences,
                &config.traffic_windows,
                &config.traffic_classes,
                &config.completion_reuse_policies,
                config.completion_reuse_window,
                config.max_concurrency,
                config.threads,
                config.dsa_op,
                tsc_freq,
                config.cold,
                config.json,
                &mut latency_results,
                &mut throughput_results,
                &mut admission_results,
                &mut submit_occupancy_results,
                &mut submit_marker_overlap_results,
                &mut traffic_class_ladder_results,
                &mut completion_reuse_policy_results,
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
            admission_results,
            submit_occupancy_results,
            submit_marker_overlap_results,
            traffic_class_ladder_results,
            completion_reuse_policy_results,
        );
        print_json_report(&report)?;
    } else {
        println!("\nDone.");
    }

    Ok(())
}
