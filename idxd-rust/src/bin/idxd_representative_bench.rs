use std::env;
use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use idxd_rust::{
    Dsa, Iax, IaxCrc64Error, IaxCrc64Report, IdxdSession, IdxdSessionError, MemmoveError,
};
use idxd_sys::crc64_t10dif_field;
use serde::Serialize;

const SCHEMA_VERSION: u32 = 1;
const SUITE: &str = "idxd_representative_bench";
const DEFAULT_REQUESTED_BYTES: usize = 4096;
const DEFAULT_ITERATIONS: u64 = 1000;
const WARMUP_ITERATIONS: u64 = 1;
const MAX_REQUESTED_BYTES: usize = u32::MAX as usize;

fn main() -> ExitCode {
    match run() {
        Ok(exit) => exit,
        Err(err) => {
            let _ = writeln!(io::stderr(), "idxd_representative_bench: {err}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode, String> {
    match CliArgs::parse(env::args().skip(1))? {
        ParseOutcome::Help => {
            print_help();
            Ok(ExitCode::SUCCESS)
        }
        ParseOutcome::Run(args) => {
            let report = execute(&args);
            emit_report(&args, &report)?;
            Ok(if report.ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

impl OutputFormat {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            other => Err(format!(
                "unsupported output format `{other}`; expected `text` or `json`"
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParseOutcome {
    Help,
    Run(CliArgs),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliArgs {
    dsa_device: PathBuf,
    iax_device: PathBuf,
    dsa_shared_device: Option<PathBuf>,
    requested_bytes: usize,
    iterations: u64,
    format: OutputFormat,
    artifact_path: Option<PathBuf>,
}

impl CliArgs {
    fn parse<I>(mut args: I) -> Result<ParseOutcome, String>
    where
        I: Iterator<Item = String>,
    {
        let mut dsa_device = None;
        let mut iax_device = None;
        let mut dsa_shared_device = None;
        let mut requested_bytes = DEFAULT_REQUESTED_BYTES;
        let mut iterations = DEFAULT_ITERATIONS;
        let mut format = OutputFormat::Text;
        let mut artifact_path = None;

        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--dsa-device" => {
                    dsa_device = Some(parse_device_path(
                        "--dsa-device",
                        args.next()
                            .ok_or_else(|| "missing value for `--dsa-device`".to_string())?,
                    )?);
                }
                "--iax-device" => {
                    iax_device = Some(parse_device_path(
                        "--iax-device",
                        args.next()
                            .ok_or_else(|| "missing value for `--iax-device`".to_string())?,
                    )?);
                }
                "--dsa-shared-device" => {
                    dsa_shared_device = Some(parse_device_path(
                        "--dsa-shared-device",
                        args.next()
                            .ok_or_else(|| "missing value for `--dsa-shared-device`".to_string())?,
                    )?);
                }
                "--bytes" => {
                    requested_bytes = parse_requested_bytes(
                        args.next()
                            .ok_or_else(|| "missing value for `--bytes`".to_string())?
                            .as_str(),
                    )?;
                }
                "--iterations" => {
                    iterations = parse_iterations(
                        args.next()
                            .ok_or_else(|| "missing value for `--iterations`".to_string())?
                            .as_str(),
                    )?;
                }
                "--format" => {
                    format = OutputFormat::parse(
                        args.next()
                            .ok_or_else(|| "missing value for `--format`".to_string())?
                            .as_str(),
                    )?;
                }
                "--artifact" => {
                    let path = PathBuf::from(
                        args.next()
                            .ok_or_else(|| "missing value for `--artifact`".to_string())?,
                    );
                    validate_artifact_path(&path)?;
                    artifact_path = Some(path);
                }
                "--help" | "-h" => return Ok(ParseOutcome::Help),
                other => {
                    return Err(format!(
                        "unsupported argument `{other}`; expected `--dsa-device`, `--iax-device`, `--dsa-shared-device`, `--bytes`, `--iterations`, `--format`, or `--artifact`"
                    ));
                }
            }
        }

        let dsa_device =
            dsa_device.ok_or_else(|| "missing required `--dsa-device` path".to_string())?;
        let iax_device =
            iax_device.ok_or_else(|| "missing required `--iax-device` path".to_string())?;

        if let Some(shared) = &dsa_shared_device {
            if shared == &dsa_device || shared == &iax_device {
                return Err(
                    "contradictory target flags: `--dsa-shared-device` must differ from required device paths"
                        .to_string(),
                );
            }
        }

        Ok(ParseOutcome::Run(Self {
            dsa_device,
            iax_device,
            dsa_shared_device,
            requested_bytes,
            iterations,
            format,
            artifact_path,
        }))
    }
}

#[derive(Debug, Serialize)]
struct BenchmarkReport {
    schema_version: u32,
    ok: bool,
    verdict: &'static str,
    claim_eligible: bool,
    suite: &'static str,
    profile: &'static str,
    requested_bytes: usize,
    iterations: u64,
    warmup_iterations: u64,
    clock: &'static str,
    failure_phase: Option<String>,
    error_kind: Option<String>,
    failure_target: Option<String>,
    failure_accelerator: Option<String>,
    targets: Vec<TargetReport>,
}

#[derive(Debug, Serialize)]
struct TargetReport {
    target: &'static str,
    operation: &'static str,
    family: &'static str,
    device_path: String,
    work_queue_mode: Option<&'static str>,
    target_role: &'static str,
    requested_bytes: usize,
    iterations: u64,
    warmup_iterations: u64,
    ok: bool,
    verdict: &'static str,
    claim_eligible: bool,
    completed_operations: u64,
    failed_operations: u64,
    elapsed_ns: Option<u64>,
    min_latency_ns: Option<u64>,
    mean_latency_ns: Option<u64>,
    max_latency_ns: Option<u64>,
    ops_per_sec: Option<f64>,
    bytes_per_sec: Option<f64>,
    total_page_fault_retries: Option<u64>,
    last_page_fault_retries: Option<u32>,
    final_status: Option<String>,
    completion_error_code: Option<String>,
    invalid_flags: Option<String>,
    fault_addr: Option<String>,
    crc64: Option<String>,
    expected_crc64: Option<String>,
    crc64_verified: Option<bool>,
    failure_phase: Option<String>,
    error_kind: Option<String>,
    message: String,
}

#[derive(Debug, Clone, Copy)]
struct TargetSpec<'a> {
    target: &'static str,
    operation: &'static str,
    family: &'static str,
    target_role: &'static str,
    device_path: &'a Path,
}

#[derive(Debug, Clone, Copy)]
struct LatencyStats {
    completed_operations: u64,
    elapsed_ns: u64,
    min_latency_ns: u64,
    mean_latency_ns: u64,
    max_latency_ns: u64,
    ops_per_sec: f64,
    bytes_per_sec: f64,
}

impl LatencyStats {
    fn new(
        completed_operations: u64,
        total_latency_ns: u64,
        min_latency_ns: u64,
        max_latency_ns: u64,
        requested_bytes: usize,
    ) -> Self {
        let elapsed_ns = total_latency_ns.max(1);
        let seconds = elapsed_ns as f64 / 1_000_000_000.0;
        Self {
            completed_operations,
            elapsed_ns,
            min_latency_ns,
            mean_latency_ns: (elapsed_ns / completed_operations.max(1)).max(1),
            max_latency_ns,
            ops_per_sec: completed_operations as f64 / seconds,
            bytes_per_sec: (completed_operations as f64 * requested_bytes as f64) / seconds,
        }
    }
}

fn execute(args: &CliArgs) -> BenchmarkReport {
    let mut targets = vec![
        run_dsa_target(
            TargetSpec {
                target: "dsa-memmove",
                operation: "memmove",
                family: "dsa",
                target_role: "required",
                device_path: &args.dsa_device,
            },
            args,
        ),
        run_iax_target(
            TargetSpec {
                target: "iax-crc64",
                operation: "crc64",
                family: "iax",
                target_role: "required",
                device_path: &args.iax_device,
            },
            args,
        ),
    ];

    if let Some(shared_device) = &args.dsa_shared_device {
        targets.push(run_dsa_target(
            TargetSpec {
                target: "dsa-shared-memmove",
                operation: "memmove",
                family: "dsa",
                target_role: "optional-shared",
                device_path: shared_device,
            },
            args,
        ));
    }

    let ok = targets.iter().all(|target| target.ok);
    let first_failure = targets.iter().find(|target| !target.ok);
    let claim_eligible = ok && profile() == "release";

    BenchmarkReport {
        schema_version: SCHEMA_VERSION,
        ok,
        verdict: if ok { "pass" } else { "expected_failure" },
        claim_eligible,
        suite: SUITE,
        profile: profile(),
        requested_bytes: args.requested_bytes,
        iterations: args.iterations,
        warmup_iterations: WARMUP_ITERATIONS,
        clock: "std::time::Instant",
        failure_phase: first_failure.and_then(|target| target.failure_phase.clone()),
        error_kind: first_failure.and_then(|target| target.error_kind.clone()),
        failure_target: first_failure.map(|target| target.target.to_string()),
        failure_accelerator: first_failure.map(|target| target.family.to_string()),
        targets,
    }
}

fn run_dsa_target(spec: TargetSpec<'_>, args: &CliArgs) -> TargetReport {
    let src = deterministic_src(args.requested_bytes);
    let mut dst = vec![0u8; args.requested_bytes];

    let session = match IdxdSession::<Dsa>::open(spec.device_path) {
        Ok(session) => session,
        Err(err) => return session_failure_row(spec, args, err),
    };
    let work_queue_mode = Some(work_queue_mode(&session));

    dst.fill(0);
    if let Err(err) = session.memmove(&mut dst, &src) {
        return dsa_failure_row(spec, args, work_queue_mode, 0, err, "warmup");
    }

    let mut total_latency_ns = 0u64;
    let mut min_latency_ns = u64::MAX;
    let mut max_latency_ns = 0u64;
    let mut completed_operations = 0u64;
    let mut total_page_fault_retries = 0u64;
    let mut last_page_fault_retries = 0u32;
    let mut final_status = 0u8;

    for _ in 0..args.iterations {
        dst.fill(0);
        let started = Instant::now();
        let report = match session.memmove(&mut dst, &src) {
            Ok(report) => report,
            Err(err) => {
                return dsa_failure_row(
                    spec,
                    args,
                    work_queue_mode,
                    completed_operations,
                    err,
                    "measured_loop",
                );
            }
        };
        let latency_ns = duration_ns(started.elapsed()).max(1);
        total_latency_ns = total_latency_ns.saturating_add(latency_ns);
        min_latency_ns = min_latency_ns.min(latency_ns);
        max_latency_ns = max_latency_ns.max(latency_ns);
        completed_operations += 1;
        total_page_fault_retries += u64::from(report.page_fault_retries);
        last_page_fault_retries = report.page_fault_retries;
        final_status = report.final_status;
    }

    let stats = LatencyStats::new(
        completed_operations,
        total_latency_ns,
        min_latency_ns,
        max_latency_ns,
        args.requested_bytes,
    );

    pass_row(
        spec,
        args,
        work_queue_mode,
        stats,
        total_page_fault_retries,
        last_page_fault_retries,
        final_status,
        None,
        None,
        None,
        format!(
            "measured {} DSA memmove operations via IdxdSession<Dsa> on {}",
            completed_operations,
            spec.device_path.display()
        ),
    )
}

fn run_iax_target(spec: TargetSpec<'_>, args: &CliArgs) -> TargetReport {
    let src = deterministic_crc64_src(args.requested_bytes);
    let expected_crc64 = crc64_t10dif_field(&src);

    let session = match IdxdSession::<Iax>::open(spec.device_path) {
        Ok(session) => session,
        Err(err) => return session_failure_row(spec, args, err),
    };
    let work_queue_mode = Some(work_queue_mode(&session));

    match session.crc64(&src) {
        Ok(report) if report.crc64 == expected_crc64 => {}
        Ok(report) => {
            return crc64_mismatch_row(
                spec,
                args,
                work_queue_mode,
                0,
                report,
                expected_crc64,
                "warmup",
            );
        }
        Err(err) => return iax_failure_row(spec, args, work_queue_mode, 0, err, "warmup"),
    }

    let mut total_latency_ns = 0u64;
    let mut min_latency_ns = u64::MAX;
    let mut max_latency_ns = 0u64;
    let mut completed_operations = 0u64;
    let mut total_page_fault_retries = 0u64;
    let mut last_page_fault_retries = 0u32;
    let mut final_status = 0u8;
    let mut last_crc64 = expected_crc64;

    for _ in 0..args.iterations {
        let started = Instant::now();
        let report = match session.crc64(&src) {
            Ok(report) => report,
            Err(err) => {
                return iax_failure_row(
                    spec,
                    args,
                    work_queue_mode,
                    completed_operations,
                    err,
                    "measured_loop",
                );
            }
        };
        let latency_ns = duration_ns(started.elapsed()).max(1);

        if report.crc64 != expected_crc64 {
            return crc64_mismatch_row(
                spec,
                args,
                work_queue_mode,
                completed_operations,
                report,
                expected_crc64,
                "crc64_verify",
            );
        }

        total_latency_ns = total_latency_ns.saturating_add(latency_ns);
        min_latency_ns = min_latency_ns.min(latency_ns);
        max_latency_ns = max_latency_ns.max(latency_ns);
        completed_operations += 1;
        total_page_fault_retries += u64::from(report.page_fault_retries);
        last_page_fault_retries = report.page_fault_retries;
        final_status = report.final_status;
        last_crc64 = report.crc64;
    }

    let stats = LatencyStats::new(
        completed_operations,
        total_latency_ns,
        min_latency_ns,
        max_latency_ns,
        args.requested_bytes,
    );

    pass_row(
        spec,
        args,
        work_queue_mode,
        stats,
        total_page_fault_retries,
        last_page_fault_retries,
        final_status,
        Some(last_crc64),
        Some(expected_crc64),
        Some(true),
        format!(
            "measured {} IAX crc64 operations via IdxdSession<Iax> on {}",
            completed_operations,
            spec.device_path.display()
        ),
    )
}

fn pass_row(
    spec: TargetSpec<'_>,
    args: &CliArgs,
    work_queue_mode: Option<&'static str>,
    stats: LatencyStats,
    total_page_fault_retries: u64,
    last_page_fault_retries: u32,
    final_status: u8,
    crc64: Option<u64>,
    expected_crc64: Option<u64>,
    crc64_verified: Option<bool>,
    message: String,
) -> TargetReport {
    TargetReport {
        target: spec.target,
        operation: spec.operation,
        family: spec.family,
        device_path: spec.device_path.display().to_string(),
        work_queue_mode,
        target_role: spec.target_role,
        requested_bytes: args.requested_bytes,
        iterations: args.iterations,
        warmup_iterations: WARMUP_ITERATIONS,
        ok: true,
        verdict: "pass",
        claim_eligible: profile() == "release",
        completed_operations: stats.completed_operations,
        failed_operations: 0,
        elapsed_ns: Some(stats.elapsed_ns),
        min_latency_ns: Some(stats.min_latency_ns),
        mean_latency_ns: Some(stats.mean_latency_ns),
        max_latency_ns: Some(stats.max_latency_ns),
        ops_per_sec: Some(stats.ops_per_sec),
        bytes_per_sec: Some(stats.bytes_per_sec),
        total_page_fault_retries: Some(total_page_fault_retries),
        last_page_fault_retries: Some(last_page_fault_retries),
        final_status: Some(hex_u8(final_status)),
        completion_error_code: None,
        invalid_flags: None,
        fault_addr: None,
        crc64: crc64.map(hex_u64),
        expected_crc64: expected_crc64.map(hex_u64),
        crc64_verified,
        failure_phase: None,
        error_kind: None,
        message,
    }
}

fn session_failure_row(
    spec: TargetSpec<'_>,
    args: &CliArgs,
    err: IdxdSessionError,
) -> TargetReport {
    let failure_phase = match err {
        IdxdSessionError::InvalidDevicePath { .. } => "argument_validation",
        IdxdSessionError::QueueOpen { .. } => "queue_open",
    };
    let device_path = err
        .device_path()
        .unwrap_or(spec.device_path)
        .display()
        .to_string();

    failure_row(
        spec,
        args,
        None,
        0,
        0,
        failure_phase,
        err.kind(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        err.to_string(),
        Some(device_path),
    )
}

fn dsa_failure_row(
    spec: TargetSpec<'_>,
    args: &CliArgs,
    work_queue_mode: Option<&'static str>,
    completed_operations: u64,
    err: MemmoveError,
    fallback_phase: &'static str,
) -> TargetReport {
    let device_path = err
        .device_path()
        .unwrap_or(spec.device_path)
        .display()
        .to_string();
    failure_row(
        spec,
        args,
        work_queue_mode,
        completed_operations,
        1,
        err.phase()
            .map(|phase| phase.to_string())
            .as_deref()
            .unwrap_or(fallback_phase),
        err.kind(),
        err.page_fault_retries().map(u64::from),
        err.page_fault_retries(),
        err.final_status().map(hex_u8),
        None,
        None,
        None,
        None,
        err.to_string(),
        Some(device_path),
    )
}

fn iax_failure_row(
    spec: TargetSpec<'_>,
    args: &CliArgs,
    work_queue_mode: Option<&'static str>,
    completed_operations: u64,
    err: IaxCrc64Error,
    fallback_phase: &'static str,
) -> TargetReport {
    let device_path = err
        .device_path()
        .unwrap_or(spec.device_path)
        .display()
        .to_string();
    failure_row(
        spec,
        args,
        work_queue_mode,
        completed_operations,
        1,
        err.phase()
            .map(|phase| phase.to_string())
            .as_deref()
            .unwrap_or(fallback_phase),
        err.kind(),
        err.page_fault_retries().map(u64::from),
        err.page_fault_retries(),
        err.final_status().map(hex_u8),
        err.error_code().map(hex_u8),
        err.invalid_flags().map(hex_u32),
        err.fault_addr().map(hex_u64),
        None,
        err.to_string(),
        Some(device_path),
    )
}

fn crc64_mismatch_row(
    spec: TargetSpec<'_>,
    args: &CliArgs,
    work_queue_mode: Option<&'static str>,
    completed_operations: u64,
    report: IaxCrc64Report,
    expected_crc64: u64,
    fallback_phase: &'static str,
) -> TargetReport {
    let mut row = failure_row(
        spec,
        args,
        work_queue_mode,
        completed_operations,
        1,
        fallback_phase,
        "crc64_mismatch",
        Some(u64::from(report.page_fault_retries)),
        Some(report.page_fault_retries),
        Some(hex_u8(report.final_status)),
        None,
        None,
        None,
        Some(false),
        format!(
            "crc64 mismatch via IdxdSession<Iax> on {}: hardware={}, expected={}",
            report.device_path.display(),
            hex_u64(report.crc64),
            hex_u64(expected_crc64)
        ),
        Some(report.device_path.display().to_string()),
    );
    row.crc64 = Some(hex_u64(report.crc64));
    row.expected_crc64 = Some(hex_u64(expected_crc64));
    row
}

fn failure_row(
    spec: TargetSpec<'_>,
    args: &CliArgs,
    work_queue_mode: Option<&'static str>,
    completed_operations: u64,
    failed_operations: u64,
    failure_phase: &str,
    error_kind: &str,
    total_page_fault_retries: Option<u64>,
    last_page_fault_retries: Option<u32>,
    final_status: Option<String>,
    completion_error_code: Option<String>,
    invalid_flags: Option<String>,
    fault_addr: Option<String>,
    crc64_verified: Option<bool>,
    message: String,
    device_path_override: Option<String>,
) -> TargetReport {
    TargetReport {
        target: spec.target,
        operation: spec.operation,
        family: spec.family,
        device_path: device_path_override.unwrap_or_else(|| spec.device_path.display().to_string()),
        work_queue_mode,
        target_role: spec.target_role,
        requested_bytes: args.requested_bytes,
        iterations: args.iterations,
        warmup_iterations: WARMUP_ITERATIONS,
        ok: false,
        verdict: "expected_failure",
        claim_eligible: false,
        completed_operations,
        failed_operations,
        elapsed_ns: None,
        min_latency_ns: None,
        mean_latency_ns: None,
        max_latency_ns: None,
        ops_per_sec: None,
        bytes_per_sec: None,
        total_page_fault_retries,
        last_page_fault_retries,
        final_status,
        completion_error_code,
        invalid_flags,
        fault_addr,
        crc64: None,
        expected_crc64: None,
        crc64_verified,
        failure_phase: Some(failure_phase.to_string()),
        error_kind: Some(error_kind.to_string()),
        message,
    }
}

fn emit_report(args: &CliArgs, report: &BenchmarkReport) -> Result<(), String> {
    let rendered = match args.format {
        OutputFormat::Json => serde_json::to_string(report)
            .map_err(|err| format!("failed to serialize benchmark artifact: {err}"))?,
        OutputFormat::Text => render_text(report),
    };

    if let Some(path) = &args.artifact_path {
        write_artifact(path, &rendered)?;
    }

    println!("{rendered}");
    Ok(())
}

fn render_text(report: &BenchmarkReport) -> String {
    let mut text = String::new();
    let _ = writeln!(text, "schema_version={}", report.schema_version);
    let _ = writeln!(text, "ok={}", report.ok);
    let _ = writeln!(text, "verdict={}", report.verdict);
    let _ = writeln!(text, "claim_eligible={}", report.claim_eligible);
    let _ = writeln!(text, "suite={}", report.suite);
    let _ = writeln!(text, "profile={}", report.profile);
    let _ = writeln!(text, "requested_bytes={}", report.requested_bytes);
    let _ = writeln!(text, "iterations={}", report.iterations);
    let _ = writeln!(text, "warmup_iterations={}", report.warmup_iterations);
    let _ = writeln!(text, "clock={}", report.clock);
    let _ = writeln!(
        text,
        "failure_phase={}",
        report.failure_phase.as_deref().unwrap_or("null")
    );
    let _ = writeln!(
        text,
        "error_kind={}",
        report.error_kind.as_deref().unwrap_or("null")
    );

    for target in &report.targets {
        let prefix = target.target;
        let _ = writeln!(text, "{prefix}.operation={}", target.operation);
        let _ = writeln!(text, "{prefix}.family={}", target.family);
        let _ = writeln!(text, "{prefix}.device_path={}", target.device_path);
        let _ = writeln!(
            text,
            "{prefix}.work_queue_mode={}",
            target.work_queue_mode.unwrap_or("null")
        );
        let _ = writeln!(text, "{prefix}.ok={}", target.ok);
        let _ = writeln!(text, "{prefix}.verdict={}", target.verdict);
        let _ = writeln!(
            text,
            "{prefix}.completed_operations={}",
            target.completed_operations
        );
        let _ = writeln!(
            text,
            "{prefix}.failed_operations={}",
            target.failed_operations
        );
        let _ = writeln!(
            text,
            "{prefix}.elapsed_ns={}",
            opt_display(target.elapsed_ns)
        );
        let _ = writeln!(
            text,
            "{prefix}.min_latency_ns={}",
            opt_display(target.min_latency_ns)
        );
        let _ = writeln!(
            text,
            "{prefix}.mean_latency_ns={}",
            opt_display(target.mean_latency_ns)
        );
        let _ = writeln!(
            text,
            "{prefix}.max_latency_ns={}",
            opt_display(target.max_latency_ns)
        );
        let _ = writeln!(
            text,
            "{prefix}.ops_per_sec={}",
            opt_display(target.ops_per_sec)
        );
        let _ = writeln!(
            text,
            "{prefix}.bytes_per_sec={}",
            opt_display(target.bytes_per_sec)
        );
        let _ = writeln!(
            text,
            "{prefix}.total_page_fault_retries={}",
            opt_display(target.total_page_fault_retries)
        );
        let _ = writeln!(
            text,
            "{prefix}.last_page_fault_retries={}",
            opt_display(target.last_page_fault_retries)
        );
        let _ = writeln!(
            text,
            "{prefix}.final_status={}",
            target.final_status.as_deref().unwrap_or("null")
        );
        let _ = writeln!(
            text,
            "{prefix}.completion_error_code={}",
            target.completion_error_code.as_deref().unwrap_or("null")
        );
        let _ = writeln!(
            text,
            "{prefix}.invalid_flags={}",
            target.invalid_flags.as_deref().unwrap_or("null")
        );
        let _ = writeln!(
            text,
            "{prefix}.fault_addr={}",
            target.fault_addr.as_deref().unwrap_or("null")
        );
        let _ = writeln!(
            text,
            "{prefix}.crc64={}",
            target.crc64.as_deref().unwrap_or("null")
        );
        let _ = writeln!(
            text,
            "{prefix}.expected_crc64={}",
            target.expected_crc64.as_deref().unwrap_or("null")
        );
        let _ = writeln!(
            text,
            "{prefix}.crc64_verified={}",
            target
                .crc64_verified
                .map(|value| value.to_string())
                .unwrap_or_else(|| "null".to_string())
        );
        let _ = writeln!(
            text,
            "{prefix}.failure_phase={}",
            target.failure_phase.as_deref().unwrap_or("null")
        );
        let _ = writeln!(
            text,
            "{prefix}.error_kind={}",
            target.error_kind.as_deref().unwrap_or("null")
        );
    }

    text.trim_end().to_string()
}

fn parse_device_path(flag: &str, raw: String) -> Result<PathBuf, String> {
    if raw.is_empty() {
        return Err(format!("device path for `{flag}` must not be empty"));
    }
    Ok(PathBuf::from(raw))
}

fn parse_requested_bytes(raw: &str) -> Result<usize, String> {
    let requested_bytes = raw
        .parse::<usize>()
        .map_err(|_| format!("invalid value `{raw}` for `--bytes`; expected a positive integer"))?;
    if requested_bytes == 0 || requested_bytes > MAX_REQUESTED_BYTES {
        return Err(format!(
            "invalid value `{raw}` for `--bytes`; expected 1..={MAX_REQUESTED_BYTES}"
        ));
    }
    Ok(requested_bytes)
}

fn parse_iterations(raw: &str) -> Result<u64, String> {
    let iterations = raw.parse::<u64>().map_err(|_| {
        format!("invalid value `{raw}` for `--iterations`; expected a positive integer")
    })?;
    if iterations == 0 {
        return Err(format!(
            "invalid value `{raw}` for `--iterations`; expected a positive integer"
        ));
    }
    Ok(iterations)
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

fn deterministic_src(len: usize) -> Vec<u8> {
    (0..len)
        .map(|index| ((index * 31 + 17) % 251) as u8)
        .collect()
}

fn deterministic_crc64_src(len: usize) -> Vec<u8> {
    if len < 2 {
        return vec![0; len];
    }

    let mut src = deterministic_src(len);
    for high in 0..=u8::MAX {
        for low in 0..=u8::MAX {
            src[len - 2] = high;
            src[len - 1] = low;
            if crc64_t10dif_field(&src) == 0 {
                return src;
            }
        }
    }

    vec![0; len]
}

fn work_queue_mode<Accel: idxd_rust::Accelerator>(session: &IdxdSession<Accel>) -> &'static str {
    if session.is_dedicated_wq() {
        "dedicated"
    } else {
        "shared"
    }
}

fn duration_ns(duration: Duration) -> u64 {
    match u64::try_from(duration.as_nanos()) {
        Ok(value) => value,
        Err(_) => u64::MAX,
    }
}

fn profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

fn opt_display<T: std::fmt::Display>(value: Option<T>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn hex_u8(value: u8) -> String {
    format!("0x{value:02x}")
}

fn hex_u32(value: u32) -> String {
    format!("0x{value:08x}")
}

fn hex_u64(value: u64) -> String {
    format!("0x{value:x}")
}

fn print_help() {
    println!(
        "Usage: idxd_representative_bench --dsa-device PATH --iax-device PATH [--dsa-shared-device PATH] [--bytes N] [--iterations N] [--format text|json] [--artifact PATH]"
    );
    println!(
        "Runs a small no-payload benchmark over IdxdSession<Dsa>::memmove and IdxdSession<Iax>::crc64."
    );
}
