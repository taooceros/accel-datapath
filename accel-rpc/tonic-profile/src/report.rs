use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::cli::{
    resolve_accelerated_path_config, AcceleratedDirection, AcceleratedLane, AcceleratedPath, Args,
    BufferPolicy, CompressionMode, InstrumentationMode, Mode, PayloadKind, ProtoShape, RpcMode,
    RuntimeMode,
};
use crate::custom_codec;
use crate::runtime_instrumentation;
use crate::service::ServerMetrics;
use crate::workload::{workload_probe, SelectionPolicy};
use crate::BoxError;

#[derive(Serialize)]
pub(crate) struct Metadata {
    timestamp_unix_s: u64,
    mode: &'static str,
    endpoint_role: &'static str,
    run_id: String,
    rpc: RpcMode,
    ordinary_path: &'static str,
    selected_path: AcceleratedPath,
    seam: &'static str,
    workload_label: String,
    selection_policy: SelectionPolicy,
    request_shape: Option<ProtoShape>,
    response_shape: Option<ProtoShape>,
    request_serialized_size: usize,
    response_serialized_size: usize,
    bind: String,
    target: String,
    payload_size: Option<usize>,
    payload_kind: Option<PayloadKind>,
    compression: CompressionMode,
    concurrency: usize,
    requests_target: Option<u64>,
    warmup_ms: u64,
    measure_ms: u64,
    runtime: RuntimeMode,
    instrumentation: InstrumentationMode,
    accelerated_device_path: Option<PathBuf>,
    accelerated_lane: Option<AcceleratedLane>,
    accelerated_direction: Option<AcceleratedDirection>,
    buffer_policy: BufferPolicy,
    effective_codec_buffer_size: Option<usize>,
    effective_codec_yield_threshold: Option<usize>,
    server_core: Option<usize>,
    client_core: Option<usize>,
}

#[derive(Serialize)]
pub(crate) struct Metrics {
    pub(crate) requests_completed: u64,
    pub(crate) bytes_sent: u64,
    pub(crate) bytes_received: u64,
    pub(crate) duration_ms: f64,
    pub(crate) throughput_rps: f64,
    pub(crate) throughput_mib_s: f64,
    pub(crate) latency_us_p50: u64,
    pub(crate) latency_us_p95: u64,
    pub(crate) latency_us_p99: u64,
    pub(crate) latency_us_max: u64,
}

#[derive(Serialize)]
pub(crate) struct Report {
    metadata: Metadata,
    metrics: Metrics,
    stages: StageSnapshot,
}

#[derive(Serialize, Default)]
struct StageSnapshot {
    enabled: bool,
    encode: StageCounter,
    decode: StageCounter,
    compress: StageCounter,
    decompress: StageCounter,
    buffer_reserve: StageCounter,
    body_accum: StageCounter,
    frame_header: StageCounter,
}

#[derive(Serialize, Default)]
struct StageCounter {
    count: u64,
    nanos: u64,
    millis: f64,
    bytes: u64,
    avg_nanos: f64,
}

pub(crate) fn server_report_path(args: &Args) -> Option<&PathBuf> {
    if args.mode == Mode::Server {
        args.server_json_out.as_ref().or(args.json_out.as_ref())
    } else {
        None
    }
}

pub(crate) fn build_client_report(
    args: &Args,
    endpoint_role: &'static str,
    run_id: &str,
    metrics: Metrics,
) -> Result<Report, BoxError> {
    build_report(
        args,
        "client",
        endpoint_role,
        run_id,
        metrics,
        StageSnapshot::from(runtime_instrumentation::snapshot()),
        custom_codec::observed_settings()
            .ok_or("custom codec observations were missing for this run")?,
    )
}

pub(crate) async fn build_server_report(
    args: &Args,
    server_metrics: &Arc<ServerMetrics>,
    run_id: &str,
) -> Result<Report, BoxError> {
    let metrics = server_metrics.snapshot().await;
    build_report(
        args,
        "server",
        "server",
        run_id,
        metrics,
        StageSnapshot::from(runtime_instrumentation::snapshot()),
        custom_codec::observed_settings()
            .ok_or("custom codec observations were missing for this server run")?,
    )
}

fn build_report(
    args: &Args,
    mode: &'static str,
    endpoint_role: &'static str,
    run_id: &str,
    metrics: Metrics,
    stages: StageSnapshot,
    observed_codec_settings: custom_codec::CodecObservation,
) -> Result<Report, BoxError> {
    let descriptor = workload_probe(args);
    let acceleration = resolve_accelerated_path_config(args)?;

    let report = Report {
        metadata: Metadata {
            timestamp_unix_s: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            mode,
            endpoint_role,
            run_id: run_id.to_string(),
            rpc: args.rpc,
            ordinary_path: "software",
            selected_path: acceleration.selected_path,
            seam: "codec_body",
            workload_label: descriptor.workload_label,
            selection_policy: descriptor.selection_policy,
            request_shape: descriptor.request_shape,
            response_shape: descriptor.response_shape,
            request_serialized_size: descriptor.request_serialized_size,
            response_serialized_size: descriptor.response_serialized_size,
            bind: args.bind.clone(),
            target: args.target.clone(),
            payload_size: descriptor.payload_size,
            payload_kind: descriptor.payload_kind,
            compression: args.compression,
            concurrency: args.concurrency,
            requests_target: args.requests,
            warmup_ms: args.warmup_ms,
            measure_ms: args.measure_ms,
            runtime: args.runtime,
            instrumentation: args.instrumentation,
            accelerated_device_path: acceleration.device_path,
            accelerated_lane: acceleration.lane,
            accelerated_direction: acceleration.direction,
            buffer_policy: args.buffer_policy,
            effective_codec_buffer_size: Some(observed_codec_settings.buffer_size),
            effective_codec_yield_threshold: Some(observed_codec_settings.yield_threshold),
            server_core: args.server_core,
            client_core: args.client_core,
        },
        metrics,
        stages,
    };
    validate_stage_evidence(&report)?;
    Ok(report)
}

fn stage_counters_are_placeholder_only(stages: &StageSnapshot) -> bool {
    [
        &stages.encode,
        &stages.decode,
        &stages.compress,
        &stages.decompress,
        &stages.buffer_reserve,
        &stages.body_accum,
        &stages.frame_header,
    ]
    .iter()
    .all(|counter| counter.count == 0 && counter.nanos == 0 && counter.bytes == 0)
}

fn validate_stage_evidence(report: &Report) -> Result<(), BoxError> {
    if report.metadata.instrumentation == InstrumentationMode::On
        && stage_counters_are_placeholder_only(&report.stages)
    {
        return Err(format!(
            "instrumentation-on report for workload {} endpoint {} stayed placeholder-only",
            report.metadata.workload_label, report.metadata.endpoint_role
        )
        .into());
    }
    Ok(())
}

pub(crate) fn emit_report(
    path: Option<&PathBuf>,
    mode: &'static str,
    mut report: Report,
) -> Result<(), BoxError> {
    report.metadata.mode = mode;
    validate_stage_evidence(&report)?;
    let json = serde_json::to_string_pretty(&report)?;
    if let Some(path) = path {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, &json)?;
    }
    println!("{}", json);
    Ok(())
}

impl From<runtime_instrumentation::Snapshot> for StageSnapshot {
    fn from(value: runtime_instrumentation::Snapshot) -> Self {
        Self {
            enabled: value.enabled,
            encode: value.encode.into(),
            decode: value.decode.into(),
            compress: value.compress.into(),
            decompress: value.decompress.into(),
            buffer_reserve: value.buffer_reserve.into(),
            body_accum: value.body_accum.into(),
            frame_header: value.frame_header.into(),
        }
    }
}

impl From<runtime_instrumentation::Counter> for StageCounter {
    fn from(value: runtime_instrumentation::Counter) -> Self {
        Self {
            count: value.count,
            nanos: value.nanos,
            millis: value.nanos as f64 / 1_000_000.0,
            bytes: value.bytes,
            avg_nanos: if value.count == 0 {
                0.0
            } else {
                value.nanos as f64 / value.count as f64
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{stage_counters_are_placeholder_only, StageCounter, StageSnapshot};

    #[test]
    fn placeholder_detection_rejects_real_stage_activity() {
        let mut snapshot = StageSnapshot::default();
        assert!(stage_counters_are_placeholder_only(&snapshot));

        snapshot.encode = StageCounter {
            count: 1,
            nanos: 42,
            millis: 0.000042,
            bytes: 16,
            avg_nanos: 42.0,
        };
        assert!(!stage_counters_are_placeholder_only(&snapshot));
    }
}
