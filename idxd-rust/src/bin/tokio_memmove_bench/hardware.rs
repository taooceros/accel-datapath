use idxd_rust::{AsyncDsaSession, AsyncMemmoveValidationMode, DsaConfig};

use crate::artifact::{BenchmarkArtifact, BenchmarkResult, SCHEMA_VERSION};
use crate::cli::{Backend, CliArgs};
use crate::failure::RowFailure;
use crate::modes::build_request;
use crate::nonbatch::run_nonbatch_submission_mode;

const POST_RUN_VALIDATION_MAX_SAMPLES: u32 = 8;

pub(crate) async fn hardware_artifact(args: &CliArgs) -> BenchmarkArtifact {
    let results = vec![run_nonbatch_submission_mode(args)];

    let first_row_failure = results.iter().find(|result| result.verdict != "pass");
    if first_row_failure.and_then(|result| result.failure_class) == Some("queue_open") {
        return expected_failure_artifact_from_result(args, first_row_failure.unwrap());
    }
    let post_run_validation = if first_row_failure.is_none() {
        run_post_run_validation(args).await
    } else {
        PostRunValidation::not_run()
    };

    artifact_from_results(args, results, post_run_validation)
}

fn build_config(
    args: &CliArgs,
    validation_mode: AsyncMemmoveValidationMode,
) -> Result<DsaConfig, idxd_rust::MemmoveError> {
    DsaConfig::builder()
        .device_path(args.device_path.clone())
        .max_page_fault_retries(args.max_page_fault_retries)
        .async_validation_mode(validation_mode)
        .build()
}

#[derive(Debug)]
struct PostRunValidation {
    status: &'static str,
    failure: Option<RowFailure>,
}

impl PostRunValidation {
    fn not_run() -> Self {
        Self {
            status: "not_run",
            failure: None,
        }
    }

    fn pass() -> Self {
        Self {
            status: "pass",
            failure: None,
        }
    }

    fn fail(failure: RowFailure) -> Self {
        Self {
            status: "fail",
            failure: Some(failure),
        }
    }
}

async fn run_post_run_validation(args: &CliArgs) -> PostRunValidation {
    let config = match build_config(args, AsyncMemmoveValidationMode::Full) {
        Ok(config) => config,
        Err(error) => return PostRunValidation::fail(RowFailure::sync_error(&error, "validation")),
    };
    let session = match AsyncDsaSession::open_config(config) {
        Ok(session) => session,
        Err(error) => return PostRunValidation::fail(RowFailure::async_error(&error)),
    };
    let handle = session.handle();
    let samples = args.concurrency.clamp(1, POST_RUN_VALIDATION_MAX_SAMPLES);

    for seed in 0..samples as u64 {
        let request = match build_request(args.bytes, seed) {
            Ok(request) => request,
            Err(failure) => return PostRunValidation::fail(failure),
        };
        if let Err(error) = handle.memmove(request).await {
            return PostRunValidation::fail(RowFailure::async_error(&error));
        }
    }

    drop(session);
    PostRunValidation::pass()
}

fn artifact_from_results(
    args: &CliArgs,
    results: Vec<BenchmarkResult>,
    post_run_validation: PostRunValidation,
) -> BenchmarkArtifact {
    let first_row_failure = results.iter().find(|result| result.verdict != "pass");
    let post_failure = post_run_validation.failure.as_ref();
    let ok = first_row_failure.is_none() && post_failure.is_none();
    BenchmarkArtifact {
        schema_version: SCHEMA_VERSION,
        ok,
        verdict: if ok { "pass" } else { "fail" },
        device_path: args.device_path.display().to_string(),
        backend: Backend::Hardware.as_str(),
        claim_eligible: ok,
        suite: args.suite.as_str(),
        runtime_flavor: "current_thread",
        worker_threads: 1,
        requested_bytes: args.bytes,
        iterations: args.iterations,
        concurrency: args.concurrency,
        duration_ms: args.duration_ms,
        max_page_fault_retries: args.max_page_fault_retries,
        validation_mode: args.validation_mode.as_str(),
        post_run_validation: post_run_validation.status,
        failure_class: first_row_failure
            .and_then(|result| result.failure_class)
            .or_else(|| post_failure.map(|failure| failure.failure_class)),
        error_kind: first_row_failure
            .and_then(|result| result.error_kind)
            .or_else(|| post_failure.map(|failure| failure.error_kind)),
        direct_failure_kind: first_row_failure
            .and_then(|result| result.direct_failure_kind)
            .or_else(|| post_failure.and_then(|failure| failure.direct_failure_kind)),
        validation_phase: first_row_failure
            .and_then(|result| result.validation_phase)
            .or_else(|| post_failure.and_then(|failure| failure.validation_phase)),
        validation_error_kind: first_row_failure
            .and_then(|result| result.validation_error_kind)
            .or_else(|| post_failure.and_then(|failure| failure.validation_error_kind)),
        direct_retry_budget: first_row_failure
            .and_then(|result| result.direct_retry_budget)
            .or_else(|| post_failure.and_then(|failure| failure.direct_retry_budget)),
        direct_retry_count: first_row_failure
            .and_then(|result| result.direct_retry_count)
            .or_else(|| post_failure.and_then(|failure| failure.direct_retry_count)),
        completion_status: first_row_failure
            .and_then(|result| result.completion_status.clone())
            .or_else(|| post_failure.and_then(|failure| failure.completion_status.clone())),
        completion_result: first_row_failure
            .and_then(|result| result.completion_result)
            .or_else(|| post_failure.and_then(|failure| failure.completion_result)),
        completion_bytes_completed: first_row_failure
            .and_then(|result| result.completion_bytes_completed)
            .or_else(|| post_failure.and_then(|failure| failure.completion_bytes_completed)),
        completion_fault_addr: first_row_failure
            .and_then(|result| result.completion_fault_addr.clone())
            .or_else(|| post_failure.and_then(|failure| failure.completion_fault_addr.clone())),
        results,
    }
}

fn expected_failure_artifact_from_result(
    args: &CliArgs,
    failure: &BenchmarkResult,
) -> BenchmarkArtifact {
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
        max_page_fault_retries: args.max_page_fault_retries,
        validation_mode: args.validation_mode.as_str(),
        post_run_validation: "not_run",
        failure_class: failure.failure_class,
        error_kind: failure.error_kind,
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
