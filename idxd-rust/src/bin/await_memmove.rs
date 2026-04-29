use std::env;
use std::fmt::Write as _;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use bytes::{Bytes, BytesMut, buf::UninitSlice};
use idxd_rust::{
    AsyncDsaSession, AsyncMemmoveError, AsyncMemmoveRequest, AsyncMemmoveWorker,
    DEFAULT_DEVICE_PATH, DsaConfig, MemmoveError, MemmovePhase, MemmoveRequest,
    MemmoveValidationReport,
};

const TEST_SCENARIO_ENV: &str = "IDXD_RUST_AWAIT_MEMMOVE_TEST_SCENARIO";

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run().await {
        Ok(exit) => exit,
        Err(err) => {
            let _ = writeln!(io::stderr(), "await_memmove: {err}");
            ExitCode::from(2)
        }
    }
}

async fn run() -> Result<ExitCode, String> {
    let args = CliArgs::parse(env::args().skip(1))?;
    let outcome = execute(&args).await;
    emit_outcome(&args, &outcome).map_err(|err| err.to_string())?;
    Ok(if outcome.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TestScenario {
    Success,
    OwnerShutdown,
    WorkerFailure,
    CompletionTimeout,
}

impl TestScenario {
    fn from_env() -> Result<Option<Self>, String> {
        match env::var(TEST_SCENARIO_ENV) {
            Ok(value) => Self::parse(&value).map(Some),
            Err(env::VarError::NotPresent) => Ok(None),
            Err(env::VarError::NotUnicode(_)) => Err(format!(
                "environment variable `{TEST_SCENARIO_ENV}` must be valid UTF-8"
            )),
        }
    }

    fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "success" => Ok(Self::Success),
            "owner_shutdown" => Ok(Self::OwnerShutdown),
            "worker_failure" => Ok(Self::WorkerFailure),
            "completion_timeout" => Ok(Self::CompletionTimeout),
            other => Err(format!(
                "unsupported `{TEST_SCENARIO_ENV}` value `{other}`; expected success, owner_shutdown, worker_failure, or completion_timeout"
            )),
        }
    }
}

#[derive(Debug, Clone)]
struct CliArgs {
    device_path: PathBuf,
    requested_bytes: usize,
    format: OutputFormat,
    artifact_path: Option<PathBuf>,
}

impl CliArgs {
    fn parse<I>(mut args: I) -> Result<Self, String>
    where
        I: Iterator<Item = String>,
    {
        let mut device_path = PathBuf::from(DEFAULT_DEVICE_PATH);
        let mut requested_bytes = 4096usize;
        let mut format = OutputFormat::Text;
        let mut artifact_path = None;

        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--device" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "missing value for `--device`".to_string())?;
                    if value.is_empty() {
                        return Err("device path must not be empty".to_string());
                    }
                    device_path = PathBuf::from(value);
                }
                "--bytes" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "missing value for `--bytes`".to_string())?;
                    requested_bytes = value.parse::<usize>().map_err(|_| {
                        format!(
                            "invalid value `{value}` for `--bytes`; expected a positive integer"
                        )
                    })?;
                    MemmoveRequest::new(requested_bytes).map_err(|err| err.to_string())?;
                }
                "--format" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "missing value for `--format`".to_string())?;
                    format = OutputFormat::parse(&value)?;
                }
                "--artifact" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "missing value for `--artifact`".to_string())?;
                    let path = PathBuf::from(value);
                    validate_artifact_path(&path)?;
                    artifact_path = Some(path);
                }
                "--help" | "-h" => {
                    print_help();
                    return Ok(Self {
                        device_path,
                        requested_bytes,
                        format,
                        artifact_path,
                    });
                }
                other => {
                    return Err(format!(
                        "unsupported argument `{other}`; expected `--device`, `--bytes`, `--format`, or `--artifact`"
                    ));
                }
            }
        }

        Ok(Self {
            device_path,
            requested_bytes,
            format,
            artifact_path,
        })
    }
}

#[derive(Debug, Clone)]
struct RunOutcome {
    ok: bool,
    device_path: String,
    requested_bytes: usize,
    page_fault_retries: Option<u32>,
    final_status: Option<u8>,
    phase: String,
    error_kind: Option<&'static str>,
    lifecycle_failure_kind: Option<&'static str>,
    worker_failure_kind: Option<&'static str>,
    direct_failure_kind: Option<&'static str>,
    retry_budget: Option<u32>,
    retry_count: Option<u32>,
    completion_result: Option<u8>,
    completion_bytes_completed: Option<u32>,
    completion_fault_addr: Option<u64>,
    validation_phase: Option<String>,
    validation_error_kind: Option<&'static str>,
    message: String,
}

async fn execute(args: &CliArgs) -> RunOutcome {
    let src = deterministic_src(args.requested_bytes);
    let request = match AsyncMemmoveRequest::new(
        Bytes::from(src),
        BytesMut::with_capacity(args.requested_bytes),
    ) {
        Ok(request) => request,
        Err(err) => {
            let (err, _source, _destination) = err.into_parts();
            return validation_failure_outcome(args, err);
        }
    };

    match TestScenario::from_env() {
        Ok(Some(scenario)) => execute_test_scenario(args, request, scenario).await,
        Ok(None) => execute_live(args, request).await,
        Err(err) => RunOutcome {
            ok: false,
            device_path: args.device_path.display().to_string(),
            requested_bytes: args.requested_bytes,
            page_fault_retries: None,
            final_status: None,
            phase: "argument_validation".to_string(),
            error_kind: Some("validation_failure"),
            lifecycle_failure_kind: None,
            worker_failure_kind: None,
            direct_failure_kind: None,
            retry_budget: None,
            retry_count: None,
            completion_result: None,
            completion_bytes_completed: None,
            completion_fault_addr: None,
            validation_phase: Some("argument_validation".to_string()),
            validation_error_kind: Some("invalid_test_scenario"),
            message: err,
        },
    }
}

async fn execute_live(args: &CliArgs, request: AsyncMemmoveRequest) -> RunOutcome {
    let config = match DsaConfig::builder()
        .device_path(args.device_path.clone())
        .build()
    {
        Ok(config) => config,
        Err(err) => return validation_failure_outcome(args, err),
    };
    let session = match AsyncDsaSession::open_config(config) {
        Ok(session) => session,
        Err(err) => return async_failure_outcome(args, err),
    };

    execute_with_handle(args, session, request).await
}

async fn execute_test_scenario(
    args: &CliArgs,
    request: AsyncMemmoveRequest,
    scenario: TestScenario,
) -> RunOutcome {
    match scenario {
        TestScenario::Success => {
            let device_path = args.device_path.clone();
            let session = match AsyncDsaSession::spawn_with_factory(move || {
                Ok(SuccessWorker { device_path })
            }) {
                Ok(session) => session,
                Err(err) => return async_failure_outcome(args, err),
            };
            execute_with_handle(args, session, request).await
        }
        TestScenario::OwnerShutdown => {
            let device_path = args.device_path.clone();
            let session = match AsyncDsaSession::spawn_with_factory(move || {
                Ok(SuccessWorker { device_path })
            }) {
                Ok(session) => session,
                Err(err) => return async_failure_outcome(args, err),
            };
            execute_after_owner_shutdown(args, session, request).await
        }
        TestScenario::WorkerFailure => {
            let previous_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(|_| {}));

            let outcome = match AsyncDsaSession::spawn_with_factory(|| Ok(PanicWorker)) {
                Ok(session) => execute_with_handle(args, session, request).await,
                Err(err) => async_failure_outcome(args, err),
            };

            std::panic::set_hook(previous_hook);
            outcome
        }
        TestScenario::CompletionTimeout => {
            let device_path = args.device_path.clone();
            let session = match AsyncDsaSession::spawn_with_factory(move || {
                Ok(ErrorWorker {
                    error: Some(MemmoveError::CompletionTimeout {
                        device_path,
                        phase: MemmovePhase::CompletionPoll,
                        page_fault_retries: 2,
                    }),
                })
            }) {
                Ok(session) => session,
                Err(err) => return async_failure_outcome(args, err),
            };
            execute_with_handle(args, session, request).await
        }
    }
}

async fn execute_with_handle(
    args: &CliArgs,
    session: AsyncDsaSession,
    request: AsyncMemmoveRequest,
) -> RunOutcome {
    let handle = session.handle();
    let memmove_result = handle.memmove(request).await;
    let shutdown_result = session.shutdown();
    map_execution_outcome(args, memmove_result, shutdown_result)
}

async fn execute_after_owner_shutdown(
    args: &CliArgs,
    session: AsyncDsaSession,
    request: AsyncMemmoveRequest,
) -> RunOutcome {
    let handle = session.handle();
    match session.shutdown() {
        Ok(()) => {
            let memmove_result = handle.memmove(request).await;
            map_execution_outcome(args, memmove_result, Ok(()))
        }
        Err(err) => async_failure_outcome(args, err),
    }
}

fn map_execution_outcome(
    args: &CliArgs,
    memmove_result: Result<idxd_rust::AsyncMemmoveResult, AsyncMemmoveError>,
    shutdown_result: Result<(), AsyncMemmoveError>,
) -> RunOutcome {
    match (memmove_result, shutdown_result) {
        (Ok(result), Ok(())) => success_outcome(result.report),
        (Err(err), Ok(())) => async_failure_outcome(args, err),
        (Ok(_), Err(err)) => async_failure_outcome(args, err),
        (Err(err), Err(_shutdown_err)) => async_failure_outcome(args, err),
    }
}

fn success_outcome(report: MemmoveValidationReport) -> RunOutcome {
    RunOutcome {
        ok: true,
        device_path: report.device_path.display().to_string(),
        requested_bytes: report.requested_bytes,
        page_fault_retries: Some(report.page_fault_retries),
        final_status: Some(report.final_status),
        phase: "completed".to_string(),
        error_kind: None,
        lifecycle_failure_kind: None,
        worker_failure_kind: None,
        direct_failure_kind: None,
        retry_budget: Some(report.page_fault_retries),
        retry_count: Some(report.page_fault_retries),
        completion_result: None,
        completion_bytes_completed: None,
        completion_fault_addr: None,
        validation_phase: Some("completed".to_string()),
        validation_error_kind: None,
        message: format!(
            "verified {} copied bytes via direct async memmove on {}",
            report.requested_bytes,
            report.device_path.display()
        ),
    }
}

fn async_failure_outcome(args: &CliArgs, err: AsyncMemmoveError) -> RunOutcome {
    match err {
        AsyncMemmoveError::Memmove { source, .. } => validation_failure_outcome(args, source),
        AsyncMemmoveError::LifecycleFailure { kind, .. } => RunOutcome {
            ok: false,
            device_path: args.device_path.display().to_string(),
            requested_bytes: args.requested_bytes,
            page_fault_retries: None,
            final_status: None,
            phase: "async_lifecycle".to_string(),
            error_kind: Some("lifecycle_failure"),
            lifecycle_failure_kind: Some(kind.as_str()),
            worker_failure_kind: None,
            direct_failure_kind: None,
            retry_budget: None,
            retry_count: None,
            completion_result: None,
            completion_bytes_completed: None,
            completion_fault_addr: None,
            validation_phase: None,
            validation_error_kind: None,
            message: format!("async memmove lifecycle failure: {}", kind.as_str()),
        },
        AsyncMemmoveError::WorkerFailure { kind, .. } => RunOutcome {
            ok: false,
            device_path: args.device_path.display().to_string(),
            requested_bytes: args.requested_bytes,
            page_fault_retries: None,
            final_status: None,
            phase: "async_worker".to_string(),
            error_kind: Some("worker_failure"),
            lifecycle_failure_kind: None,
            worker_failure_kind: Some(kind.as_str()),
            direct_failure_kind: None,
            retry_budget: None,
            retry_count: None,
            completion_result: None,
            completion_bytes_completed: None,
            completion_fault_addr: None,
            validation_phase: None,
            validation_error_kind: None,
            message: format!("async memmove worker failure: {}", kind.as_str()),
        },
        AsyncMemmoveError::DirectFailure { failure, .. } => RunOutcome {
            ok: false,
            device_path: args.device_path.display().to_string(),
            requested_bytes: failure.requested_bytes(),
            page_fault_retries: Some(failure.retry_count()),
            final_status: failure
                .completion_snapshot()
                .map(|snapshot| snapshot.status),
            phase: "async_direct".to_string(),
            error_kind: Some("direct_failure"),
            lifecycle_failure_kind: None,
            worker_failure_kind: None,
            direct_failure_kind: Some(failure.kind().as_str()),
            retry_budget: Some(failure.retry_budget()),
            retry_count: Some(failure.retry_count()),
            completion_result: failure
                .completion_snapshot()
                .map(|snapshot| snapshot.result),
            completion_bytes_completed: failure
                .completion_snapshot()
                .map(|snapshot| snapshot.bytes_completed),
            completion_fault_addr: failure
                .completion_snapshot()
                .map(|snapshot| snapshot.fault_addr),
            validation_phase: None,
            validation_error_kind: None,
            message: format!("async direct memmove failure: {failure}"),
        },
    }
}

fn validation_failure_outcome(args: &CliArgs, err: MemmoveError) -> RunOutcome {
    let validation_phase = err
        .phase()
        .map(|phase| phase.to_string())
        .unwrap_or_else(|| "argument_validation".to_string());

    RunOutcome {
        ok: false,
        device_path: err
            .device_path()
            .unwrap_or(args.device_path.as_path())
            .display()
            .to_string(),
        requested_bytes: err.requested_bytes().unwrap_or(args.requested_bytes),
        page_fault_retries: err.page_fault_retries(),
        final_status: err.final_status(),
        phase: validation_phase.clone(),
        error_kind: Some("validation_failure"),
        lifecycle_failure_kind: None,
        worker_failure_kind: None,
        direct_failure_kind: None,
        retry_budget: None,
        retry_count: err.page_fault_retries(),
        completion_result: None,
        completion_bytes_completed: None,
        completion_fault_addr: None,
        validation_phase: Some(validation_phase),
        validation_error_kind: Some(err.kind()),
        message: err.to_string(),
    }
}

fn emit_outcome(args: &CliArgs, outcome: &RunOutcome) -> io::Result<()> {
    let rendered = match args.format {
        OutputFormat::Text => render_text(outcome),
        OutputFormat::Json => render_json(outcome),
    };

    if let Some(path) = &args.artifact_path {
        std::fs::write(path, rendered.as_bytes())?;
    }

    println!("{rendered}");
    Ok(())
}

fn render_text(outcome: &RunOutcome) -> String {
    let mut text = String::new();
    let _ = writeln!(text, "ok={}", outcome.ok);
    let _ = writeln!(text, "device_path={}", outcome.device_path);
    let _ = writeln!(text, "requested_bytes={}", outcome.requested_bytes);
    let _ = writeln!(
        text,
        "page_fault_retries={}",
        outcome
            .page_fault_retries
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_string())
    );
    let _ = writeln!(
        text,
        "final_status={}",
        outcome
            .final_status
            .map(hex_status)
            .unwrap_or_else(|| "null".to_string())
    );
    let _ = writeln!(text, "phase={}", outcome.phase);
    let _ = writeln!(text, "error_kind={}", outcome.error_kind.unwrap_or("null"));
    let _ = writeln!(
        text,
        "lifecycle_failure_kind={}",
        outcome.lifecycle_failure_kind.unwrap_or("null")
    );
    let _ = writeln!(
        text,
        "worker_failure_kind={}",
        outcome.worker_failure_kind.unwrap_or("null")
    );
    let _ = writeln!(
        text,
        "direct_failure_kind={}",
        outcome.direct_failure_kind.unwrap_or("null")
    );
    let _ = writeln!(
        text,
        "retry_budget={}",
        outcome
            .retry_budget
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_string())
    );
    let _ = writeln!(
        text,
        "retry_count={}",
        outcome
            .retry_count
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_string())
    );
    let _ = writeln!(
        text,
        "completion_result={}",
        outcome
            .completion_result
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_string())
    );
    let _ = writeln!(
        text,
        "completion_bytes_completed={}",
        outcome
            .completion_bytes_completed
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_string())
    );
    let _ = writeln!(
        text,
        "completion_fault_addr={}",
        outcome
            .completion_fault_addr
            .map(hex_addr)
            .unwrap_or_else(|| "null".to_string())
    );
    let _ = writeln!(
        text,
        "validation_phase={}",
        outcome.validation_phase.as_deref().unwrap_or("null")
    );
    let _ = writeln!(
        text,
        "validation_error_kind={}",
        outcome.validation_error_kind.unwrap_or("null")
    );
    let _ = write!(text, "message={}", outcome.message);
    text
}

fn render_json(outcome: &RunOutcome) -> String {
    format!(
        concat!(
            "{{",
            "\"ok\":{},",
            "\"device_path\":\"{}\",",
            "\"requested_bytes\":{},",
            "\"page_fault_retries\":{},",
            "\"final_status\":{},",
            "\"phase\":\"{}\",",
            "\"error_kind\":{},",
            "\"lifecycle_failure_kind\":{},",
            "\"worker_failure_kind\":{},",
            "\"direct_failure_kind\":{},",
            "\"retry_budget\":{},",
            "\"retry_count\":{},",
            "\"completion_result\":{},",
            "\"completion_bytes_completed\":{},",
            "\"completion_fault_addr\":{},",
            "\"validation_phase\":{},",
            "\"validation_error_kind\":{},",
            "\"message\":\"{}\"",
            "}}"
        ),
        outcome.ok,
        escape_json(&outcome.device_path),
        outcome.requested_bytes,
        outcome
            .page_fault_retries
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_string()),
        outcome
            .final_status
            .map(|value| format!("\"{}\"", hex_status(value)))
            .unwrap_or_else(|| "null".to_string()),
        escape_json(&outcome.phase),
        outcome
            .error_kind
            .map(|value| format!("\"{}\"", escape_json(value)))
            .unwrap_or_else(|| "null".to_string()),
        outcome
            .lifecycle_failure_kind
            .map(|value| format!("\"{}\"", escape_json(value)))
            .unwrap_or_else(|| "null".to_string()),
        outcome
            .worker_failure_kind
            .map(|value| format!("\"{}\"", escape_json(value)))
            .unwrap_or_else(|| "null".to_string()),
        outcome
            .direct_failure_kind
            .map(|value| format!("\"{}\"", escape_json(value)))
            .unwrap_or_else(|| "null".to_string()),
        outcome
            .retry_budget
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_string()),
        outcome
            .retry_count
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_string()),
        outcome
            .completion_result
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_string()),
        outcome
            .completion_bytes_completed
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_string()),
        outcome
            .completion_fault_addr
            .map(|value| format!("\"{}\"", hex_addr(value)))
            .unwrap_or_else(|| "null".to_string()),
        outcome
            .validation_phase
            .as_ref()
            .map(|value| format!("\"{}\"", escape_json(value)))
            .unwrap_or_else(|| "null".to_string()),
        outcome
            .validation_error_kind
            .map(|value| format!("\"{}\"", escape_json(value)))
            .unwrap_or_else(|| "null".to_string()),
        escape_json(&outcome.message),
    )
}

fn validate_artifact_path(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("artifact path must not be empty".to_string());
    }

    if path.is_dir() {
        return Err(format!(
            "artifact path `{}` is a directory; expected a writable file path",
            path.display()
        ));
    }

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err(format!(
                "artifact parent directory `{}` does not exist",
                parent.display()
            ));
        }
    }

    OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map(|_| ())
        .map_err(|err| format!("artifact path `{}` is not writable: {err}", path.display()))
}

fn deterministic_src(len: usize) -> Vec<u8> {
    (0..len)
        .map(|index| ((index * 31 + 17) % 251) as u8)
        .collect()
}

fn hex_status(status: u8) -> String {
    format!("0x{status:02x}")
}

fn hex_addr(addr: u64) -> String {
    format!("0x{addr:x}")
}

fn escape_json(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| match ch {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            other => vec![other],
        })
        .collect()
}

fn print_help() {
    println!(
        "Usage: await_memmove [--device PATH] [--bytes N] [--format text|json] [--artifact PATH]"
    );
    println!("Runs one real DSA memmove through the direct async path and prints a stable report.");
}

struct SuccessWorker {
    device_path: PathBuf,
}

impl AsyncMemmoveWorker for SuccessWorker {
    fn memmove(
        &mut self,
        dst: &mut UninitSlice,
        src: &[u8],
    ) -> Result<MemmoveValidationReport, MemmoveError> {
        dst.copy_from_slice(src);
        MemmoveValidationReport::new(&self.device_path, MemmoveRequest::new(src.len())?, 0, 1)
    }
}

struct ErrorWorker {
    error: Option<MemmoveError>,
}

impl AsyncMemmoveWorker for ErrorWorker {
    fn memmove(
        &mut self,
        _dst: &mut UninitSlice,
        _src: &[u8],
    ) -> Result<MemmoveValidationReport, MemmoveError> {
        Err(self
            .error
            .take()
            .expect("test scenario should only issue one request"))
    }
}

struct PanicWorker;

impl AsyncMemmoveWorker for PanicWorker {
    fn memmove(
        &mut self,
        _dst: &mut UninitSlice,
        _src: &[u8],
    ) -> Result<MemmoveValidationReport, MemmoveError> {
        panic!("worker dropped before replying");
    }
}
