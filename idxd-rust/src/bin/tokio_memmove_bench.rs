use std::env;
use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

use bytes::{Bytes, BytesMut, buf::UninitSlice};
use idxd_rust::{
    AsyncDsaHandle, AsyncDsaSession, AsyncMemmoveError, AsyncMemmoveRequest, DEFAULT_DEVICE_PATH,
    DirectMemmoveBackend, MemmoveRequest, MemmoveValidationConfig,
};
use idxd_sys::{DSA_COMP_SUCCESS, DsaHwDesc, EnqcmdSubmission};
use serde::Serialize;
use tokio::task::JoinSet;

const SCHEMA_VERSION: u32 = 1;
const MAX_BYTES: usize = 1 << 30;
const MAX_ITERATIONS: u64 = 1_000_000;
const MAX_CONCURRENCY: u32 = 4096;
const MAX_DURATION_MS: u64 = 60_000;
const SOFTWARE_TARGET: &str = "software_direct_async_diagnostic";

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    Hardware,
    Software,
}

impl Backend {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "hardware" => Ok(Self::Hardware),
            "software" => Ok(Self::Software),
            other => Err(format!(
                "unsupported backend `{other}`; expected `hardware` or `software`"
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Hardware => "hardware",
            Self::Software => "software",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Suite {
    Canonical,
    Latency,
    Concurrency,
    Throughput,
}

impl Suite {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "canonical" => Ok(Self::Canonical),
            "latency" => Ok(Self::Latency),
            "concurrency" => Ok(Self::Concurrency),
            "throughput" => Ok(Self::Throughput),
            other => Err(format!(
                "unsupported suite `{other}`; expected `canonical`, `latency`, `concurrency`, or `throughput`"
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Canonical => "canonical",
            Self::Latency => "latency",
            Self::Concurrency => "concurrency",
            Self::Throughput => "throughput",
        }
    }

    fn modes(self) -> &'static [BenchmarkMode] {
        match self {
            Self::Canonical => &[
                BenchmarkMode::SingleLatency,
                BenchmarkMode::ConcurrentSubmissions,
                BenchmarkMode::FixedDurationThroughput,
            ],
            Self::Latency => &[BenchmarkMode::SingleLatency],
            Self::Concurrency => &[BenchmarkMode::ConcurrentSubmissions],
            Self::Throughput => &[BenchmarkMode::FixedDurationThroughput],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Json,
    Text,
}

impl OutputFormat {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "json" => Ok(Self::Json),
            "text" => Ok(Self::Text),
            other => Err(format!(
                "unsupported output format `{other}`; expected `json` or `text`"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BenchmarkMode {
    SingleLatency,
    ConcurrentSubmissions,
    FixedDurationThroughput,
}

impl BenchmarkMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::SingleLatency => "single_latency",
            Self::ConcurrentSubmissions => "concurrent_submissions",
            Self::FixedDurationThroughput => "fixed_duration_throughput",
        }
    }
}

#[derive(Debug, Clone)]
struct CliArgs {
    device_path: PathBuf,
    backend: Backend,
    suite: Suite,
    bytes: usize,
    iterations: u64,
    concurrency: u32,
    duration_ms: u64,
    format: OutputFormat,
    artifact_path: Option<PathBuf>,
}

enum ParseOutcome {
    Help,
    Run(CliArgs),
}

impl CliArgs {
    fn parse<I>(mut args: I) -> Result<ParseOutcome, String>
    where
        I: Iterator<Item = String>,
    {
        let mut cli = Self {
            device_path: PathBuf::from(DEFAULT_DEVICE_PATH),
            backend: Backend::Hardware,
            suite: Suite::Canonical,
            bytes: 4096,
            iterations: 8,
            concurrency: 4,
            duration_ms: 100,
            format: OutputFormat::Text,
            artifact_path: None,
        };

        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--help" | "-h" => return Ok(ParseOutcome::Help),
                "--device" => {
                    let value = required_value(&mut args, "--device")?;
                    if value.is_empty() {
                        return Err("device path must not be empty".to_string());
                    }
                    cli.device_path = PathBuf::from(value);
                }
                "--backend" => {
                    cli.backend = Backend::parse(&required_value(&mut args, "--backend")?)?
                }
                "--suite" => cli.suite = Suite::parse(&required_value(&mut args, "--suite")?)?,
                "--bytes" => {
                    cli.bytes = parse_bounded_usize(
                        &required_value(&mut args, "--bytes")?,
                        "--bytes",
                        1,
                        MAX_BYTES,
                    )?;
                    MemmoveRequest::new(cli.bytes).map_err(|err| err.to_string())?;
                }
                "--iterations" => {
                    cli.iterations = parse_bounded_u64(
                        &required_value(&mut args, "--iterations")?,
                        "--iterations",
                        1,
                        MAX_ITERATIONS,
                    )?;
                }
                "--concurrency" => {
                    cli.concurrency = parse_bounded_u32(
                        &required_value(&mut args, "--concurrency")?,
                        "--concurrency",
                        1,
                        MAX_CONCURRENCY,
                    )?;
                }
                "--duration-ms" => {
                    cli.duration_ms = parse_bounded_u64(
                        &required_value(&mut args, "--duration-ms")?,
                        "--duration-ms",
                        1,
                        MAX_DURATION_MS,
                    )?;
                }
                "--format" => {
                    cli.format = OutputFormat::parse(&required_value(&mut args, "--format")?)?
                }
                "--artifact" => {
                    let path = PathBuf::from(required_value(&mut args, "--artifact")?);
                    validate_artifact_path(&path)?;
                    cli.artifact_path = Some(path);
                }
                other => {
                    return Err(format!(
                        "unsupported argument `{other}`; expected `--device`, `--backend`, `--suite`, `--bytes`, `--iterations`, `--concurrency`, `--duration-ms`, `--format`, `--artifact`, or `--help`"
                    ));
                }
            }
        }

        Ok(ParseOutcome::Run(cli))
    }
}

#[derive(Debug, Serialize)]
struct BenchmarkArtifact {
    schema_version: u32,
    ok: bool,
    verdict: &'static str,
    device_path: String,
    backend: &'static str,
    claim_eligible: bool,
    suite: &'static str,
    runtime_flavor: &'static str,
    worker_threads: u32,
    requested_bytes: usize,
    iterations: u64,
    concurrency: u32,
    duration_ms: u64,
    failure_class: Option<&'static str>,
    error_kind: Option<&'static str>,
    direct_failure_kind: Option<&'static str>,
    validation_phase: Option<&'static str>,
    validation_error_kind: Option<&'static str>,
    results: Vec<BenchmarkResult>,
}

#[derive(Debug, Serialize)]
struct BenchmarkResult {
    mode: &'static str,
    target: &'static str,
    comparison_target: Option<&'static str>,
    requested_bytes: usize,
    iterations: u64,
    concurrency: u32,
    duration_ms: u64,
    completed_operations: u64,
    failed_operations: u64,
    elapsed_ns: u128,
    min_latency_ns: Option<u128>,
    mean_latency_ns: Option<u128>,
    max_latency_ns: Option<u128>,
    ops_per_sec: Option<f64>,
    bytes_per_sec: Option<f64>,
    verdict: &'static str,
    failure_class: Option<&'static str>,
    error_kind: Option<&'static str>,
    direct_failure_kind: Option<&'static str>,
    validation_phase: Option<&'static str>,
    validation_error_kind: Option<&'static str>,
    claim_eligible: bool,
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

#[derive(Debug, Clone, Copy)]
struct RowFailure {
    failure_class: &'static str,
    error_kind: &'static str,
    direct_failure_kind: Option<&'static str>,
    validation_phase: Option<&'static str>,
    validation_error_kind: Option<&'static str>,
}

impl RowFailure {
    fn request(error_kind: &'static str) -> Self {
        Self {
            failure_class: "validation",
            error_kind,
            direct_failure_kind: None,
            validation_phase: Some("request_construction"),
            validation_error_kind: Some(error_kind),
        }
    }

    fn async_error(error: &AsyncMemmoveError) -> Self {
        let direct_failure_kind = error.direct_failure_kind().map(|kind| kind.as_str());
        let failure_class = if direct_failure_kind.is_some() {
            "async_direct"
        } else if error.lifecycle_failure_kind().is_some() {
            "async_lifecycle"
        } else if error.worker_failure_kind().is_some() {
            "async_worker"
        } else {
            "memmove"
        };
        Self {
            failure_class,
            error_kind: error.kind(),
            direct_failure_kind,
            validation_phase: None,
            validation_error_kind: None,
        }
    }

    fn join_error() -> Self {
        Self {
            failure_class: "tokio_join",
            error_kind: "join_error",
            direct_failure_kind: None,
            validation_phase: None,
            validation_error_kind: None,
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
        Backend::Hardware => hardware_preflight_artifact(args),
    }
}

async fn software_artifact(args: &CliArgs) -> BenchmarkArtifact {
    let config = match MemmoveValidationConfig::new(&args.device_path) {
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
        results.push(run_software_mode(args, handle.clone(), *mode).await);
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
        results: Vec::new(),
    }
}

async fn run_software_mode(
    args: &CliArgs,
    handle: AsyncDsaHandle,
    mode: BenchmarkMode,
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
    stats.into_result(args, mode, elapsed_ns)
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
    fn into_result(self, args: &CliArgs, mode: BenchmarkMode, elapsed_ns: u128) -> BenchmarkResult {
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
        let first_failure = self.first_failure;

        BenchmarkResult {
            mode: mode.as_str(),
            target: SOFTWARE_TARGET,
            comparison_target: None,
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
            claim_eligible: false,
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

fn hardware_preflight_artifact(args: &CliArgs) -> BenchmarkArtifact {
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
        failure_class: Some("preflight"),
        error_kind: Some("hardware_backend_not_wired"),
        direct_failure_kind: None,
        validation_phase: Some("preflight"),
        validation_error_kind: Some("hardware_backend_not_wired"),
        results: Vec::new(),
    }
}

fn emit_artifact(args: &CliArgs, artifact: &BenchmarkArtifact) -> Result<(), String> {
    let rendered = match args.format {
        OutputFormat::Json => serde_json::to_string(artifact)
            .map_err(|err| format!("failed to serialize benchmark artifact: {err}"))?,
        OutputFormat::Text => render_text(artifact),
    };

    if let Some(path) = &args.artifact_path {
        write_artifact(path, &rendered)?;
    }

    println!("{rendered}");
    Ok(())
}

fn render_text(artifact: &BenchmarkArtifact) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "verdict={} ok={} backend={} suite={} claim_eligible={}",
        artifact.verdict, artifact.ok, artifact.backend, artifact.suite, artifact.claim_eligible
    );
    for result in &artifact.results {
        let _ = writeln!(
            out,
            "mode={} target={} completed_operations={} failed_operations={} verdict={}",
            result.mode,
            result.target,
            result.completed_operations,
            result.failed_operations,
            result.verdict
        );
    }
    out.trim_end().to_string()
}

fn write_artifact(path: &Path, rendered: &str) -> Result<(), String> {
    validate_artifact_path(path)?;
    let temp_path = temporary_artifact_path(path)?;
    fs::write(&temp_path, rendered)
        .map_err(|err| format!("failed to write artifact `{}`: {err}", path.display()))?;
    fs::rename(&temp_path, path).map_err(|err| {
        let _ = fs::remove_file(&temp_path);
        format!("failed to commit artifact `{}`: {err}", path.display())
    })?;
    Ok(())
}

fn temporary_artifact_path(path: &Path) -> Result<PathBuf, String> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "artifact path must include a valid UTF-8 file name".to_string())?;
    Ok(path.with_file_name(format!(".{file_name}.tmp-{}", std::process::id())))
}

fn validate_artifact_path(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("artifact path must not be empty".to_string());
    }
    if path.exists() {
        let metadata = path.metadata().map_err(|err| {
            format!(
                "failed to inspect artifact path `{}`: {err}",
                path.display()
            )
        })?;
        if metadata.is_dir() {
            return Err(format!(
                "artifact path `{}` expected a writable file path, found directory",
                path.display()
            ));
        }
        if metadata.permissions().readonly() {
            return Err(format!(
                "artifact path `{}` expected a writable file path, found readonly file",
                path.display()
            ));
        }
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    if let Some(parent) = parent {
        let metadata = parent.metadata().map_err(|err| {
            format!(
                "artifact path `{}` expected an existing writable parent directory: {err}",
                path.display()
            )
        })?;
        if !metadata.is_dir() || metadata.permissions().readonly() {
            return Err(format!(
                "artifact path `{}` expected a writable parent directory",
                path.display()
            ));
        }
    }
    Ok(())
}

fn required_value<I>(args: &mut I, flag: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    args.next()
        .ok_or_else(|| format!("missing value for `{flag}`"))
}

fn parse_bounded_usize(raw: &str, flag: &str, min: usize, max: usize) -> Result<usize, String> {
    let value = raw.parse::<usize>().map_err(|_| {
        format!("invalid value `{raw}` for `{flag}`; expected an integer in {min}..={max}")
    })?;
    if !(min..=max).contains(&value) {
        return Err(format!(
            "invalid value `{raw}` for `{flag}`; expected an integer in {min}..={max}"
        ));
    }
    Ok(value)
}

fn parse_bounded_u64(raw: &str, flag: &str, min: u64, max: u64) -> Result<u64, String> {
    let value = raw.parse::<u64>().map_err(|_| {
        format!("invalid value `{raw}` for `{flag}`; expected an integer in {min}..={max}")
    })?;
    if !(min..=max).contains(&value) {
        return Err(format!(
            "invalid value `{raw}` for `{flag}`; expected an integer in {min}..={max}"
        ));
    }
    Ok(value)
}

fn parse_bounded_u32(raw: &str, flag: &str, min: u32, max: u32) -> Result<u32, String> {
    let value = raw.parse::<u32>().map_err(|_| {
        format!("invalid value `{raw}` for `{flag}`; expected an integer in {min}..={max}")
    })?;
    if !(min..=max).contains(&value) {
        return Err(format!(
            "invalid value `{raw}` for `{flag}`; expected an integer in {min}..={max}"
        ));
    }
    Ok(value)
}

fn print_help() {
    println!(
        "tokio_memmove_bench\n\nUSAGE:\n    tokio_memmove_bench [OPTIONS]\n\nOPTIONS:\n    --device <PATH>              DSA work queue path (default: {DEFAULT_DEVICE_PATH})\n    --backend <hardware|software>\n    --suite <canonical|latency|concurrency|throughput>\n    --bytes <N>                  Transfer size in bytes (1..={MAX_BYTES})\n    --iterations <N>             Iterations per latency/concurrency mode (1..={MAX_ITERATIONS})\n    --concurrency <N>            Concurrent submissions for concurrency/throughput modes (1..={MAX_CONCURRENCY})\n    --duration-ms <N>            Duration knob for throughput mode (1..={MAX_DURATION_MS})\n    --format <json|text>\n    --artifact <PATH>            Write exactly the emitted stdout artifact to this file\n    -h, --help                   Print help"
    );
}
