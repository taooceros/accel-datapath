use std::env;
use std::fmt::Write as _;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use dsa_ffi::{
    AsyncDsaSession, AsyncMemmoveError, MemmoveError, MemmoveRequest, MemmoveValidationReport,
    DEFAULT_DEVICE_PATH,
};

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
    worker_failure_kind: Option<&'static str>,
    validation_phase: Option<String>,
    validation_error_kind: Option<&'static str>,
    message: String,
}

async fn execute(args: &CliArgs) -> RunOutcome {
    let src = deterministic_src(args.requested_bytes);
    let request = match dsa_ffi::AsyncMemmoveRequest::new(src.clone()) {
        Ok(request) => request,
        Err(err) => return validation_failure_outcome(args, err),
    };

    let session = match AsyncDsaSession::open(&args.device_path) {
        Ok(session) => session,
        Err(err) => return async_failure_outcome(args, err),
    };

    let memmove_result = session.memmove(request).await;
    let shutdown_result = session.shutdown();

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
        worker_failure_kind: None,
        validation_phase: Some("completed".to_string()),
        validation_error_kind: None,
        message: format!(
            "verified {} copied bytes via async wrapper on {}",
            report.requested_bytes,
            report.device_path.display()
        ),
    }
}

fn async_failure_outcome(args: &CliArgs, err: AsyncMemmoveError) -> RunOutcome {
    match err {
        AsyncMemmoveError::Memmove(err) => validation_failure_outcome(args, err),
        AsyncMemmoveError::WorkerFailure { kind } => RunOutcome {
            ok: false,
            device_path: args.device_path.display().to_string(),
            requested_bytes: args.requested_bytes,
            page_fault_retries: None,
            final_status: None,
            phase: "async_worker".to_string(),
            error_kind: Some("worker_failure"),
            worker_failure_kind: Some(kind.as_str()),
            validation_phase: None,
            validation_error_kind: None,
            message: format!("async memmove worker failure: {}", kind.as_str()),
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
        worker_failure_kind: None,
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
            "\"device_path\":\"{}\",",
            "\"requested_bytes\":{},",
            "\"page_fault_retries\":{},",
            "\"final_status\":{},",
            "\"phase\":\"{}\",",
            "\"error_kind\":{},",
            "\"worker_failure_kind\":{},",
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

fn deterministic_src(len: usize) -> Vec<u8> {
    (0..len)
        .map(|index| ((index * 31 + 17) % 251) as u8)
        .collect()
}

fn hex_status(status: u8) -> String {
    format!("0x{status:02x}")
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
    println!("Runs one real DSA memmove through the async wrapper and prints a stable report.");
}
