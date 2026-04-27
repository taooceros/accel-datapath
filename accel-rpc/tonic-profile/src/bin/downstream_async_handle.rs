use std::env;
use std::fmt::Write as _;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use idxd_rust::{
    AsyncDsaSession, AsyncMemmoveError, AsyncMemmoveRequest, AsyncMemmoveWorker, MemmoveError,
    MemmovePhase, MemmoveRequest, MemmoveValidationReport, DEFAULT_DEVICE_PATH,
};

const TEST_SCENARIO_ENV: &str = "IDXD_TONIC_ASYNC_HANDLE_TEST_SCENARIO";
const PROOF_SEAM: &str = "downstream_async_handle";
const CONSUMER_PACKAGE: &str = "tonic-profile";
const BINDING_PACKAGE: &str = "idxd-rust";
const COMPOSITION: &str = "tokio_join";
const OPERATION_COUNT: usize = 2;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run().await {
        Ok(exit) => exit,
        Err(err) => {
            let _ = writeln!(io::stderr(), "downstream_async_handle: {err}");
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
    InvalidDestinationLen,
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
            "invalid_destination_len" => Ok(Self::InvalidDestinationLen),
            "completion_timeout" => Ok(Self::CompletionTimeout),
            other => Err(format!(
                "unsupported `{TEST_SCENARIO_ENV}` value `{other}`; expected success, owner_shutdown, worker_failure, invalid_destination_len, or completion_timeout"
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
    proof_seam: &'static str,
    consumer_package: &'static str,
    binding_package: &'static str,
    composition: &'static str,
    operation_count: usize,
    device_path: String,
    requested_bytes: usize,
    phase: String,
    error_kind: Option<&'static str>,
    lifecycle_failure_kind: Option<&'static str>,
    worker_failure_kind: Option<&'static str>,
    validation_phase: Option<String>,
    validation_error_kind: Option<&'static str>,
    message: String,
}

async fn execute(args: &CliArgs) -> RunOutcome {
    match TestScenario::from_env() {
        Ok(Some(TestScenario::InvalidDestinationLen)) => execute_invalid_destination_len(args),
        Ok(Some(scenario)) => {
            let requests = match build_requests(args.requested_bytes) {
                Ok(requests) => requests,
                Err(err) => return validation_failure_outcome(args, err),
            };
            execute_test_scenario(args, requests, scenario).await
        }
        Ok(None) => {
            let requests = match build_requests(args.requested_bytes) {
                Ok(requests) => requests,
                Err(err) => return validation_failure_outcome(args, err),
            };
            execute_live(args, requests).await
        }
        Err(err) => base_outcome(
            args,
            false,
            "argument_validation",
            Some("validation_failure"),
            err,
        )
        .with_validation("argument_validation", Some("invalid_test_scenario")),
    }
}

fn build_requests(
    requested_bytes: usize,
) -> Result<[AsyncMemmoveRequest; OPERATION_COUNT], MemmoveError> {
    let first = AsyncMemmoveRequest::copy_exact(deterministic_src(requested_bytes, 0))?;
    let second = AsyncMemmoveRequest::copy_exact(deterministic_src(requested_bytes, 1))?;
    Ok([first, second])
}

fn execute_invalid_destination_len(args: &CliArgs) -> RunOutcome {
    let src = deterministic_src(args.requested_bytes, 0);
    let invalid_destination_len = args.requested_bytes.saturating_sub(1);
    match AsyncMemmoveRequest::copy_into(src, vec![0u8; invalid_destination_len]) {
        Ok(_) => base_outcome(
            args,
            false,
            "argument_validation",
            Some("validation_failure"),
            "invalid_destination_len scenario unexpectedly accepted mismatched buffers".to_string(),
        )
        .with_validation("argument_validation", Some("scenario_bug")),
        Err(err) => validation_failure_outcome(args, err),
    }
}

async fn execute_live(
    args: &CliArgs,
    requests: [AsyncMemmoveRequest; OPERATION_COUNT],
) -> RunOutcome {
    let session = match AsyncDsaSession::open(&args.device_path) {
        Ok(session) => session,
        Err(err) => return async_failure_outcome(args, err),
    };

    execute_with_session(args, session, requests).await
}

async fn execute_test_scenario(
    args: &CliArgs,
    requests: [AsyncMemmoveRequest; OPERATION_COUNT],
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
            execute_with_session(args, session, requests).await
        }
        TestScenario::OwnerShutdown => {
            let device_path = args.device_path.clone();
            let session = match AsyncDsaSession::spawn_with_factory(move || {
                Ok(SuccessWorker { device_path })
            }) {
                Ok(session) => session,
                Err(err) => return async_failure_outcome(args, err),
            };
            execute_after_owner_shutdown(args, session, requests).await
        }
        TestScenario::WorkerFailure => {
            let previous_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(|_| {}));
            let outcome = match AsyncDsaSession::spawn_with_factory(|| Ok(PanicWorker)) {
                Ok(session) => execute_with_session(args, session, requests).await,
                Err(err) => async_failure_outcome(args, err),
            };
            std::panic::set_hook(previous_hook);
            outcome
        }
        TestScenario::CompletionTimeout => {
            let device_path = args.device_path.clone();
            let session = match AsyncDsaSession::spawn_with_factory(move || {
                Ok(CompletionTimeoutWorker { device_path })
            }) {
                Ok(session) => session,
                Err(err) => return async_failure_outcome(args, err),
            };
            execute_with_session(args, session, requests).await
        }
        TestScenario::InvalidDestinationLen => execute_invalid_destination_len(args),
    }
}

async fn execute_with_session(
    args: &CliArgs,
    session: AsyncDsaSession,
    requests: [AsyncMemmoveRequest; OPERATION_COUNT],
) -> RunOutcome {
    let [first_request, second_request] = requests;
    let expected_first = first_request.source_bytes().to_vec();
    let expected_second = second_request.source_bytes().to_vec();
    let first_handle = session.handle();
    let second_handle = session.handle();

    let (first_result, second_result) = tokio::join!(
        first_handle.memmove(first_request),
        second_handle.memmove(second_request)
    );
    let shutdown_result = session.shutdown();

    map_joined_outcome(
        args,
        first_result,
        second_result,
        shutdown_result,
        [&expected_first, &expected_second],
    )
}

async fn execute_after_owner_shutdown(
    args: &CliArgs,
    session: AsyncDsaSession,
    requests: [AsyncMemmoveRequest; OPERATION_COUNT],
) -> RunOutcome {
    let [first_request, second_request] = requests;
    let first_handle = session.handle();
    let second_handle = session.handle();

    match session.shutdown() {
        Ok(()) => {
            let (first_result, second_result) = tokio::join!(
                first_handle.memmove(first_request),
                second_handle.memmove(second_request)
            );
            map_joined_outcome(args, first_result, second_result, Ok(()), [&[], &[]])
        }
        Err(err) => async_failure_outcome(args, err),
    }
}

fn map_joined_outcome(
    args: &CliArgs,
    first_result: Result<idxd_rust::AsyncMemmoveResult, AsyncMemmoveError>,
    second_result: Result<idxd_rust::AsyncMemmoveResult, AsyncMemmoveError>,
    shutdown_result: Result<(), AsyncMemmoveError>,
    expected: [&[u8]; OPERATION_COUNT],
) -> RunOutcome {
    match (first_result, second_result, shutdown_result) {
        (Ok(first), Ok(second), Ok(())) => {
            if first.destination != expected[0] {
                return base_outcome(
                    args,
                    false,
                    "post_copy_verify",
                    Some("validation_failure"),
                    "first downstream async memmove returned unexpected destination".to_string(),
                )
                .with_validation("post_copy_verify", Some("byte_mismatch"));
            }
            if second.destination != expected[1] {
                return base_outcome(
                    args,
                    false,
                    "post_copy_verify",
                    Some("validation_failure"),
                    "second downstream async memmove returned unexpected destination".to_string(),
                )
                .with_validation("post_copy_verify", Some("byte_mismatch"));
            }
            success_outcome(args)
        }
        (Err(err), _, Ok(())) | (_, Err(err), Ok(())) => async_failure_outcome(args, err),
        (Ok(_), Ok(_), Err(err)) | (Err(_), _, Err(err)) | (_, Err(_), Err(err)) => {
            async_failure_outcome(args, err)
        }
    }
}

fn success_outcome(args: &CliArgs) -> RunOutcome {
    base_outcome(
        args,
        true,
        "completed",
        None,
        format!(
            "verified {OPERATION_COUNT} joined cloned-handle async memmoves of {} bytes via {BINDING_PACKAGE} from {CONSUMER_PACKAGE}",
            args.requested_bytes
        ),
    )
    .with_validation("completed", None)
}

fn async_failure_outcome(args: &CliArgs, err: AsyncMemmoveError) -> RunOutcome {
    match err {
        AsyncMemmoveError::Memmove(err) => validation_failure_outcome(args, err),
        AsyncMemmoveError::LifecycleFailure { kind } => base_outcome(
            args,
            false,
            "async_lifecycle",
            Some("lifecycle_failure"),
            format!(
                "downstream async-handle lifecycle failure: {}",
                kind.as_str()
            ),
        )
        .with_lifecycle(kind.as_str()),
        AsyncMemmoveError::WorkerFailure { kind } => base_outcome(
            args,
            false,
            "async_worker",
            Some("worker_failure"),
            format!("downstream async-handle worker failure: {}", kind.as_str()),
        )
        .with_worker(kind.as_str()),
    }
}

fn validation_failure_outcome(args: &CliArgs, err: MemmoveError) -> RunOutcome {
    let validation_phase = err
        .phase()
        .map(|phase| phase.to_string())
        .unwrap_or_else(|| "argument_validation".to_string());
    let device_path = err
        .device_path()
        .unwrap_or(args.device_path.as_path())
        .display()
        .to_string();
    let requested_bytes = err.requested_bytes().unwrap_or(args.requested_bytes);

    RunOutcome {
        ok: false,
        proof_seam: PROOF_SEAM,
        consumer_package: CONSUMER_PACKAGE,
        binding_package: BINDING_PACKAGE,
        composition: COMPOSITION,
        operation_count: OPERATION_COUNT,
        device_path,
        requested_bytes,
        phase: validation_phase.clone(),
        error_kind: Some("validation_failure"),
        lifecycle_failure_kind: None,
        worker_failure_kind: None,
        validation_phase: Some(validation_phase),
        validation_error_kind: Some(err.kind()),
        message: err.to_string(),
    }
}

fn base_outcome(
    args: &CliArgs,
    ok: bool,
    phase: impl Into<String>,
    error_kind: Option<&'static str>,
    message: String,
) -> RunOutcome {
    RunOutcome {
        ok,
        proof_seam: PROOF_SEAM,
        consumer_package: CONSUMER_PACKAGE,
        binding_package: BINDING_PACKAGE,
        composition: COMPOSITION,
        operation_count: OPERATION_COUNT,
        device_path: args.device_path.display().to_string(),
        requested_bytes: args.requested_bytes,
        phase: phase.into(),
        error_kind,
        lifecycle_failure_kind: None,
        worker_failure_kind: None,
        validation_phase: None,
        validation_error_kind: None,
        message,
    }
}

impl RunOutcome {
    fn with_lifecycle(mut self, kind: &'static str) -> Self {
        self.lifecycle_failure_kind = Some(kind);
        self
    }

    fn with_worker(mut self, kind: &'static str) -> Self {
        self.worker_failure_kind = Some(kind);
        self
    }

    fn with_validation(mut self, phase: impl Into<String>, kind: Option<&'static str>) -> Self {
        self.validation_phase = Some(phase.into());
        self.validation_error_kind = kind;
        self
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
    let _ = writeln!(text, "proof_seam={}", outcome.proof_seam);
    let _ = writeln!(text, "consumer_package={}", outcome.consumer_package);
    let _ = writeln!(text, "binding_package={}", outcome.binding_package);
    let _ = writeln!(text, "composition={}", outcome.composition);
    let _ = writeln!(text, "operation_count={}", outcome.operation_count);
    let _ = writeln!(text, "device_path={}", outcome.device_path);
    let _ = writeln!(text, "requested_bytes={}", outcome.requested_bytes);
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
            "\"proof_seam\":\"{}\",",
            "\"consumer_package\":\"{}\",",
            "\"binding_package\":\"{}\",",
            "\"composition\":\"{}\",",
            "\"operation_count\":{},",
            "\"device_path\":\"{}\",",
            "\"requested_bytes\":{},",
            "\"phase\":\"{}\",",
            "\"error_kind\":{},",
            "\"lifecycle_failure_kind\":{},",
            "\"worker_failure_kind\":{},",
            "\"validation_phase\":{},",
            "\"validation_error_kind\":{},",
            "\"message\":\"{}\"",
            "}}"
        ),
        outcome.ok,
        escape_json(outcome.proof_seam),
        escape_json(outcome.consumer_package),
        escape_json(outcome.binding_package),
        escape_json(outcome.composition),
        outcome.operation_count,
        escape_json(&outcome.device_path),
        outcome.requested_bytes,
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

fn deterministic_src(len: usize, stream: usize) -> Vec<u8> {
    (0..len)
        .map(|index| ((index * 31 + 17 + stream * 13) % 251) as u8)
        .collect()
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
        "Usage: downstream_async_handle [--device PATH] [--bytes N] [--format text|json] [--artifact PATH]"
    );
    println!(
        "Runs two joined cloned-handle async DSA memmoves from tonic-profile and prints stable proof metadata."
    );
}

struct SuccessWorker {
    device_path: PathBuf,
}

impl AsyncMemmoveWorker for SuccessWorker {
    fn memmove(
        &mut self,
        dst: &mut [u8],
        src: &[u8],
    ) -> Result<MemmoveValidationReport, MemmoveError> {
        dst[..src.len()].copy_from_slice(src);
        MemmoveValidationReport::new(&self.device_path, MemmoveRequest::new(src.len())?, 0, 1)
    }
}

struct CompletionTimeoutWorker {
    device_path: PathBuf,
}

impl AsyncMemmoveWorker for CompletionTimeoutWorker {
    fn memmove(
        &mut self,
        _dst: &mut [u8],
        _src: &[u8],
    ) -> Result<MemmoveValidationReport, MemmoveError> {
        Err(MemmoveError::CompletionTimeout {
            device_path: self.device_path.clone(),
            phase: MemmovePhase::CompletionPoll,
            page_fault_retries: 2,
        })
    }
}

struct PanicWorker;

impl AsyncMemmoveWorker for PanicWorker {
    fn memmove(
        &mut self,
        _dst: &mut [u8],
        _src: &[u8],
    ) -> Result<MemmoveValidationReport, MemmoveError> {
        panic!("downstream async-handle test worker panicked before replying");
    }
}
