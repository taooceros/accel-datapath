use clap::{Parser, ValueEnum};
use snafu::Snafu;
use std::num::ParseIntError;
use std::path::PathBuf;

pub(crate) const DEFAULT_SIZES: &str = "64,256,1024,4096,16384,65536,262144,1048576";
pub(crate) const DEFAULT_ITERATIONS: usize = 10_000;
pub(crate) const DEFAULT_MAX_CONCURRENCY: usize = 128;

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
    pub(crate) sw_only: bool,
    pub(crate) pin_core: Option<usize>,
    pub(crate) cold: bool,
    pub(crate) json: bool,
}

#[bon::bon]
impl BenchmarkConfig {
    /// Build normalized benchmark runtime state from already-parsed CLI values.
    ///
    /// Clap remains the external parser. This internal builder only resolves
    /// defaults that depend on other fields and validates the comma-separated
    /// size list before any benchmark loop or hardware queue-open path runs.
    #[builder(start_fn = builder, finish_fn = build)]
    pub(crate) fn from_parts(
        #[builder(default = AccelKind::Dsa)] accel: AccelKind,
        device: Option<PathBuf>,
        #[builder(default = DEFAULT_SIZES.to_string(), into)] sizes: String,
        #[builder(default = DEFAULT_ITERATIONS)] iterations: usize,
        #[builder(default = DEFAULT_MAX_CONCURRENCY)] max_concurrency: usize,
        #[builder(default)] sw_only: bool,
        pin_core: Option<usize>,
        #[builder(default)] cold: bool,
        #[builder(default)] json: bool,
    ) -> Result<Self, BenchmarkConfigError> {
        let device = device.unwrap_or_else(|| default_device(accel));
        let sizes = parse_sizes(&sizes)?;

        Ok(Self {
            accel,
            device,
            sizes,
            iterations,
            max_concurrency,
            sw_only,
            pin_core,
            cold,
            json,
        })
    }

    pub(crate) fn from_args(args: Args) -> Result<Self, BenchmarkConfigError> {
        Self::builder()
            .accel(args.accel)
            .maybe_device(args.device)
            .sizes(args.sizes)
            .iterations(args.iterations)
            .max_concurrency(args.max_concurrency)
            .sw_only(args.sw_only)
            .maybe_pin_core(args.pin_core)
            .cold(args.cold)
            .json(args.json)
            .build()
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
    fn benchmark_config_preserves_explicit_device_and_runtime_knobs() {
        let config = BenchmarkConfig::from_parts(
            AccelKind::Iax,
            Some(PathBuf::from("/tmp/custom-wq")),
            "64, 128,256".to_string(),
            7,
            4,
            true,
            Some(3),
            true,
            true,
        )
        .unwrap();

        assert_eq!(config.device, PathBuf::from("/tmp/custom-wq"));
        assert_eq!(config.sizes, vec![64, 128, 256]);
        assert_eq!(config.iterations, 7);
        assert_eq!(config.max_concurrency, 4);
        assert!(config.sw_only);
        assert_eq!(config.pin_core, Some(3));
        assert!(config.cold);
        assert!(config.json);
    }

    #[test]
    fn parse_sizes_rejects_malformed_tokens_without_panicking() {
        let error = parse_sizes("64,abc,128").unwrap_err();

        match &error {
            BenchmarkConfigError::InvalidSize { raw, token, .. } => {
                assert_eq!(raw, "64,abc,128");
                assert_eq!(token, "abc");
            }
            other => panic!("unexpected error: {other:?}"),
        }
        assert!(
            std::error::Error::source(&error).is_some(),
            "invalid numeric tokens should preserve ParseIntError as source"
        );
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
}
