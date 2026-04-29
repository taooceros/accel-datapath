use std::path::PathBuf;

use idxd_rust::{DEFAULT_DEVICE_PATH, MemmoveRequest};

use crate::artifact::validate_artifact_path;

const MAX_BYTES: usize = 1 << 30;
const MAX_ITERATIONS: u64 = 1_000_000;
const MAX_CONCURRENCY: u32 = 4096;
const MAX_DURATION_MS: u64 = 60_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Backend {
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

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Hardware => "hardware",
            Self::Software => "software",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Suite {
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

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Canonical => "canonical",
            Self::Latency => "latency",
            Self::Concurrency => "concurrency",
            Self::Throughput => "throughput",
        }
    }

    pub(crate) fn modes(self) -> &'static [BenchmarkMode] {
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
pub(crate) enum OutputFormat {
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
pub(crate) enum BenchmarkMode {
    SingleLatency,
    ConcurrentSubmissions,
    FixedDurationThroughput,
}

impl BenchmarkMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::SingleLatency => "single_latency",
            Self::ConcurrentSubmissions => "concurrent_submissions",
            Self::FixedDurationThroughput => "fixed_duration_throughput",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CliArgs {
    pub(crate) device_path: PathBuf,
    pub(crate) backend: Backend,
    pub(crate) suite: Suite,
    pub(crate) bytes: usize,
    pub(crate) iterations: u64,
    pub(crate) concurrency: u32,
    pub(crate) duration_ms: u64,
    pub(crate) format: OutputFormat,
    pub(crate) artifact_path: Option<PathBuf>,
}

pub(crate) enum ParseOutcome {
    Help,
    Run(CliArgs),
}

impl CliArgs {
    pub(crate) fn parse<I>(mut args: I) -> Result<ParseOutcome, String>
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

pub(crate) fn print_help() {
    println!(
        "tokio_memmove_bench\n\nUSAGE:\n    tokio_memmove_bench [OPTIONS]\n\nOPTIONS:\n    --device <PATH>              DSA work queue path (default: {DEFAULT_DEVICE_PATH})\n    --backend <hardware|software>\n    --suite <canonical|latency|concurrency|throughput>\n    --bytes <N>                  Transfer size in bytes (1..={MAX_BYTES})\n    --iterations <N>             Iterations per latency/concurrency mode (1..={MAX_ITERATIONS})\n    --concurrency <N>            Concurrent submissions for concurrency/throughput modes (1..={MAX_CONCURRENCY})\n    --duration-ms <N>            Duration knob for throughput mode (1..={MAX_DURATION_MS})\n    --format <json|text>\n    --artifact <PATH>            Write exactly the emitted stdout artifact to this file\n    -h, --help                   Print help"
    );
}
