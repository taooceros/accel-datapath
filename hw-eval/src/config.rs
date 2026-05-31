use clap::{Parser, ValueEnum};
use snafu::Snafu;
use std::num::ParseIntError;
use std::path::PathBuf;

pub(crate) const DEFAULT_SIZES: &str = "64,256,1024,4096,16384,65536,262144,1048576";
pub(crate) const DEFAULT_ITERATIONS: usize = 10_000;
pub(crate) const DEFAULT_MAX_CONCURRENCY: usize = 128;
pub(crate) const DEFAULT_SUBMIT_THREADS: usize = 1;
pub(crate) const DEFAULT_SUBMIT_BURSTS: &str = "1,2,4,8,16,32,64,128,256,512";
pub(crate) const DEFAULT_SUBMIT_OCCUPANCIES: &str =
    "0,32,64,96,112,120,124,126,127,128,129,132,136,144,160";
pub(crate) const DEFAULT_MARKER_BURSTS: &str = "64,96,128,160,256";
pub(crate) const DEFAULT_MARKER_POSITIONS: &str = "first,half,last";
pub(crate) const DEFAULT_MARKER_POLL_CADENCES: &str = "1,4,16,64,never";
pub(crate) const DEFAULT_MARKER_POLL_OFFSETS: &str = "96";
pub(crate) const DEFAULT_TRAFFIC_WINDOWS: &str = "1,8,32,64,96,112,120,124,128,160,256";
pub(crate) const DEFAULT_TRAFFIC_CLASSES: &str = "submit-only,noop-completion,memmove64,memmove4k";
pub(crate) const DEFAULT_COMPLETION_REUSE_POLICIES: &str =
    "packed-scan,padded-round-robin,poll-only,delayed-reset,batch-harvest";
pub(crate) const DEFAULT_COMPLETION_REUSE_WINDOW: usize = 128;

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
    /// Submit burst sizes for submit-only benchmarks (comma-separated)
    #[arg(long, default_value = DEFAULT_SUBMIT_BURSTS)]
    submit_bursts: String,

    /// Prefill occupancies for submit-occupancy benchmarks (comma-separated, zero allowed)
    #[arg(long, default_value = DEFAULT_SUBMIT_OCCUPANCIES)]
    submit_occupancies: String,

    /// Burst lengths for submit-marker-overlap benchmarks (comma-separated)
    #[arg(long, default_value = DEFAULT_MARKER_BURSTS)]
    marker_bursts: String,

    /// Marker positions for submit-marker-overlap benchmarks: first,half,last (comma-separated)
    #[arg(long, default_value = DEFAULT_MARKER_POSITIONS)]
    marker_positions: String,

    /// Marker poll cadences for submit-marker-overlap benchmarks: integers or never (comma-separated)
    #[arg(long, default_value = DEFAULT_MARKER_POLL_CADENCES)]
    marker_poll_cadences: String,

    /// First zero-based submit indexes for submit-marker-overlap tracing; poll step is fixed at 1
    #[arg(long, default_value = DEFAULT_MARKER_POLL_OFFSETS)]
    marker_poll_offsets: String,

    /// Windows for traffic-class-ladder benchmarks (comma-separated)
    #[arg(long, default_value = DEFAULT_TRAFFIC_WINDOWS)]
    traffic_windows: String,

    /// Traffic classes for traffic-class-ladder benchmarks (comma-separated)
    #[arg(long, default_value = DEFAULT_TRAFFIC_CLASSES)]
    traffic_classes: String,

    /// Completion reuse policies for completion-reuse-policy benchmarks (comma-separated)
    #[arg(long, default_value = DEFAULT_COMPLETION_REUSE_POLICIES)]
    completion_reuse_policies: String,

    /// Fixed window for completion-reuse-policy benchmarks
    #[arg(long, default_value_t = DEFAULT_COMPLETION_REUSE_WINDOW)]
    completion_reuse_window: usize,

    /// DSA operation class for bottleneck experiments
    #[arg(long, value_enum, default_value = "noop")]
    dsa_op: DsaOperationClass,

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
    SubmitAdmission,
    SubmitOccupancy,
    SubmitMarkerOverlap,
    SubmitMarkerMechanism,
    TrafficClassLadder,
    CompletionReusePolicy,
}

impl BenchmarkKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::SubmitOnly => "submit-only",
            Self::SubmitAdmission => "submit-admission",
            Self::SubmitOccupancy => "submit-occupancy",
            Self::SubmitMarkerOverlap => "submit-marker-overlap",
            Self::SubmitMarkerMechanism => "submit-marker-mechanism",
            Self::TrafficClassLadder => "traffic-class-ladder",
            Self::CompletionReusePolicy => "completion-reuse-policy",
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

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum DsaOperationClass {
    Noop,
    Memmove64,
    Memmove4k,
}

impl DsaOperationClass {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Noop => "noop",
            Self::Memmove64 => "memmove64",
            Self::Memmove4k => "memmove4k",
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum MarkerPosition {
    First,
    Half,
    Last,
}

impl MarkerPosition {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::First => "first",
            Self::Half => "half",
            Self::Last => "last",
        }
    }

    pub(crate) fn to_index(self, n: usize) -> usize {
        match self {
            Self::First => 0,
            Self::Half => n / 2,
            Self::Last => n.saturating_sub(1),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum MarkerPollCadence {
    Every(usize),
    Never,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum TrafficClass {
    SubmitOnly,
    NoopCompletion,
    Memmove64,
    Memmove4k,
}

impl TrafficClass {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::SubmitOnly => "submit-only",
            Self::NoopCompletion => "noop-completion",
            Self::Memmove64 => "memmove64",
            Self::Memmove4k => "memmove4k",
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum CompletionReusePolicy {
    PackedScan,
    PaddedRoundRobin,
    PollOnly,
    DelayedReset,
    BatchHarvest,
}

impl CompletionReusePolicy {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::PackedScan => "packed-scan",
            Self::PaddedRoundRobin => "padded-round-robin",
            Self::PollOnly => "poll-only",
            Self::DelayedReset => "delayed-reset",
            Self::BatchHarvest => "batch-harvest",
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
    parse_positive_usize_list(s, "--sizes")
}

pub(crate) fn parse_submit_bursts(s: &str) -> Result<Vec<usize>, BenchmarkConfigError> {
    parse_positive_usize_list(s, "--submit-bursts")
}

pub(crate) fn parse_submit_occupancies(s: &str) -> Result<Vec<usize>, BenchmarkConfigError> {
    parse_usize_list(s, "--submit-occupancies", false)
}

pub(crate) fn parse_marker_bursts(s: &str) -> Result<Vec<usize>, BenchmarkConfigError> {
    parse_positive_usize_list(s, "--marker-bursts")
}

pub(crate) fn parse_marker_poll_offsets(s: &str) -> Result<Vec<usize>, BenchmarkConfigError> {
    parse_usize_list(s, "--marker-poll-offsets", false)
}

pub(crate) fn parse_marker_positions(s: &str) -> Result<Vec<MarkerPosition>, BenchmarkConfigError> {
    parse_value_enum_list(s, "--marker-positions")
}

pub(crate) fn parse_marker_poll_cadences(
    s: &str,
) -> Result<Vec<MarkerPollCadence>, BenchmarkConfigError> {
    let mut values = Vec::new();

    for token in s.split(',') {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            return Err(BenchmarkConfigError::EmptyListToken {
                flag: "--marker-poll-cadences",
                raw: s.to_owned(),
            });
        }

        if trimmed.eq_ignore_ascii_case("never") {
            values.push(MarkerPollCadence::Never);
            continue;
        }

        let value =
            trimmed
                .parse::<usize>()
                .map_err(|source| BenchmarkConfigError::InvalidListEntry {
                    flag: "--marker-poll-cadences",
                    raw: s.to_owned(),
                    token: trimmed.to_owned(),
                    source,
                })?;

        if value == 0 {
            return Err(BenchmarkConfigError::ZeroListEntry {
                flag: "--marker-poll-cadences",
                raw: s.to_owned(),
            });
        }

        values.push(MarkerPollCadence::Every(value));
    }

    if values.is_empty() {
        return Err(BenchmarkConfigError::EmptyList {
            flag: "--marker-poll-cadences",
            raw: s.to_owned(),
        });
    }

    Ok(values)
}

pub(crate) fn parse_traffic_windows(s: &str) -> Result<Vec<usize>, BenchmarkConfigError> {
    parse_positive_usize_list(s, "--traffic-windows")
}

pub(crate) fn parse_traffic_classes(s: &str) -> Result<Vec<TrafficClass>, BenchmarkConfigError> {
    parse_value_enum_list(s, "--traffic-classes")
}

pub(crate) fn parse_completion_reuse_policies(
    s: &str,
) -> Result<Vec<CompletionReusePolicy>, BenchmarkConfigError> {
    parse_value_enum_list(s, "--completion-reuse-policies")
}

fn parse_positive_usize_list(
    s: &str,
    flag: &'static str,
) -> Result<Vec<usize>, BenchmarkConfigError> {
    parse_usize_list(s, flag, true)
}

fn parse_usize_list(
    s: &str,
    flag: &'static str,
    require_positive: bool,
) -> Result<Vec<usize>, BenchmarkConfigError> {
    let mut values = Vec::new();

    for token in s.split(',') {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            return Err(BenchmarkConfigError::EmptyListToken {
                flag,
                raw: s.to_owned(),
            });
        }

        let value =
            trimmed
                .parse::<usize>()
                .map_err(|source| BenchmarkConfigError::InvalidListEntry {
                    flag,
                    raw: s.to_owned(),
                    token: trimmed.to_owned(),
                    source,
                })?;

        if require_positive && value == 0 {
            return Err(BenchmarkConfigError::ZeroListEntry {
                flag,
                raw: s.to_owned(),
            });
        }

        values.push(value);
    }

    if values.is_empty() {
        return Err(BenchmarkConfigError::EmptyList {
            flag,
            raw: s.to_owned(),
        });
    }

    Ok(values)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubmissionBottleneckConfig {
    pub(crate) submit_occupancies: Vec<usize>,
    pub(crate) marker_bursts: Vec<usize>,
    pub(crate) marker_positions: Vec<MarkerPosition>,
    pub(crate) marker_poll_cadences: Vec<MarkerPollCadence>,
    pub(crate) marker_poll_offsets: Vec<usize>,
    pub(crate) traffic_windows: Vec<usize>,
    pub(crate) traffic_classes: Vec<TrafficClass>,
    pub(crate) completion_reuse_policies: Vec<CompletionReusePolicy>,
    pub(crate) completion_reuse_window: usize,
    pub(crate) dsa_operation: DsaOperationClass,
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

    pub(crate) submit_bursts: Vec<usize>,
    pub(crate) submission_bottleneck: SubmissionBottleneckConfig,
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
        #[builder(default = DEFAULT_SUBMIT_BURSTS.to_string(), into)] submit_bursts: String,
        #[builder(default = DEFAULT_SUBMIT_OCCUPANCIES.to_string(), into)]
        submit_occupancies: String,
        #[builder(default = DEFAULT_MARKER_BURSTS.to_string(), into)] marker_bursts: String,
        #[builder(default = DEFAULT_MARKER_POSITIONS.to_string(), into)] marker_positions: String,
        #[builder(default = DEFAULT_MARKER_POLL_CADENCES.to_string(), into)]
        marker_poll_cadences: String,
        #[builder(default = DEFAULT_MARKER_POLL_OFFSETS.to_string(), into)]
        marker_poll_offsets: String,
        #[builder(default = DEFAULT_TRAFFIC_WINDOWS.to_string(), into)] traffic_windows: String,
        #[builder(default = DEFAULT_TRAFFIC_CLASSES.to_string(), into)] traffic_classes: String,
        #[builder(default = DEFAULT_COMPLETION_REUSE_POLICIES.to_string(), into)]
        completion_reuse_policies: String,
        #[builder(default = DEFAULT_COMPLETION_REUSE_WINDOW)] completion_reuse_window: usize,
        #[builder(default)] sw_only: bool,
        #[builder(default = DsaOperationClass::Noop)] dsa_op: DsaOperationClass,
        pin_core: Option<usize>,
        #[builder(default)] cold: bool,
        #[builder(default)] json: bool,
    ) -> Result<Self, BenchmarkConfigError> {
        let device = device.unwrap_or_else(|| default_device(accel));
        let sizes = parse_sizes(&sizes)?;
        let submit_bursts = parse_submit_bursts(&submit_bursts)?;
        let submission_bottleneck = SubmissionBottleneckConfig {
            submit_occupancies: parse_submit_occupancies(&submit_occupancies)?,
            marker_bursts: parse_marker_bursts(&marker_bursts)?,
            marker_positions: parse_marker_positions(&marker_positions)?,
            marker_poll_cadences: parse_marker_poll_cadences(&marker_poll_cadences)?,
            marker_poll_offsets: parse_marker_poll_offsets(&marker_poll_offsets)?,
            traffic_windows: parse_traffic_windows(&traffic_windows)?,
            traffic_classes: parse_traffic_classes(&traffic_classes)?,
            completion_reuse_policies: parse_completion_reuse_policies(&completion_reuse_policies)?,
            completion_reuse_window,
            dsa_operation: dsa_op,
        };
        if threads == 0 {
            return Err(BenchmarkConfigError::ZeroThreads);
        }
        if matches!(
            benchmark,
            BenchmarkKind::SubmitOnly
                | BenchmarkKind::SubmitAdmission
                | BenchmarkKind::SubmitOccupancy
                | BenchmarkKind::SubmitMarkerOverlap
                | BenchmarkKind::SubmitMarkerMechanism
                | BenchmarkKind::TrafficClassLadder
                | BenchmarkKind::CompletionReusePolicy,
        ) && accel != AccelKind::Dsa
        {
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
            submit_bursts,
            submission_bottleneck,
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
            submit_bursts,
            sw_only,
            submit_occupancies,
            marker_bursts,
            marker_positions,
            marker_poll_cadences,
            traffic_windows,
            marker_poll_offsets,
            traffic_classes,
            completion_reuse_policies,
            completion_reuse_window,
            pin_core,
            dsa_op,
            cold,
            json,
        } = args;

        Self::builder()
            .accel(accel)
            .maybe_device(device)
            .sizes(sizes)
            .iterations(iterations)
            .max_concurrency(max_concurrency)
            .threads(threads)
            .benchmark(benchmark)
            .submit_mode(submit_mode)
            .submit_bursts(submit_bursts)
            .submit_occupancies(submit_occupancies)
            .marker_bursts(marker_bursts)
            .marker_positions(marker_positions)
            .marker_poll_cadences(marker_poll_cadences)
            .marker_poll_offsets(marker_poll_offsets)
            .traffic_windows(traffic_windows)
            .traffic_classes(traffic_classes)
            .completion_reuse_policies(completion_reuse_policies)
            .completion_reuse_window(completion_reuse_window)
            .sw_only(sw_only)
            .dsa_op(dsa_op)
            .maybe_pin_core(pin_core)
            .cold(cold)
            .json(json)
            .build()
    }
}

#[derive(Debug, Snafu)]
pub(crate) enum BenchmarkConfigError {
    #[snafu(display("{flag} must contain at least one integer (got {raw:?})"))]
    EmptyList { flag: &'static str, raw: String },
    #[snafu(display("{flag} must not contain empty entries (got {raw:?})"))]
    EmptyListToken { flag: &'static str, raw: String },
    #[snafu(display("invalid {flag} entry {token:?} in {raw:?}; expected integers"))]
    InvalidListEntry {
        flag: &'static str,
        raw: String,
        token: String,
        source: ParseIntError,
    },
    #[snafu(display("{flag} entries must be positive integers greater than zero (got {raw:?})"))]
    ZeroListEntry { flag: &'static str, raw: String },
    #[snafu(display("--threads must be greater than zero"))]
    ZeroThreads,
    #[snafu(display("--benchmark {benchmark} is not supported for --accel {accel}"))]
    UnsupportedBenchmark {
        benchmark: &'static str,
        accel: &'static str,
    },
    #[snafu(display("invalid {flag} entry {token:?} in {raw:?}"))]
    InvalidEnumEntry {
        flag: &'static str,
        raw: String,
        token: String,
    },
}

fn parse_value_enum_list<T>(s: &str, flag: &'static str) -> Result<Vec<T>, BenchmarkConfigError>
where
    T: ValueEnum,
{
    let mut values = Vec::new();

    for token in s.split(',') {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            return Err(BenchmarkConfigError::EmptyListToken {
                flag,
                raw: s.to_owned(),
            });
        }

        let value =
            T::from_str(trimmed, true).map_err(|_| BenchmarkConfigError::InvalidEnumEntry {
                flag,
                raw: s.to_owned(),
                token: trimmed.to_owned(),
            })?;
        values.push(value);
    }

    if values.is_empty() {
        return Err(BenchmarkConfigError::EmptyList {
            flag,
            raw: s.to_owned(),
        });
    }

    Ok(values)
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
        assert_eq!(
            config.submit_bursts,
            vec![1, 2, 4, 8, 16, 32, 64, 128, 256, 512]
        );
        assert_eq!(
            config.submission_bottleneck.submit_occupancies,
            vec![0, 32, 64, 96, 112, 120, 124, 126, 127, 128, 129, 132, 136, 144, 160]
        );
        assert_eq!(
            config.submission_bottleneck.marker_bursts,
            vec![64, 96, 128, 160, 256]
        );
        assert_eq!(
            config.submission_bottleneck.marker_positions,
            vec![
                MarkerPosition::First,
                MarkerPosition::Half,
                MarkerPosition::Last
            ]
        );
        assert_eq!(
            config.submission_bottleneck.marker_poll_cadences,
            vec![
                MarkerPollCadence::Every(1),
                MarkerPollCadence::Every(4),
                MarkerPollCadence::Every(16),
                MarkerPollCadence::Every(64),
                MarkerPollCadence::Never,
            ]
        );
        assert_eq!(config.submission_bottleneck.marker_poll_offsets, vec![96]);
        assert_eq!(
            config.submission_bottleneck.traffic_windows,
            vec![1, 8, 32, 64, 96, 112, 120, 124, 128, 160, 256]
        );
        assert_eq!(
            config.submission_bottleneck.traffic_classes,
            vec![
                TrafficClass::SubmitOnly,
                TrafficClass::NoopCompletion,
                TrafficClass::Memmove64,
                TrafficClass::Memmove4k,
            ]
        );
        assert_eq!(
            config.submission_bottleneck.completion_reuse_policies,
            vec![
                CompletionReusePolicy::PackedScan,
                CompletionReusePolicy::PaddedRoundRobin,
                CompletionReusePolicy::PollOnly,
                CompletionReusePolicy::DelayedReset,
                CompletionReusePolicy::BatchHarvest,
            ]
        );
        assert_eq!(
            config.submission_bottleneck.completion_reuse_window,
            DEFAULT_COMPLETION_REUSE_WINDOW
        );
        assert_eq!(config.threads, DEFAULT_SUBMIT_THREADS);
        assert_eq!(
            config.submission_bottleneck.dsa_operation,
            DsaOperationClass::Noop
        );
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
            .submit_bursts("2, 64".to_string())
            .submit_occupancies("0, 32,128".to_string())
            .marker_bursts("64,160".to_string())
            .marker_positions("first,last".to_string())
            .marker_poll_cadences("4,never".to_string())
            .marker_poll_offsets("96,112".to_string())
            .traffic_windows("1,128".to_string())
            .traffic_classes("submit-only,memmove64".to_string())
            .completion_reuse_policies("packed-scan,batch-harvest".to_string())
            .completion_reuse_window(64)
            .sw_only(true)
            .dsa_op(DsaOperationClass::Memmove64)
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
        assert_eq!(config.submit_bursts, vec![2, 64]);
        assert_eq!(
            config.submission_bottleneck.submit_occupancies,
            vec![0, 32, 128]
        );
        assert_eq!(config.submission_bottleneck.marker_bursts, vec![64, 160]);
        assert_eq!(
            config.submission_bottleneck.marker_positions,
            vec![MarkerPosition::First, MarkerPosition::Last]
        );
        assert_eq!(
            config.submission_bottleneck.marker_poll_cadences,
            vec![MarkerPollCadence::Every(4), MarkerPollCadence::Never]
        );
        assert_eq!(
            config.submission_bottleneck.marker_poll_offsets,
            vec![96, 112]
        );
        assert_eq!(config.submission_bottleneck.traffic_windows, vec![1, 128]);
        assert_eq!(
            config.submission_bottleneck.traffic_classes,
            vec![TrafficClass::SubmitOnly, TrafficClass::Memmove64]
        );
        assert_eq!(
            config.submission_bottleneck.completion_reuse_policies,
            vec![
                CompletionReusePolicy::PackedScan,
                CompletionReusePolicy::BatchHarvest,
            ]
        );
        assert_eq!(config.submission_bottleneck.completion_reuse_window, 64);
        assert!(config.sw_only);
        assert_eq!(
            config.submission_bottleneck.dsa_operation,
            DsaOperationClass::Memmove64
        );
        assert_eq!(config.pin_core, Some(3));
        assert!(config.cold);
        assert!(config.json);

        let admission = BenchmarkConfig::builder()
            .benchmark(BenchmarkKind::SubmitAdmission)
            .submit_bursts("64,128".to_string())
            .build()
            .unwrap();
        assert_eq!(admission.benchmark, BenchmarkKind::SubmitAdmission);
        assert_eq!(admission.submit_bursts, vec![64, 128]);
    }

    #[test]
    fn parse_sizes_rejects_malformed_tokens_without_panicking() {
        let error = parse_sizes("64,abc,128").unwrap_err();

        match &error {
            BenchmarkConfigError::InvalidListEntry {
                flag,
                raw,
                token,
                source,
            } => {
                assert_eq!(*flag, "--sizes");
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
            Err(BenchmarkConfigError::EmptyListToken { .. })
        ));
        assert!(matches!(
            parse_sizes("64,0,128"),
            Err(BenchmarkConfigError::ZeroListEntry { .. })
        ));
    }

    #[test]
    fn parse_submit_bursts_rejects_malformed_tokens_without_panicking() {
        let error = parse_submit_bursts("1,nope,64").unwrap_err();

        match &error {
            BenchmarkConfigError::InvalidListEntry {
                flag,
                raw,
                token,
                source,
            } => {
                assert_eq!(*flag, "--submit-bursts");
                assert_eq!(raw, "1,nope,64");
                assert_eq!(token, "nope");
                assert_eq!(source.to_string(), "invalid digit found in string");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn parse_submit_occupancies_allows_zero_and_rejects_malformed_tokens() {
        assert_eq!(
            parse_submit_occupancies("0, 32,128").unwrap(),
            vec![0, 32, 128]
        );

        let error = parse_submit_occupancies("0,nope,128").unwrap_err();
        match &error {
            BenchmarkConfigError::InvalidListEntry {
                flag,
                raw,
                token,
                source,
            } => {
                assert_eq!(*flag, "--submit-occupancies");
                assert_eq!(raw, "0,nope,128");
                assert_eq!(token, "nope");
                assert_eq!(source.to_string(), "invalid digit found in string");
            }
            other => panic!("unexpected error: {other:?}"),
        }

        assert!(matches!(
            parse_submit_occupancies("0,,128"),
            Err(BenchmarkConfigError::EmptyListToken { .. })
        ));
    }

    #[test]
    fn parse_bottleneck_experiment_lists_validate_tokens() {
        assert_eq!(parse_marker_bursts("64, 128").unwrap(), vec![64, 128]);
        assert_eq!(
            parse_marker_positions("first, HALF,last").unwrap(),
            vec![
                MarkerPosition::First,
                MarkerPosition::Half,
                MarkerPosition::Last
            ]
        );
        assert_eq!(MarkerPosition::First.to_index(160), 0);
        assert_eq!(MarkerPosition::Half.to_index(160), 80);
        assert_eq!(MarkerPosition::Last.to_index(160), 159);
        assert_eq!(
            parse_marker_poll_cadences("1, 16, never").unwrap(),
            vec![
                MarkerPollCadence::Every(1),
                MarkerPollCadence::Every(16),
                MarkerPollCadence::Never,
            ]
        );
        assert_eq!(parse_traffic_windows("1,128").unwrap(), vec![1, 128]);
        assert_eq!(
            parse_marker_poll_offsets("0,96,112").unwrap(),
            vec![0, 96, 112]
        );
        assert_eq!(
            parse_traffic_classes("submit-only,memmove4k").unwrap(),
            vec![TrafficClass::SubmitOnly, TrafficClass::Memmove4k]
        );
        assert_eq!(
            parse_completion_reuse_policies("packed-scan,delayed-reset").unwrap(),
            vec![
                CompletionReusePolicy::PackedScan,
                CompletionReusePolicy::DelayedReset,
            ]
        );

        assert!(matches!(
            parse_marker_poll_cadences("0"),
            Err(BenchmarkConfigError::ZeroListEntry {
                flag: "--marker-poll-cadences",
                ..
            })
        ));
        assert!(matches!(
            parse_marker_positions("first,nope"),
            Err(BenchmarkConfigError::InvalidEnumEntry {
                flag: "--marker-positions",
                ..
            })
        ));
        assert!(matches!(
            parse_traffic_classes("submit-only,nope"),
            Err(BenchmarkConfigError::InvalidEnumEntry {
                flag: "--traffic-classes",
                ..
            })
        ));
        assert!(matches!(
            parse_completion_reuse_policies("packed-scan,nope"),
            Err(BenchmarkConfigError::InvalidEnumEntry {
                flag: "--completion-reuse-policies",
                ..
            })
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

    #[test]
    fn submit_admission_benchmark_is_dsa_only() {
        let error = BenchmarkConfig::builder()
            .accel(AccelKind::Iax)
            .benchmark(BenchmarkKind::SubmitAdmission)
            .build()
            .unwrap_err();

        assert!(matches!(
            error,
            BenchmarkConfigError::UnsupportedBenchmark { .. }
        ));
        assert_eq!(
            error.to_string(),
            "--benchmark submit-admission is not supported for --accel iax"
        );
    }

    #[test]
    fn submit_occupancy_benchmark_is_dsa_only() {
        let error = BenchmarkConfig::builder()
            .accel(AccelKind::Iax)
            .benchmark(BenchmarkKind::SubmitOccupancy)
            .build()
            .unwrap_err();

        assert!(matches!(
            error,
            BenchmarkConfigError::UnsupportedBenchmark { .. }
        ));
        assert_eq!(
            error.to_string(),
            "--benchmark submit-occupancy is not supported for --accel iax"
        );
    }

    #[test]
    fn remaining_bottleneck_experiment_benchmarks_are_dsa_only() {
        for (benchmark, name) in [
            (BenchmarkKind::SubmitMarkerOverlap, "submit-marker-overlap"),
            (
                BenchmarkKind::SubmitMarkerMechanism,
                "submit-marker-mechanism",
            ),
            (BenchmarkKind::TrafficClassLadder, "traffic-class-ladder"),
            (
                BenchmarkKind::CompletionReusePolicy,
                "completion-reuse-policy",
            ),
        ] {
            let error = BenchmarkConfig::builder()
                .accel(AccelKind::Iax)
                .benchmark(benchmark)
                .build()
                .unwrap_err();

            assert!(matches!(
                error,
                BenchmarkConfigError::UnsupportedBenchmark { .. }
            ));
            assert_eq!(
                error.to_string(),
                format!("--benchmark {name} is not supported for --accel iax")
            );
        }
    }
}
