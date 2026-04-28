use std::env;
use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use idxd_rust::{DEFAULT_DEVICE_PATH, MemmoveRequest};
use serde::Serialize;

const SCHEMA_VERSION: u32 = 1;
const MAX_BYTES: usize = 1 << 30;
const MAX_ITERATIONS: u64 = 1_000_000;
const MAX_CONCURRENCY: u32 = 4096;
const MAX_DURATION_MS: u64 = 60_000;

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

async fn execute(args: &CliArgs) -> BenchmarkArtifact {
    match args.backend {
        Backend::Software => software_success_artifact(args),
        Backend::Hardware => hardware_preflight_artifact(args),
    }
}

fn software_success_artifact(args: &CliArgs) -> BenchmarkArtifact {
    let mut results = Vec::with_capacity(args.suite.modes().len());
    for mode in args.suite.modes() {
        results.push(software_result(args, *mode));
    }

    BenchmarkArtifact {
        schema_version: SCHEMA_VERSION,
        ok: true,
        verdict: "pass",
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
        failure_class: None,
        error_kind: None,
        direct_failure_kind: None,
        validation_phase: None,
        validation_error_kind: None,
        results,
    }
}

fn software_result(args: &CliArgs, mode: BenchmarkMode) -> BenchmarkResult {
    let start = Instant::now();
    let completed_operations = match mode {
        BenchmarkMode::SingleLatency => args.iterations,
        BenchmarkMode::ConcurrentSubmissions => u64::from(args.concurrency) * args.iterations,
        BenchmarkMode::FixedDurationThroughput => u64::from(args.concurrency).max(1),
    };
    let simulated_bytes = completed_operations.saturating_mul(args.bytes as u64);
    let elapsed_ns = start.elapsed().as_nanos().max(1);
    let ops_per_sec = (completed_operations as f64) * 1_000_000_000.0 / (elapsed_ns as f64);
    let bytes_per_sec = (simulated_bytes as f64) * 1_000_000_000.0 / (elapsed_ns as f64);
    let mean_latency_ns = (elapsed_ns / u128::from(completed_operations.max(1))).max(1);

    BenchmarkResult {
        mode: mode.as_str(),
        target: "software_direct_async_diagnostic",
        comparison_target: None,
        requested_bytes: args.bytes,
        iterations: args.iterations,
        concurrency: args.concurrency,
        duration_ms: args.duration_ms,
        completed_operations,
        failed_operations: 0,
        elapsed_ns,
        min_latency_ns: Some(1),
        mean_latency_ns: Some(mean_latency_ns),
        max_latency_ns: Some(elapsed_ns),
        ops_per_sec: Some(ops_per_sec),
        bytes_per_sec: Some(bytes_per_sec),
        verdict: "pass",
        failure_class: None,
        error_kind: None,
        direct_failure_kind: None,
        validation_phase: None,
        validation_error_kind: None,
        claim_eligible: false,
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
