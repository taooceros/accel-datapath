use std::time::Instant;

use idxd_rust::{
    AsyncDsaSession, DEFAULT_MAX_PAGE_FAULT_RETRIES, DsaSession, MemmoveValidationConfig,
};

use crate::artifact::{
    BenchmarkArtifact, BenchmarkResult, HARDWARE_ASYNC_TARGET, HARDWARE_SYNC_TARGET, SCHEMA_VERSION,
};
use crate::cli::{Backend, BenchmarkMode, CliArgs, Suite};
use crate::failure::RowFailure;
use crate::modes::{ModeStats, deterministic_source, run_async_mode};

pub(crate) async fn hardware_artifact(args: &CliArgs) -> BenchmarkArtifact {
    let config = match MemmoveValidationConfig::builder()
        .device_path(args.device_path.clone())
        .max_page_fault_retries(DEFAULT_MAX_PAGE_FAULT_RETRIES)
        .build()
    {
        Ok(config) => config,
        Err(error) => {
            let failure = RowFailure::sync_error(&error, "validation");
            return failure_artifact_from_row(args, &failure);
        }
    };
    let session = match AsyncDsaSession::open_config(config) {
        Ok(session) => session,
        Err(error) => {
            let failure = RowFailure::async_error(&error);
            return failure_artifact_from_row(args, &failure);
        }
    };
    let handle = session.handle();

    let mut results = Vec::with_capacity(args.suite.modes().len() + 1);
    for mode in args.suite.modes() {
        results.push(
            run_async_mode(
                args,
                handle.clone(),
                *mode,
                HARDWARE_ASYNC_TARGET,
                sync_comparison_target_for(*mode),
                true,
            )
            .await,
        );
    }

    drop(session);

    if matches!(args.suite, Suite::Canonical | Suite::Latency) {
        results.push(run_sync_comparison(args));
    }

    let first_failure = results.iter().find(|result| result.verdict != "pass");
    BenchmarkArtifact {
        schema_version: SCHEMA_VERSION,
        ok: first_failure.is_none(),
        verdict: if first_failure.is_none() {
            "pass"
        } else {
            "fail"
        },
        device_path: args.device_path.display().to_string(),
        backend: Backend::Hardware.as_str(),
        claim_eligible: first_failure.is_none(),
        suite: args.suite.as_str(),
        runtime_flavor: "current_thread",
        worker_threads: 1,
        requested_bytes: args.bytes,
        iterations: args.iterations,
        concurrency: args.concurrency,
        duration_ms: args.duration_ms,
        failure_class: first_failure.and_then(|result| result.failure_class),
        error_kind: first_failure.and_then(|result| result.error_kind),
        direct_failure_kind: first_failure.and_then(|result| result.direct_failure_kind),
        validation_phase: first_failure.and_then(|result| result.validation_phase),
        validation_error_kind: first_failure.and_then(|result| result.validation_error_kind),
        direct_retry_budget: first_failure.and_then(|result| result.direct_retry_budget),
        direct_retry_count: first_failure.and_then(|result| result.direct_retry_count),
        completion_status: first_failure.and_then(|result| result.completion_status.clone()),
        completion_result: first_failure.and_then(|result| result.completion_result),
        completion_bytes_completed: first_failure
            .and_then(|result| result.completion_bytes_completed),
        completion_fault_addr: first_failure
            .and_then(|result| result.completion_fault_addr.clone()),
        results,
    }
}

fn sync_comparison_target_for(mode: BenchmarkMode) -> Option<&'static str> {
    match mode {
        BenchmarkMode::SingleLatency => Some(HARDWARE_SYNC_TARGET),
        BenchmarkMode::ConcurrentSubmissions | BenchmarkMode::FixedDurationThroughput => None,
    }
}

fn failure_artifact_from_row(args: &CliArgs, failure: &RowFailure) -> BenchmarkArtifact {
    BenchmarkArtifact {
        schema_version: SCHEMA_VERSION,
        ok: false,
        verdict: "expected_failure",
        device_path: args.device_path.display().to_string(),
        backend: Backend::Hardware.as_str(),
        claim_eligible: false,
        suite: args.suite.as_str(),
        runtime_flavor: "current_thread",
        worker_threads: 1,
        requested_bytes: args.bytes,
        iterations: args.iterations,
        concurrency: args.concurrency,
        duration_ms: args.duration_ms,
        failure_class: Some(failure.failure_class),
        error_kind: Some(failure.error_kind),
        direct_failure_kind: failure.direct_failure_kind,
        validation_phase: failure.validation_phase,
        validation_error_kind: failure.validation_error_kind,
        direct_retry_budget: failure.direct_retry_budget,
        direct_retry_count: failure.direct_retry_count,
        completion_status: failure.completion_status.clone(),
        completion_result: failure.completion_result,
        completion_bytes_completed: failure.completion_bytes_completed,
        completion_fault_addr: failure.completion_fault_addr.clone(),
        results: Vec::new(),
    }
}

fn run_sync_comparison(args: &CliArgs) -> BenchmarkResult {
    let start = Instant::now();
    let mut stats = ModeStats::default();

    let config = match MemmoveValidationConfig::builder()
        .device_path(args.device_path.clone())
        .max_page_fault_retries(DEFAULT_MAX_PAGE_FAULT_RETRIES)
        .build()
    {
        Ok(config) => config,
        Err(error) => {
            stats.record_failure(RowFailure::sync_error(&error, "validation"));
            return stats.into_result(
                args,
                BenchmarkMode::SingleLatency,
                HARDWARE_SYNC_TARGET,
                Some(HARDWARE_ASYNC_TARGET),
                true,
                start.elapsed().as_nanos().max(1),
            );
        }
    };

    match DsaSession::open_config(config) {
        Ok(session) => {
            for seed in 0..args.iterations {
                let source = deterministic_source(args.bytes, seed);
                let mut destination = vec![0u8; args.bytes];
                let op_start = Instant::now();
                match session.memmove(&mut destination, &source) {
                    Ok(report) => record_sync_success(&mut stats, op_start, report),
                    Err(error) => {
                        stats.record_failure(RowFailure::sync_error(&error, "sync_memmove"))
                    }
                }
            }
        }
        Err(error) => stats.record_failure(RowFailure::sync_error(&error, "sync_queue_open")),
    }

    stats.into_result(
        args,
        BenchmarkMode::SingleLatency,
        HARDWARE_SYNC_TARGET,
        Some(HARDWARE_ASYNC_TARGET),
        true,
        start.elapsed().as_nanos().max(1),
    )
}

fn record_sync_success(
    stats: &mut ModeStats,
    op_start: Instant,
    _report: idxd_rust::MemmoveValidationReport,
) {
    stats.record_success(op_start.elapsed().as_nanos().max(1));
}
