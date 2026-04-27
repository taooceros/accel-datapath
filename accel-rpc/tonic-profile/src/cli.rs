use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Parser, ValueEnum};
use serde::Serialize;

use crate::custom_codec;
use crate::BoxError;

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum Mode {
    Server,
    Client,
    Selftest,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RpcMode {
    #[value(alias = "unary")]
    UnaryBytes,
    UnaryProtoShape,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PayloadKind {
    Random,
    Structured,
    Repeated,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CompressionMode {
    Off,
    On,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RuntimeMode {
    Single,
    Multi,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InstrumentationMode {
    Off,
    On,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BufferPolicy {
    Default,
    /// Pre-size codec buffers to a coarse payload bucket to reduce growth noise.
    Pooled,
    /// Pre-size codec buffers to the current payload frame size as a copy/buffer control.
    CopyMinimized,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ProtoShape {
    FleetSmall,
    FleetStringHeavy,
    FleetResponseHeavy,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum ResponseShape {
    Same,
    FleetSmall,
    FleetStringHeavy,
    FleetResponseHeavy,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AcceleratedPath {
    Software,
    Idxd,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AcceleratedLane {
    CodecMemmove,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AcceleratedDirection {
    Bidirectional,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AcceleratedPathConfig {
    pub(crate) selected_path: AcceleratedPath,
    pub(crate) device_path: Option<PathBuf>,
    pub(crate) lane: Option<AcceleratedLane>,
    pub(crate) direction: Option<AcceleratedDirection>,
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct Args {
    #[arg(long, value_enum)]
    pub(crate) mode: Mode,

    #[arg(long, value_enum, default_value = "unary-bytes")]
    pub(crate) rpc: RpcMode,

    #[arg(long, value_enum)]
    pub(crate) proto_shape: Option<ProtoShape>,

    #[arg(long, value_enum, default_value = "same")]
    pub(crate) response_shape: ResponseShape,

    #[arg(long, default_value = "127.0.0.1:50051")]
    pub(crate) bind: String,

    #[arg(long, default_value = "127.0.0.1:50051")]
    pub(crate) target: String,

    #[arg(long, default_value_t = 256)]
    pub(crate) payload_size: usize,

    #[arg(long, value_enum, default_value = "structured")]
    pub(crate) payload_kind: PayloadKind,

    #[arg(long, value_enum, default_value = "off")]
    pub(crate) compression: CompressionMode,

    #[arg(long, default_value_t = 1)]
    pub(crate) concurrency: usize,

    #[arg(long)]
    pub(crate) requests: Option<u64>,

    #[arg(long, default_value_t = 3000)]
    pub(crate) warmup_ms: u64,

    #[arg(long, default_value_t = 10000)]
    pub(crate) measure_ms: u64,

    #[arg(long, value_enum, default_value = "multi")]
    pub(crate) runtime: RuntimeMode,

    #[arg(long, value_enum, default_value = "on")]
    pub(crate) instrumentation: InstrumentationMode,

    #[arg(long, value_enum, default_value = "software")]
    pub(crate) accelerated_path: AcceleratedPath,

    #[arg(long)]
    pub(crate) accelerator_device: Option<PathBuf>,

    #[arg(long, value_enum)]
    pub(crate) accelerator_lane: Option<AcceleratedLane>,

    #[arg(long, value_enum, default_value = "default")]
    pub(crate) buffer_policy: BufferPolicy,

    #[arg(long)]
    pub(crate) server_core: Option<usize>,

    #[arg(long)]
    pub(crate) client_core: Option<usize>,

    #[arg(long)]
    pub(crate) json_out: Option<PathBuf>,

    #[arg(long)]
    pub(crate) server_json_out: Option<PathBuf>,

    #[arg(long)]
    pub(crate) run_id: Option<String>,

    #[arg(long)]
    pub(crate) shutdown_after_requests: Option<u64>,
}

pub(crate) fn validate_args(args: &Args) -> Result<(), BoxError> {
    match args.rpc {
        RpcMode::UnaryBytes => {
            if args.proto_shape.is_some() {
                return Err("--proto-shape is only supported with --rpc unary-proto-shape".into());
            }
            if args.response_shape != ResponseShape::Same {
                return Err(
                    "--response-shape is only supported with --rpc unary-proto-shape".into(),
                );
            }
        }
        RpcMode::UnaryProtoShape => {
            if args.proto_shape.is_none() {
                return Err("--proto-shape is required with --rpc unary-proto-shape".into());
            }
        }
    }
    if args.concurrency == 0 {
        return Err("--concurrency must be at least 1".into());
    }
    if args.server_json_out.is_some() && args.mode != Mode::Server {
        return Err("--server-json-out is only supported with --mode server".into());
    }
    if args.shutdown_after_requests.is_some()
        && args.mode == Mode::Server
        && args.server_json_out.is_none()
        && args.json_out.is_none()
    {
        return Err(
            "--shutdown-after-requests requires --server-json-out or --json-out in --mode server"
                .into(),
        );
    }
    resolve_accelerated_path_config(args)?;
    Ok(())
}

pub(crate) fn resolve_accelerated_path_config(
    args: &Args,
) -> Result<AcceleratedPathConfig, BoxError> {
    match args.accelerated_path {
        AcceleratedPath::Software => {
            if args.accelerator_device.is_some() {
                return Err(
                    "--accelerator-device is only supported with --accelerated-path idxd".into(),
                );
            }
            if args.accelerator_lane.is_some() {
                return Err(
                    "--accelerator-lane is only supported with --accelerated-path idxd".into(),
                );
            }
            Ok(AcceleratedPathConfig {
                selected_path: AcceleratedPath::Software,
                device_path: None,
                lane: None,
                direction: None,
            })
        }
        AcceleratedPath::Idxd => {
            let device_path = args.accelerator_device.clone().ok_or_else(|| {
                "--accelerator-device is required with --accelerated-path idxd".to_string()
            })?;
            Ok(AcceleratedPathConfig {
                selected_path: AcceleratedPath::Idxd,
                device_path: Some(device_path),
                lane: Some(args.accelerator_lane.unwrap_or(AcceleratedLane::CodecMemmove)),
                direction: Some(AcceleratedDirection::Bidirectional),
            })
        }
    }
}

pub(crate) fn effective_buffer_settings(
    args: &Args,
    workload_size: usize,
) -> (Option<usize>, Option<usize>) {
    const HEADER_SIZE: usize = tonic::codec::HEADER_SIZE;
    match args.buffer_policy {
        BufferPolicy::Default => (None, None),
        BufferPolicy::Pooled => {
            let frame = workload_size
                .saturating_add(HEADER_SIZE)
                .max(custom_codec::DEFAULT_CODEC_BUFFER_SIZE);
            let buffer = next_multiple(frame, custom_codec::DEFAULT_CODEC_BUFFER_SIZE);
            (
                Some(buffer),
                Some(buffer.max(custom_codec::DEFAULT_CODEC_YIELD_THRESHOLD)),
            )
        }
        BufferPolicy::CopyMinimized => {
            let buffer = workload_size.saturating_add(HEADER_SIZE).max(1);
            (
                Some(buffer),
                Some(buffer.max(custom_codec::DEFAULT_CODEC_YIELD_THRESHOLD)),
            )
        }
    }
}

pub(crate) fn next_multiple(value: usize, multiple: usize) -> usize {
    value.saturating_add(multiple.saturating_sub(1)) / multiple * multiple
}

pub(crate) fn resolve_run_id(args: &Args) -> String {
    args.run_id.clone().unwrap_or_else(|| {
        format!(
            "run-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time after unix epoch")
                .as_nanos()
        )
    })
}
