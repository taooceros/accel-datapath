use clap::{Parser, ValueEnum};
use snafu::Snafu;
use std::num::ParseIntError;
use std::path::PathBuf;

pub(crate) const DEFAULT_SIZES: &str = "64,256,1024,4096,16384,65536,262144,1048576";
pub(crate) const DEFAULT_ITERATIONS: usize = 10_000;
pub(crate) const DEFAULT_MAX_CONCURRENCY: usize = 128;
pub(crate) const DEFAULT_SUBMIT_THREADS: usize = 1;

#[derive(Parser)]
#[command(
    name = "hw-eval",
    about = "Raw DSA/IAX hardware performance evaluation"
)]
pub(crate) struct Args {
    /// Accelerator backend to benchmark
    #[arg(long, value_enum, default_value = "dsa")]
    accel: AccelKind,

    /// WQ device path (default: /dev/dsa/wq0.0 for dsa, /dev/iax/wq1.0 for iax)
    #[arg(short, long)]
    device: Option<PathBuf>,

    /// Message sizes to test (bytes, comma-separated)
    #[arg(short, long, default_value = DEFAULT_SIZES)]
    sizes: String,

    /// Number of iterations per measurement
    #[arg(short, long, default_value_t = DEFAULT_ITERATIONS)]
    iterations: usize,

    /// Maximum concurrency for sliding window test
    #[arg(short, long, default_value_t = DEFAULT_MAX_CONCURRENCY)]
    max_concurrency: usize,

    /// Number of submitter threads for DSA direct memmove multi-thread throughput
    #[arg(long, default_value_t = DEFAULT_SUBMIT_THREADS)]
    threads: usize,

    /// Benchmark subset to run
    #[arg(long, value_enum, default_value = "all")]
    benchmark: BenchmarkKind,
    /// Submit-only workload variant to run when --benchmark submit-only is selected
    #[arg(long, value_enum, default_value = "all")]
    submit_mode: SubmitOnlyMode,

    /// Run software baselines only (no hardware required)
    #[arg(long)]
    sw_only: bool,

    /// Pin benchmark thread to this CPU core
    #[arg(long)]
    pin_core: Option<usize>,

    /// Flush caches between iterations (cold-cache measurement)
    #[arg(long)]
    cold: bool,

    /// Output results as JSON
    #[arg(long)]
    json: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum AccelKind {
    Dsa,
    Iax,
}

impl AccelKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Dsa => "dsa",
            Self::Iax => "iax",
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum BenchmarkKind {
    All,
    SubmitOnly,
}

impl BenchmarkKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::SubmitOnly => "submit-only",
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum SubmitOnlyMode {
    All,
    Unloaded,
    Sustained,
    Mfence,
}

pub(crate) fn default_device(accel: AccelKind) -> PathBuf {
    match accel {
        AccelKind::Dsa => PathBuf::from("/dev/dsa/wq0.0"),
        AccelKind::Iax => PathBuf::from("/dev/iax/wq1.0"),
    }
}

pub(crate) fn parse_sizes(s: &str) -> Result<Vec<usize>, BenchmarkConfigError> {
    let raw = s.to_string();
    let mut sizes = Vec::new();

    for token in s.split(',') {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            return Err(BenchmarkConfigError::EmptySizeToken { raw });
        }

        let size =
            trimmed
                .parse::<usize>()
                .map_err(|source| BenchmarkConfigError::InvalidSize {
                    raw: raw.clone(),
                    token: trimmed.to_string(),
                    source,
                })?;

        if size == 0 {
            return Err(BenchmarkConfigError::ZeroSize { raw });
        }

        sizes.push(size);
    }

    if sizes.is_empty() {
        return Err(BenchmarkConfigError::EmptySizes { raw });
    }

    Ok(sizes)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BenchmarkConfig {
    pub(crate) accel: AccelKind,
    pub(crate) device: PathBuf,
    pub(crate) sizes: Vec<usize>,
    pub(crate) iterations: usize,
    pub(crate) max_concurrency: usize,
    pub(crate) threads: usize,
    pub(crate) benchmark: BenchmarkKind,
    pub(crate) submit_mode: SubmitOnlyMode,

    pub(crate) sw_only: bool,
    pub(crate) pin_core: Option<usize>,
    pub(crate) cold: bool,
    pub(crate) json: bool,
}

#[bon::bon]
impl BenchmarkConfig {
    /// Build normalized benchmark runtime state from already-parsed CLI values.
    ///
    /// Clap remains the external parser. The method builder is kept for sparse,
    /// named construction in tests and diagnostics, while production CLI flow
    /// enters through `from_args`. This constructor resolves defaults that
    /// depend on other fields and validates the comma-separated size list before
    /// any benchmark loop or hardware queue-open path runs.
    #[builder(start_fn = builder, finish_fn = build)]
    pub(crate) fn from_parts(
        #[builder(default = AccelKind::Dsa)] accel: AccelKind,
        device: Option<PathBuf>,
        #[builder(default = DEFAULT_SIZES.to_string(), into)] sizes: String,
        #[builder(default = DEFAULT_ITERATIONS)] iterations: usize,
        #[builder(default = DEFAULT_MAX_CONCURRENCY)] max_concurrency: usize,
        #[builder(default = DEFAULT_SUBMIT_THREADS)] threads: usize,
        #[builder(default = BenchmarkKind::All)] benchmark: BenchmarkKind,
        #[builder(default = SubmitOnlyMode::All)] submit_mode: SubmitOnlyMode,
        #[builder(default)] sw_only: bool,
        pin_core: Option<usize>,
        #[builder(default)] cold: bool,
        #[builder(default)] json: bool,
    ) -> Result<Self, BenchmarkConfigError> {
        let device = device.unwrap_or_else(|| default_device(accel));
        let sizes = parse_sizes(&sizes)?;
        if threads == 0 {
            return Err(BenchmarkConfigError::ZeroThreads);
        }
        if benchmark == BenchmarkKind::SubmitOnly && accel != AccelKind::Dsa {
            return Err(BenchmarkConfigError::UnsupportedBenchmark {
                benchmark: benchmark.as_str(),
                accel: accel.as_str(),
            });
        }

        Ok(Self {
            accel,
            device,
            sizes,
            iterations,
            max_concurrency,
            threads,
            benchmark,
            submit_mode,
            sw_only,
            pin_core,
            cold,
            json,
        })
    }

    pub(crate) fn from_args(args: Args) -> Result<Self, BenchmarkConfigError> {
        let Args {
            accel,
            device,
            sizes,
            iterations,
            max_concurrency,
            threads,
            benchmark,
            submit_mode,
            sw_only,
            pin_core,
            cold,
            json,
        } = args;

        Self::from_parts(
            accel,
            device,
            sizes,
            iterations,
            max_concurrency,
            threads,
            benchmark,
            submit_mode,
            sw_only,
            pin_core,
            cold,
            json,
        )
    }
}

#[derive(Debug, Snafu)]
pub(crate) enum BenchmarkConfigError {
    #[snafu(display("--sizes must contain at least one size (got {raw:?})"))]
    EmptySizes { raw: String },
    #[snafu(display("--sizes must not contain empty entries (got {raw:?})"))]
    EmptySizeToken { raw: String },
    #[snafu(display("invalid --sizes entry {token:?} in {raw:?}; expected positive byte counts"))]
    InvalidSize {
        raw: String,
        token: String,
        source: ParseIntError,
    },
    #[snafu(display(
        "--sizes entries must be positive byte counts greater than zero (got {raw:?})"
    ))]
    ZeroSize { raw: String },
    #[snafu(display("--threads must be greater than zero"))]
    ZeroThreads,
    #[snafu(display("--benchmark {benchmark} is not supported for --accel {accel}"))]
    UnsupportedBenchmark {
        benchmark: &'static str,
        accel: &'static str,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn benchmark_config_builder_uses_dsa_defaults() {
        let config = BenchmarkConfig::builder().build().unwrap();

        assert_eq!(config.accel, AccelKind::Dsa);
        assert_eq!(config.device, PathBuf::from("/dev/dsa/wq0.0"));
        assert_eq!(
            config.sizes,
            vec![64, 256, 1024, 4096, 16384, 65536, 262144, 1048576]
        );
        assert_eq!(config.iterations, DEFAULT_ITERATIONS);
        assert_eq!(config.max_concurrency, DEFAULT_MAX_CONCURRENCY);
        assert_eq!(config.benchmark, BenchmarkKind::All);
        assert_eq!(config.submit_mode, SubmitOnlyMode::All);
        assert_eq!(config.threads, DEFAULT_SUBMIT_THREADS);
        assert!(!config.sw_only);
        assert_eq!(config.pin_core, None);
        assert!(!config.cold);
        assert!(!config.json);
    }

    #[test]
    fn benchmark_config_builder_uses_iax_default_device_when_device_omitted() {
        let config = BenchmarkConfig::builder()
            .accel(AccelKind::Iax)
            .build()
            .unwrap();

        assert_eq!(config.accel, AccelKind::Iax);
        assert_eq!(config.device, PathBuf::from("/dev/iax/wq1.0"));
    }

    #[test]
    fn benchmark_config_builder_preserves_explicit_device_and_runtime_knobs() {
        let config = BenchmarkConfig::builder()
            .accel(AccelKind::Dsa)
            .device(PathBuf::from("/tmp/custom-wq"))
            .sizes("64, 128,256".to_string())
            .iterations(7)
            .max_concurrency(4)
            .threads(5)
            .benchmark(BenchmarkKind::SubmitOnly)
            .submit_mode(SubmitOnlyMode::Mfence)
            .sw_only(true)
            .pin_core(3)
            .cold(true)
            .json(true)
            .build()
            .unwrap();

        assert_eq!(config.device, PathBuf::from("/tmp/custom-wq"));
        assert_eq!(config.sizes, vec![64, 128, 256]);
        assert_eq!(config.iterations, 7);
        assert_eq!(config.max_concurrency, 4);
        assert_eq!(config.threads, 5);
        assert_eq!(config.benchmark, BenchmarkKind::SubmitOnly);
        assert_eq!(config.submit_mode, SubmitOnlyMode::Mfence);
        assert!(config.sw_only);
        assert_eq!(config.pin_core, Some(3));
        assert!(config.cold);
        assert!(config.json);
    }

    #[test]
    fn parse_sizes_rejects_malformed_tokens_without_panicking() {
        let error = parse_sizes("64,abc,128").unwrap_err();

        match &error {
            BenchmarkConfigError::InvalidSize { raw, token, source } => {
                assert_eq!(raw, "64,abc,128");
                assert_eq!(token, "abc");
                assert_eq!(source.to_string(), "invalid digit found in string");
            }
            other => panic!("unexpected error: {other:?}"),
        }

        let display = error.to_string();
        assert!(
            display.contains("abc") && display.contains("64,abc,128"),
            "invalid size display should preserve token and raw input: {display}"
        );

        let source = std::error::Error::source(&error)
            .expect("invalid numeric tokens should preserve ParseIntError as source");
        assert_eq!(source.to_string(), "invalid digit found in string");
    }

    #[test]
    fn parse_sizes_rejects_empty_entries_and_zero_sizes() {
        assert!(matches!(
            parse_sizes("64,,128"),
            Err(BenchmarkConfigError::EmptySizeToken { .. })
        ));
        assert!(matches!(
            parse_sizes("64,0,128"),
            Err(BenchmarkConfigError::ZeroSize { .. })
        ));
    }

    #[test]
    fn benchmark_config_rejects_zero_submit_threads() {
        let error = BenchmarkConfig::builder().threads(0).build().unwrap_err();
        assert!(matches!(error, BenchmarkConfigError::ZeroThreads));
        assert_eq!(error.to_string(), "--threads must be greater than zero");
    }

    #[test]
    fn submit_only_benchmark_is_dsa_only() {
        let error = BenchmarkConfig::builder()
            .accel(AccelKind::Iax)
            .benchmark(BenchmarkKind::SubmitOnly)
            .build()
            .unwrap_err();

        assert!(matches!(
            error,
            BenchmarkConfigError::UnsupportedBenchmark { .. }
        ));
        assert_eq!(
            error.to_string(),
            "--benchmark submit-only is not supported for --accel iax"
        );
    }
}
