use std::env;
use std::io::{self, Write};
use std::process::ExitCode;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

use bytes::{Bytes, BytesMut, buf::UninitSlice};
use idxd_rust::{
    AsyncDsaHandle, AsyncDsaSession, AsyncMemmoveError, AsyncMemmoveRequest,
    DEFAULT_MAX_PAGE_FAULT_RETRIES, DirectMemmoveBackend, DsaSession, MemmoveError, MemmovePhase,
    MemmoveValidationConfig,
};
use idxd_sys::{DSA_COMP_SUCCESS, DsaHwDesc, EnqcmdSubmission};
use tokio::task::JoinSet;

#[path = "tokio_memmove_bench/artifact.rs"]
mod artifact;
#[path = "tokio_memmove_bench/cli.rs"]
mod cli;

use artifact::{
    BenchmarkArtifact, BenchmarkResult, HARDWARE_ASYNC_TARGET, HARDWARE_SYNC_TARGET,
    SCHEMA_VERSION, SOFTWARE_TARGET, emit_artifact,
};
use cli::{Backend, BenchmarkMode, CliArgs, ParseOutcome, Suite, print_help};

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match CliArgs::parse(env::args().skip(1)) {
        Ok(ParseOutcome::Help) => {
            print_help();
            ExitCode::SUCCESS
        }
        Ok(ParseOutcome::Run(args)) => match run(args).await {
            Ok(exit) => exit,
            Err(err) => {
                let _ = writeln!(io::stderr(), "tokio_memmove_bench: {err}");
                ExitCode::from(2)
            }
        },
        Err(err) => {
            let _ = writeln!(io::stderr(), "tokio_memmove_bench: {err}");
            ExitCode::from(2)
        }
    }
}

async fn run(args: CliArgs) -> Result<ExitCode, String> {
    let artifact = execute(&args).await;
    emit_artifact(&args, &artifact)?;
    Ok(if artifact.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

#[derive(Debug, Clone)]
struct SoftwareDirectBackend {
    inner: Arc<SoftwareBackendInner>,
}

#[derive(Debug, Default)]
struct SoftwareBackendInner {
    submitted_op_ids: Mutex<Vec<u64>>,
    successful_copies: AtomicU64,
}

impl SoftwareDirectBackend {
    fn new() -> Self {
        Self {
            inner: Arc::new(SoftwareBackendInner::default()),
        }
    }
}

impl DirectMemmoveBackend for SoftwareDirectBackend {
    fn submit(&self, op_id: u64, descriptor: &DsaHwDesc) -> EnqcmdSubmission {
        self.inner
            .submitted_op_ids
            .lock()
            .expect("software backend submission registry poisoned")
            .push(op_id);

        let completion_addr = descriptor.completion_addr() as *mut u8;
        if !completion_addr.is_null() {
            // SAFETY: The direct runtime gave the backend a descriptor whose completion
            // address points at the operation-owned completion record. The diagnostic
            // backend only publishes the terminal success status byte; payload bytes are
            // copied later by initialize_success_destination, preserving the runtime's
            // success-copy boundary.
            unsafe {
                std::ptr::write_volatile(completion_addr, DSA_COMP_SUCCESS);
            }
        }

        EnqcmdSubmission::Accepted
    }

    fn initialize_success_destination(&self, _op_id: u64, dst: &mut UninitSlice, src: &[u8]) {
        self.inner.successful_copies.fetch_add(1, Ordering::SeqCst);
        dst.copy_from_slice(src);
    }
}

#[derive(Debug, Clone)]
struct RowFailure {
    failure_class: &'static str,
    error_kind: &'static str,
    direct_failure_kind: Option<&'static str>,
    validation_phase: Option<&'static str>,
    validation_error_kind: Option<&'static str>,
    direct_retry_budget: Option<u32>,
    direct_retry_count: Option<u32>,
    completion_status: Option<String>,
    completion_result: Option<u8>,
    completion_bytes_completed: Option<u32>,
    completion_fault_addr: Option<String>,
}

impl RowFailure {
    fn request(error_kind: &'static str) -> Self {
        Self {
            failure_class: "validation",
            error_kind,
            direct_failure_kind: None,
            validation_phase: Some("request_construction"),
            validation_error_kind: Some(error_kind),
            direct_retry_budget: None,
            direct_retry_count: None,
            completion_status: None,
            completion_result: None,
            completion_bytes_completed: None,
            completion_fault_addr: None,
        }
    }

    fn async_error(error: &AsyncMemmoveError) -> Self {
        let direct_failure_kind = error.direct_failure_kind().map(|kind| kind.as_str());
        let direct_failure = error.direct_failure();
        let completion_snapshot = direct_failure.and_then(|failure| failure.completion_snapshot());
        let failure_class = if direct_failure_kind.is_some() {
            "async_direct"
        } else if error.lifecycle_failure_kind().is_some() {
            "async_lifecycle"
        } else if error.worker_failure_kind().is_some() {
            "async_worker"
        } else if error
            .memmove_error()
            .is_some_and(|err| err.kind() == "queue_open")
        {
            "queue_open"
        } else {
            "memmove"
        };
        Self {
            failure_class,
            error_kind: error.kind(),
            direct_failure_kind,
            validation_phase: error
                .memmove_error()
                .and_then(|err| err.phase())
                .map(phase_name),
            validation_error_kind: error.memmove_error().map(|err| err.kind()),
            direct_retry_budget: direct_failure.map(|failure| failure.retry_budget()),
            direct_retry_count: direct_failure.map(|failure| failure.retry_count()),
            completion_status: completion_snapshot.map(|snapshot| hex_status(snapshot.status)),
            completion_result: completion_snapshot.map(|snapshot| snapshot.result),
            completion_bytes_completed: completion_snapshot
                .map(|snapshot| snapshot.bytes_completed),
            completion_fault_addr: completion_snapshot.map(|snapshot| hex_u64(snapshot.fault_addr)),
        }
    }

    fn sync_error(error: &MemmoveError, failure_class: &'static str) -> Self {
        Self {
            failure_class,
            error_kind: error.kind(),
            direct_failure_kind: None,
            validation_phase: error.phase().map(phase_name),
            validation_error_kind: Some(error.kind()),
            direct_retry_budget: None,
            direct_retry_count: error.page_fault_retries(),
            completion_status: error.final_status().map(hex_status),
            completion_result: None,
            completion_bytes_completed: None,
            completion_fault_addr: None,
        }
    }

    fn join_error() -> Self {
        Self {
            failure_class: "tokio_join",
            error_kind: "join_error",
            direct_failure_kind: None,
            validation_phase: None,
            validation_error_kind: None,
            direct_retry_budget: None,
            direct_retry_count: None,
            completion_status: None,
            completion_result: None,
            completion_bytes_completed: None,
            completion_fault_addr: None,
        }
    }
}

#[derive(Debug, Default)]
struct ModeStats {
    completed: u64,
    failed: u64,
    latencies_ns: Vec<u128>,
    first_failure: Option<RowFailure>,
}

impl ModeStats {
    fn record_success(&mut self, latency_ns: u128) {
        self.completed += 1;
        self.latencies_ns.push(latency_ns.max(1));
    }

    fn record_failure(&mut self, failure: RowFailure) {
        self.failed += 1;
        if self.first_failure.is_none() {
            self.first_failure = Some(failure);
        }
    }
}

async fn execute(args: &CliArgs) -> BenchmarkArtifact {
    match args.backend {
        Backend::Software => software_artifact(args).await,
        Backend::Hardware => hardware_artifact(args).await,
    }
}

async fn software_artifact(args: &CliArgs) -> BenchmarkArtifact {
    let config = match MemmoveValidationConfig::builder()
        .device_path(args.device_path.clone())
        .build()
    {
        Ok(config) => config,
        Err(error) => {
            return top_level_failure_artifact(
                args,
                Backend::Software,
                "validation",
                error.kind(),
                Some("preflight"),
                Some(error.kind()),
            );
        }
    };
    let backend = SoftwareDirectBackend::new();
    let session = match AsyncDsaSession::spawn_with_direct_backend(config, backend) {
        Ok(session) => session,
        Err(error) => {
            return top_level_failure_artifact(
                args,
                Backend::Software,
                "async_direct",
                error.kind(),
                None,
                None,
            );
        }
    };
    let handle = session.handle();

    let mut results = Vec::with_capacity(args.suite.modes().len());
    for mode in args.suite.modes() {
        results
            .push(run_async_mode(args, handle.clone(), *mode, SOFTWARE_TARGET, None, false).await);
    }

    drop(session);

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
        backend: Backend::Software.as_str(),
        claim_eligible: false,
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

fn top_level_failure_artifact(
    args: &CliArgs,
    backend: Backend,
    failure_class: &'static str,
    error_kind: &'static str,
    validation_phase: Option<&'static str>,
    validation_error_kind: Option<&'static str>,
) -> BenchmarkArtifact {
    BenchmarkArtifact {
        schema_version: SCHEMA_VERSION,
        ok: false,
        verdict: "fail",
        device_path: args.device_path.display().to_string(),
        backend: backend.as_str(),
        claim_eligible: false,
        suite: args.suite.as_str(),
        runtime_flavor: "current_thread",
        worker_threads: 1,
        requested_bytes: args.bytes,
        iterations: args.iterations,
        concurrency: args.concurrency,
        duration_ms: args.duration_ms,
        failure_class: Some(failure_class),
        error_kind: Some(error_kind),
        direct_failure_kind: None,
        validation_phase,
        validation_error_kind,
        direct_retry_budget: None,
        direct_retry_count: None,
        completion_status: None,
        completion_result: None,
        completion_bytes_completed: None,
        completion_fault_addr: None,
        results: Vec::new(),
    }
}

async fn run_async_mode(
    args: &CliArgs,
    handle: AsyncDsaHandle,
    mode: BenchmarkMode,
    target: &'static str,
    comparison_target: Option<&'static str>,
    claim_eligible: bool,
) -> BenchmarkResult {
    let start = Instant::now();
    let stats = match mode {
        BenchmarkMode::SingleLatency => single_latency(handle, args.bytes, args.iterations).await,
        BenchmarkMode::ConcurrentSubmissions => {
            concurrent_submissions(handle, args.bytes, args.iterations, args.concurrency).await
        }
        BenchmarkMode::FixedDurationThroughput => {
            fixed_duration_throughput(handle, args.bytes, args.concurrency, args.duration_ms).await
        }
    };
    let elapsed_ns = start.elapsed().as_nanos().max(1);
    stats.into_result(
        args,
        mode,
        target,
        comparison_target,
        claim_eligible,
        elapsed_ns,
    )
}

async fn single_latency(handle: AsyncDsaHandle, bytes: usize, iterations: u64) -> ModeStats {
    let mut stats = ModeStats::default();
    for seed in 0..iterations {
        match submit_one(handle.clone(), bytes, seed).await {
            Ok(latency_ns) => stats.record_success(latency_ns),
            Err(failure) => stats.record_failure(failure),
        }
    }
    stats
}

async fn concurrent_submissions(
    handle: AsyncDsaHandle,
    bytes: usize,
    iterations: u64,
    concurrency: u32,
) -> ModeStats {
    let mut stats = ModeStats::default();
    let mut seed = 0;
    for _ in 0..iterations {
        let mut tasks = JoinSet::new();
        for _ in 0..concurrency {
            tasks.spawn(submit_one(handle.clone(), bytes, seed));
            seed += 1;
        }
        drain_join_set(&mut tasks, &mut stats).await;
    }
    stats
}

async fn fixed_duration_throughput(
    handle: AsyncDsaHandle,
    bytes: usize,
    concurrency: u32,
    duration_ms: u64,
) -> ModeStats {
    let mut stats = ModeStats::default();
    let deadline = Instant::now() + Duration::from_millis(duration_ms);
    let mut tasks = JoinSet::new();
    let mut seed = 0;

    while Instant::now() < deadline {
        while tasks.len() < concurrency as usize && Instant::now() < deadline {
            tasks.spawn(submit_one(handle.clone(), bytes, seed));
            seed += 1;
        }

        if tasks.len() >= concurrency as usize {
            drain_one_join(&mut tasks, &mut stats).await;
        } else {
            tokio::task::yield_now().await;
        }
    }

    drain_join_set(&mut tasks, &mut stats).await;
    stats
}

async fn drain_join_set(tasks: &mut JoinSet<Result<u128, RowFailure>>, stats: &mut ModeStats) {
    while !tasks.is_empty() {
        drain_one_join(tasks, stats).await;
    }
}

async fn drain_one_join(tasks: &mut JoinSet<Result<u128, RowFailure>>, stats: &mut ModeStats) {
    match tasks.join_next().await {
        Some(Ok(Ok(latency_ns))) => stats.record_success(latency_ns),
        Some(Ok(Err(failure))) => stats.record_failure(failure),
        Some(Err(_join_error)) => stats.record_failure(RowFailure::join_error()),
        None => {}
    }
}

async fn submit_one(handle: AsyncDsaHandle, bytes: usize, seed: u64) -> Result<u128, RowFailure> {
    let request = build_request(bytes, seed)?;
    let start = Instant::now();
    handle
        .memmove(request)
        .await
        .map_err(|error| RowFailure::async_error(&error))?;
    Ok(start.elapsed().as_nanos().max(1))
}

fn build_request(bytes: usize, seed: u64) -> Result<AsyncMemmoveRequest, RowFailure> {
    let source = Bytes::from(deterministic_source(bytes, seed));
    let destination = BytesMut::with_capacity(bytes);
    AsyncMemmoveRequest::new(source, destination).map_err(|error| RowFailure::request(error.kind()))
}

fn deterministic_source(bytes: usize, seed: u64) -> Vec<u8> {
    (0..bytes)
        .map(|offset| seed.wrapping_add(offset as u64).to_le_bytes()[0])
        .collect()
}

impl ModeStats {
    fn into_result(
        self,
        args: &CliArgs,
        mode: BenchmarkMode,
        target: &'static str,
        comparison_target: Option<&'static str>,
        claim_eligible: bool,
        elapsed_ns: u128,
    ) -> BenchmarkResult {
        let simulated_bytes = self.completed.saturating_mul(args.bytes as u64);
        let ops_per_sec = rate_per_second(self.completed, elapsed_ns);
        let bytes_per_sec = rate_per_second(simulated_bytes, elapsed_ns);
        let min_latency_ns = self.latencies_ns.iter().copied().min();
        let max_latency_ns = self.latencies_ns.iter().copied().max();
        let mean_latency_ns = if self.latencies_ns.is_empty() {
            None
        } else {
            Some(self.latencies_ns.iter().copied().sum::<u128>() / self.latencies_ns.len() as u128)
        };
        let first_failure = self.first_failure.as_ref();

        BenchmarkResult {
            mode: mode.as_str(),
            target,
            comparison_target,
            requested_bytes: args.bytes,
            iterations: args.iterations,
            concurrency: args.concurrency,
            duration_ms: args.duration_ms,
            completed_operations: self.completed,
            failed_operations: self.failed,
            elapsed_ns,
            min_latency_ns,
            mean_latency_ns,
            max_latency_ns,
            ops_per_sec,
            bytes_per_sec,
            verdict: if self.failed == 0 && self.completed > 0 {
                "pass"
            } else {
                "fail"
            },
            failure_class: first_failure.map(|failure| failure.failure_class),
            error_kind: first_failure.map(|failure| failure.error_kind),
            direct_failure_kind: first_failure.and_then(|failure| failure.direct_failure_kind),
            validation_phase: first_failure.and_then(|failure| failure.validation_phase),
            validation_error_kind: first_failure.and_then(|failure| failure.validation_error_kind),
            direct_retry_budget: first_failure.and_then(|failure| failure.direct_retry_budget),
            direct_retry_count: first_failure.and_then(|failure| failure.direct_retry_count),
            completion_status: first_failure.and_then(|failure| failure.completion_status.clone()),
            completion_result: first_failure.and_then(|failure| failure.completion_result),
            completion_bytes_completed: first_failure
                .and_then(|failure| failure.completion_bytes_completed),
            completion_fault_addr: first_failure
                .and_then(|failure| failure.completion_fault_addr.clone()),
            claim_eligible: claim_eligible && self.failed == 0 && self.completed > 0,
        }
    }
}

fn rate_per_second(value: u64, elapsed_ns: u128) -> Option<f64> {
    if value == 0 {
        None
    } else {
        Some((value as f64) * 1_000_000_000.0 / (elapsed_ns.max(1) as f64))
    }
}

async fn hardware_artifact(args: &CliArgs) -> BenchmarkArtifact {
    let config = match MemmoveValidationConfig::builder()
        .device_path(args.device_path.clone())
        .max_page_fault_retries(DEFAULT_MAX_PAGE_FAULT_RETRIES)
        .build()
    {
        Ok(config) => config,
        Err(error) => {
            let failure = RowFailure::sync_error(&error, "validation");
            return failure_artifact_from_row(args, Backend::Hardware, &failure);
        }
    };
    let session = match AsyncDsaSession::open_config(config) {
        Ok(session) => session,
        Err(error) => {
            let failure = RowFailure::async_error(&error);
            return failure_artifact_from_row(args, Backend::Hardware, &failure);
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

fn failure_artifact_from_row(
    args: &CliArgs,
    backend: Backend,
    failure: &RowFailure,
) -> BenchmarkArtifact {
    BenchmarkArtifact {
        schema_version: SCHEMA_VERSION,
        ok: false,
        verdict: "expected_failure",
        device_path: args.device_path.display().to_string(),
        backend: backend.as_str(),
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

fn phase_name(phase: MemmovePhase) -> &'static str {
    match phase {
        MemmovePhase::QueueOpen => "queue_open",
        MemmovePhase::CompletionPoll => "completion_poll",
        MemmovePhase::PageFaultRetry => "page_fault_retry",
        MemmovePhase::PostCopyVerify => "post_copy_verify",
    }
}

fn hex_status(status: u8) -> String {
    format!("0x{status:02x}")
}

fn hex_u64(value: u64) -> String {
    format!("0x{value:x}")
}
