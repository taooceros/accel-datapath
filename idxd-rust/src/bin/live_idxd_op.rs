use std::env;
use std::fmt::Write as _;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use idxd_rust::{
    Dsa, Iax, IaxCrc64Error, IaxCrc64Report, IdxdSession, IdxdSessionError, MemmoveError,
    MemmoveValidationReport,
};
use idxd_sys::crc64_t10dif_field;

const DEFAULT_REQUESTED_BYTES: usize = 4096;
const MAX_REQUESTED_BYTES: usize = u32::MAX as usize;

fn main() -> ExitCode {
    match run() {
        Ok(exit) => exit,
        Err(err) => {
            let _ = writeln!(io::stderr(), "live_idxd_op: {err}");
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
            emit_report(&args, &report).map_err(|err| err.to_string())?;
            Ok(if report.ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operation {
    DsaMemmove,
    IaxCrc64,
}

impl Operation {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "dsa-memmove" => Ok(Self::DsaMemmove),
            "iax-crc64" => Ok(Self::IaxCrc64),
            other => Err(format!(
                "unsupported operation `{other}`; expected `dsa-memmove` or `iax-crc64`"
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::DsaMemmove => "dsa-memmove",
            Self::IaxCrc64 => "iax-crc64",
        }
    }

    fn accelerator(self) -> &'static str {
        match self {
            Self::DsaMemmove => "dsa",
            Self::IaxCrc64 => "iax",
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
    operation: Operation,
    device_path: PathBuf,
    requested_bytes: usize,
    format: OutputFormat,
    artifact_path: Option<PathBuf>,
}

impl CliArgs {
    fn parse<I>(mut args: I) -> Result<ParseOutcome, String>
    where
        I: Iterator<Item = String>,
    {
        let mut operation = None;
        let mut device_path = None;
        let mut requested_bytes = DEFAULT_REQUESTED_BYTES;
        let mut format = OutputFormat::Text;
        let mut artifact_path = None;

        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--op" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "missing value for `--op`".to_string())?;
                    operation = Some(Operation::parse(&value)?);
                }
                "--device" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "missing value for `--device`".to_string())?;
                    if value.is_empty() {
                        return Err("device path must not be empty".to_string());
                    }
                    device_path = Some(PathBuf::from(value));
                }
                "--bytes" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "missing value for `--bytes`".to_string())?;
                    requested_bytes = parse_requested_bytes(&value)?;
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
                "--help" | "-h" => return Ok(ParseOutcome::Help),
                other => {
                    return Err(format!(
                        "unsupported argument `{other}`; expected `--op`, `--device`, `--bytes`, `--format`, or `--artifact`"
                    ));
                }
            }
        }

        let operation = operation.ok_or_else(|| {
            "missing required `--op`; expected `dsa-memmove` or `iax-crc64`".to_string()
        })?;
        let device_path =
            device_path.ok_or_else(|| "missing required `--device` path".to_string())?;

        Ok(ParseOutcome::Run(Self {
            operation,
            device_path,
            requested_bytes,
            format,
            artifact_path,
        }))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OperationReport {
    ok: bool,
    operation: &'static str,
    accelerator: &'static str,
    device_path: String,
    requested_bytes: usize,
    page_fault_retries: Option<u32>,
    final_status: Option<u8>,
    phase: String,
    error_kind: Option<&'static str>,
    completion_error_code: Option<u8>,
    invalid_flags: Option<u32>,
    fault_addr: Option<u64>,
    crc64: Option<u64>,
    expected_crc64: Option<u64>,
    crc64_verified: Option<bool>,
    message: String,
}

fn execute(args: &CliArgs) -> OperationReport {
    match args.operation {
        Operation::DsaMemmove => execute_dsa_memmove(args),
        Operation::IaxCrc64 => execute_iax_crc64(args),
    }
}

fn execute_dsa_memmove(args: &CliArgs) -> OperationReport {
    let src = deterministic_src(args.requested_bytes);
    let mut dst = vec![0u8; args.requested_bytes];

    let session = match IdxdSession::<Dsa>::open(&args.device_path) {
        Ok(session) => session,
        Err(err) => return session_failure_report(args, err),
    };

    match session.memmove(&mut dst, &src) {
        Ok(report) => dsa_success_report(args.operation, report),
        Err(err) => dsa_failure_report(args, err),
    }
}

fn execute_iax_crc64(args: &CliArgs) -> OperationReport {
    let src = deterministic_src(args.requested_bytes);

    let session = match IdxdSession::<Iax>::open(&args.device_path) {
        Ok(session) => session,
        Err(err) => return session_failure_report(args, err),
    };

    match session.crc64(&src) {
        Ok(report) => iax_success_or_mismatch_report(args.operation, report, &src),
        Err(err) => iax_failure_report(args, err),
    }
}

fn dsa_success_report(operation: Operation, report: MemmoveValidationReport) -> OperationReport {
    OperationReport {
        ok: true,
        operation: operation.as_str(),
        accelerator: operation.accelerator(),
        device_path: report.device_path.display().to_string(),
        requested_bytes: report.requested_bytes,
        page_fault_retries: Some(report.page_fault_retries),
        final_status: Some(report.final_status),
        phase: "completed".to_string(),
        error_kind: None,
        completion_error_code: None,
        invalid_flags: None,
        fault_addr: None,
        crc64: None,
        expected_crc64: None,
        crc64_verified: None,
        message: format!(
            "verified {} copied bytes via IdxdSession<Dsa> memmove on {}",
            report.requested_bytes,
            report.device_path.display()
        ),
    }
}

fn iax_success_or_mismatch_report(
    operation: Operation,
    report: IaxCrc64Report,
    src: &[u8],
) -> OperationReport {
    let expected_crc64 = crc64_t10dif_field(src);
    let crc64_verified = report.crc64 == expected_crc64;

    OperationReport {
        ok: crc64_verified,
        operation: operation.as_str(),
        accelerator: operation.accelerator(),
        device_path: report.device_path.display().to_string(),
        requested_bytes: report.requested_bytes,
        page_fault_retries: Some(report.page_fault_retries),
        final_status: Some(report.final_status),
        phase: if crc64_verified {
            "completed".to_string()
        } else {
            "crc64_verify".to_string()
        },
        error_kind: if crc64_verified {
            None
        } else {
            Some("crc64_mismatch")
        },
        completion_error_code: None,
        invalid_flags: None,
        fault_addr: None,
        crc64: Some(report.crc64),
        expected_crc64: Some(expected_crc64),
        crc64_verified: Some(crc64_verified),
        message: if crc64_verified {
            format!(
                "verified crc64 result via IdxdSession<Iax> on {}",
                report.device_path.display()
            )
        } else {
            format!(
                "crc64 mismatch via IdxdSession<Iax> on {}: hardware={}, expected={}",
                report.device_path.display(),
                hex_u64(report.crc64),
                hex_u64(expected_crc64)
            )
        },
    }
}

fn session_failure_report(args: &CliArgs, err: IdxdSessionError) -> OperationReport {
    let phase = match err {
        IdxdSessionError::InvalidDevicePath { .. } => "argument_validation",
        IdxdSessionError::QueueOpen { .. } => "queue_open",
    };

    OperationReport {
        ok: false,
        operation: args.operation.as_str(),
        accelerator: err.accelerator_name(),
        device_path: err
            .device_path()
            .unwrap_or(args.device_path.as_path())
            .display()
            .to_string(),
        requested_bytes: args.requested_bytes,
        page_fault_retries: None,
        final_status: None,
        phase: phase.to_string(),
        error_kind: Some(err.kind()),
        completion_error_code: None,
        invalid_flags: None,
        fault_addr: None,
        crc64: None,
        expected_crc64: None,
        crc64_verified: None,
        message: err.to_string(),
    }
}

fn dsa_failure_report(args: &CliArgs, err: MemmoveError) -> OperationReport {
    OperationReport {
        ok: false,
        operation: args.operation.as_str(),
        accelerator: args.operation.accelerator(),
        device_path: err
            .device_path()
            .unwrap_or(args.device_path.as_path())
            .display()
            .to_string(),
        requested_bytes: err.requested_bytes().unwrap_or(args.requested_bytes),
        page_fault_retries: err.page_fault_retries(),
        final_status: err.final_status(),
        phase: err
            .phase()
            .map(|phase| phase.to_string())
            .unwrap_or_else(|| "argument_validation".to_string()),
        error_kind: Some(err.kind()),
        completion_error_code: None,
        invalid_flags: None,
        fault_addr: None,
        crc64: None,
        expected_crc64: None,
        crc64_verified: None,
        message: err.to_string(),
    }
}

fn iax_failure_report(args: &CliArgs, err: IaxCrc64Error) -> OperationReport {
    OperationReport {
        ok: false,
        operation: args.operation.as_str(),
        accelerator: args.operation.accelerator(),
        device_path: err
            .device_path()
            .unwrap_or(args.device_path.as_path())
            .display()
            .to_string(),
        requested_bytes: err.requested_bytes(),
        page_fault_retries: err.page_fault_retries(),
        final_status: err.final_status(),
        phase: err
            .phase()
            .map(|phase| phase.to_string())
            .unwrap_or_else(|| "argument_validation".to_string()),
        error_kind: Some(err.kind()),
        completion_error_code: err.error_code(),
        invalid_flags: err.invalid_flags(),
        fault_addr: err.fault_addr(),
        crc64: None,
        expected_crc64: None,
        crc64_verified: None,
        message: err.to_string(),
    }
}

fn emit_report(args: &CliArgs, report: &OperationReport) -> io::Result<()> {
    let rendered = match args.format {
        OutputFormat::Text => render_text(report),
        OutputFormat::Json => render_json(report),
    };

    if let Some(path) = &args.artifact_path {
        std::fs::write(path, rendered.as_bytes())?;
    }

    println!("{rendered}");
    Ok(())
}

fn render_text(report: &OperationReport) -> String {
    let mut text = String::new();
    let _ = writeln!(text, "ok={}", report.ok);
    let _ = writeln!(text, "operation={}", report.operation);
    let _ = writeln!(text, "accelerator={}", report.accelerator);
    let _ = writeln!(text, "device_path={}", report.device_path);
    let _ = writeln!(text, "requested_bytes={}", report.requested_bytes);
    let _ = writeln!(
        text,
        "page_fault_retries={}",
        opt_display(report.page_fault_retries)
    );
    let _ = writeln!(
        text,
        "final_status={}",
        report
            .final_status
            .map(hex_u8)
            .unwrap_or_else(|| "null".to_string())
    );
    let _ = writeln!(text, "phase={}", report.phase);
    let _ = writeln!(text, "error_kind={}", report.error_kind.unwrap_or("null"));
    let _ = writeln!(
        text,
        "completion_error_code={}",
        report
            .completion_error_code
            .map(hex_u8)
            .unwrap_or_else(|| "null".to_string())
    );
    let _ = writeln!(
        text,
        "invalid_flags={}",
        report
            .invalid_flags
            .map(hex_u32)
            .unwrap_or_else(|| "null".to_string())
    );
    let _ = writeln!(
        text,
        "fault_addr={}",
        report
            .fault_addr
            .map(hex_u64)
            .unwrap_or_else(|| "null".to_string())
    );
    let _ = writeln!(
        text,
        "crc64={}",
        report
            .crc64
            .map(hex_u64)
            .unwrap_or_else(|| "null".to_string())
    );
    let _ = writeln!(
        text,
        "expected_crc64={}",
        report
            .expected_crc64
            .map(hex_u64)
            .unwrap_or_else(|| "null".to_string())
    );
    let _ = writeln!(
        text,
        "crc64_verified={}",
        report
            .crc64_verified
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_string())
    );
    let _ = write!(text, "message={}", report.message);
    text
}

fn render_json(report: &OperationReport) -> String {
    format!(
        concat!(
            "{{",
            "\"ok\":{},",
            "\"operation\":\"{}\",",
            "\"accelerator\":\"{}\",",
            "\"device_path\":\"{}\",",
            "\"requested_bytes\":{},",
            "\"page_fault_retries\":{},",
            "\"final_status\":{},",
            "\"phase\":\"{}\",",
            "\"error_kind\":{},",
            "\"completion_error_code\":{},",
            "\"invalid_flags\":{},",
            "\"fault_addr\":{},",
            "\"crc64\":{},",
            "\"expected_crc64\":{},",
            "\"crc64_verified\":{},",
            "\"message\":\"{}\"",
            "}}"
        ),
        report.ok,
        escape_json(report.operation),
        escape_json(report.accelerator),
        escape_json(&report.device_path),
        report.requested_bytes,
        json_opt_u32(report.page_fault_retries),
        json_opt_hex_u8(report.final_status),
        escape_json(&report.phase),
        json_opt_str(report.error_kind),
        json_opt_hex_u8(report.completion_error_code),
        json_opt_hex_u32(report.invalid_flags),
        json_opt_hex_u64(report.fault_addr),
        json_opt_hex_u64(report.crc64),
        json_opt_hex_u64(report.expected_crc64),
        json_opt_bool(report.crc64_verified),
        escape_json(&report.message),
    )
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

fn opt_display<T: std::fmt::Display>(value: Option<T>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn json_opt_str(value: Option<&str>) -> String {
    value
        .map(|value| format!("\"{}\"", escape_json(value)))
        .unwrap_or_else(|| "null".to_string())
}

fn json_opt_bool(value: Option<bool>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn json_opt_u32(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn json_opt_hex_u8(value: Option<u8>) -> String {
    value
        .map(|value| format!("\"{}\"", hex_u8(value)))
        .unwrap_or_else(|| "null".to_string())
}

fn json_opt_hex_u32(value: Option<u32>) -> String {
    value
        .map(|value| format!("\"{}\"", hex_u32(value)))
        .unwrap_or_else(|| "null".to_string())
}

fn json_opt_hex_u64(value: Option<u64>) -> String {
    value
        .map(|value| format!("\"{}\"", hex_u64(value)))
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
        "Usage: live_idxd_op --op dsa-memmove|iax-crc64 --device PATH [--bytes N] [--format text|json] [--artifact PATH]"
    );
    println!(
        "Runs one real generic IdxdSession<Dsa> memmove or IdxdSession<Iax> crc64 proof and prints a no-payload report."
    );
}
