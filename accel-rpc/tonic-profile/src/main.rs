use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use clap::{Parser, ValueEnum};
use hdrhistogram::Histogram;
use prost::Message;
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};
use serde::Serialize;
use tokio::runtime::Builder;
use tokio::sync::{Mutex, Notify};
use tonic::codec::CompressionEncoding;
use tonic::transport::{Channel, Endpoint, Server};
use tonic::{Request, Response, Status};

mod custom_codec;
mod runtime_instrumentation;

pub mod profile {
    tonic::include_proto!("tonicprofile");
}

use profile::echo_reply::Body as EchoReplyBody;
use profile::echo_request::Body as EchoRequestBody;
use profile::profile_client::ProfileClient;
use profile::profile_server::{Profile, ProfileServer};
use profile::{
    CompactProfileShape, EchoReply, EchoRequest, FleetResponseEntry, FleetResponseHeavyShape,
    FleetSmallShape, FleetStringHeavyShape, FleetStringLeaf, ShapeKind,
};

type BoxError = Box<dyn std::error::Error + Send + Sync>;
const REQUEST_POOL_LEN: usize = 4;

fn set_default_buffer_settings_for_process(
    buffer_size: Option<usize>,
    yield_threshold: Option<usize>,
) -> Result<custom_codec::EffectiveBufferSettings, BoxError> {
    custom_codec::set_process_default_buffer_settings(buffer_size, yield_threshold)
        .map_err(Into::into)
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum Mode {
    Server,
    Client,
    Selftest,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Serialize)]
#[serde(rename_all = "kebab-case")]
enum RpcMode {
    #[value(alias = "unary")]
    UnaryBytes,
    UnaryProtoShape,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
enum PayloadKind {
    Random,
    Structured,
    Repeated,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
enum CompressionMode {
    Off,
    On,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
enum RuntimeMode {
    Single,
    Multi,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
enum InstrumentationMode {
    Off,
    On,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
enum BufferPolicy {
    Default,
    /// Pre-size codec buffers to a coarse payload bucket to reduce growth noise.
    Pooled,
    /// Pre-size codec buffers to the current payload frame size as a copy/buffer control.
    CopyMinimized,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ProtoShape {
    FleetSmall,
    FleetStringHeavy,
    FleetResponseHeavy,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum ResponseShape {
    Same,
    FleetSmall,
    FleetStringHeavy,
    FleetResponseHeavy,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum SelectionPolicy {
    EchoPayload,
    SameAsRequest,
    ExplicitResponse,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
enum AcceleratedPath {
    Software,
    Idxd,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
enum AcceleratedLane {
    CodecMemmove,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum AcceleratedDirection {
    Bidirectional,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AcceleratedPathConfig {
    selected_path: AcceleratedPath,
    device_path: Option<PathBuf>,
    lane: Option<AcceleratedLane>,
    direction: Option<AcceleratedDirection>,
}

#[derive(Parser, Debug, Clone)]
struct Args {
    #[arg(long, value_enum)]
    mode: Mode,

    #[arg(long, value_enum, default_value = "unary-bytes")]
    rpc: RpcMode,

    #[arg(long, value_enum)]
    proto_shape: Option<ProtoShape>,

    #[arg(long, value_enum, default_value = "same")]
    response_shape: ResponseShape,

    #[arg(long, default_value = "127.0.0.1:50051")]
    bind: String,

    #[arg(long, default_value = "127.0.0.1:50051")]
    target: String,

    #[arg(long, default_value_t = 256)]
    payload_size: usize,

    #[arg(long, value_enum, default_value = "structured")]
    payload_kind: PayloadKind,

    #[arg(long, value_enum, default_value = "off")]
    compression: CompressionMode,

    #[arg(long, default_value_t = 1)]
    concurrency: usize,

    #[arg(long)]
    requests: Option<u64>,

    #[arg(long, default_value_t = 3000)]
    warmup_ms: u64,

    #[arg(long, default_value_t = 10000)]
    measure_ms: u64,

    #[arg(long, value_enum, default_value = "multi")]
    runtime: RuntimeMode,

    #[arg(long, value_enum, default_value = "on")]
    instrumentation: InstrumentationMode,

    #[arg(long, value_enum, default_value = "software")]
    accelerated_path: AcceleratedPath,

    #[arg(long)]
    accelerator_device: Option<PathBuf>,

    #[arg(long, value_enum)]
    accelerator_lane: Option<AcceleratedLane>,

    #[arg(long, value_enum, default_value = "default")]
    buffer_policy: BufferPolicy,

    #[arg(long)]
    server_core: Option<usize>,

    #[arg(long)]
    client_core: Option<usize>,

    #[arg(long)]
    json_out: Option<PathBuf>,

    #[arg(long)]
    server_json_out: Option<PathBuf>,

    #[arg(long)]
    run_id: Option<String>,

    #[arg(long)]
    shutdown_after_requests: Option<u64>,
}

#[derive(Default)]
struct SharedState {
    shutdown: Arc<Notify>,
}

struct ServerMetrics {
    requests_completed: AtomicU64,
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
    latency_hist: Mutex<Histogram<u64>>,
    started_at: Instant,
}

impl ServerMetrics {
    fn new() -> Result<Self, BoxError> {
        Ok(Self {
            requests_completed: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            latency_hist: Mutex::new(Histogram::<u64>::new(3)?),
            started_at: Instant::now(),
        })
    }

    async fn record_request(&self, request_bytes: usize, response_bytes: usize, latency_us: u64) {
        self.requests_completed.fetch_add(1, Ordering::Relaxed);
        self.bytes_received
            .fetch_add(request_bytes as u64, Ordering::Relaxed);
        self.bytes_sent
            .fetch_add(response_bytes as u64, Ordering::Relaxed);
        let mut hist = self.latency_hist.lock().await;
        let _ = hist.record(latency_us.max(1));
    }

    async fn snapshot(&self) -> Metrics {
        let duration = self.started_at.elapsed();
        let duration_s = duration.as_secs_f64().max(1e-9);
        let requests_completed = self.requests_completed.load(Ordering::Relaxed);
        let bytes_sent = self.bytes_sent.load(Ordering::Relaxed);
        let bytes_received = self.bytes_received.load(Ordering::Relaxed);
        let hist = self.latency_hist.lock().await;

        Metrics {
            requests_completed,
            bytes_sent,
            bytes_received,
            duration_ms: duration.as_secs_f64() * 1000.0,
            throughput_rps: requests_completed as f64 / duration_s,
            throughput_mib_s: bytes_sent as f64 / duration_s / (1024.0 * 1024.0),
            latency_us_p50: if requests_completed == 0 {
                0
            } else {
                hist.value_at_quantile(0.50)
            },
            latency_us_p95: if requests_completed == 0 {
                0
            } else {
                hist.value_at_quantile(0.95)
            },
            latency_us_p99: if requests_completed == 0 {
                0
            } else {
                hist.value_at_quantile(0.99)
            },
            latency_us_max: if requests_completed == 0 {
                0
            } else {
                hist.max()
            },
        }
    }
}

struct EchoSvc {
    compression: CompressionMode,
    proto_pools: Arc<ProtoPools>,
    next_proto_index: AtomicU64,
    shutdown: Arc<Notify>,
    shutdown_after_requests: Option<u64>,
    server_metrics: Arc<ServerMetrics>,
}

#[tonic::async_trait]
impl Profile for EchoSvc {
    async fn unary_echo(
        &self,
        request: Request<EchoRequest>,
    ) -> Result<Response<EchoReply>, Status> {
        let started = Instant::now();
        let request = request.into_inner();
        let request_bytes = request.encoded_len();
        let body = request
            .body
            .ok_or_else(|| Status::invalid_argument("missing request body"))?;

        let reply_body = match body {
            EchoRequestBody::Payload(payload) => EchoReplyBody::Payload(payload),
            EchoRequestBody::ProtoPayload(proto_payload) => {
                let request_shape = proto_shape_from_message(&proto_payload)
                    .ok_or_else(|| Status::invalid_argument("missing proto shape body"))?;
                let response_shape = proto_shape_from_wire(request.requested_response_shape)?
                    .unwrap_or(request_shape);
                let response = self.proto_pools.pick(
                    response_shape,
                    self.next_proto_index.fetch_add(1, Ordering::Relaxed),
                );
                EchoReplyBody::ProtoPayload(response)
            }
        };

        let response_message = EchoReply {
            body: Some(reply_body),
        };
        let response_bytes = response_message.encoded_len();
        let mut response = Response::new(response_message);
        if self.compression == CompressionMode::On {
            response
                .metadata_mut()
                .insert("x-tonic-profile", "gzip".parse().unwrap());
        }
        self.server_metrics
            .record_request(
                request_bytes,
                response_bytes,
                started.elapsed().as_micros() as u64,
            )
            .await;
        if let Some(target) = self.shutdown_after_requests {
            let completed = self
                .server_metrics
                .requests_completed
                .load(Ordering::Relaxed);
            if completed >= target {
                self.shutdown.notify_waiters();
            }
        }
        Ok(response)
    }
}

#[derive(Serialize)]
struct Metadata {
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
struct Metrics {
    requests_completed: u64,
    bytes_sent: u64,
    bytes_received: u64,
    duration_ms: f64,
    throughput_rps: f64,
    throughput_mib_s: f64,
    latency_us_p50: u64,
    latency_us_p95: u64,
    latency_us_p99: u64,
    latency_us_max: u64,
}

#[derive(Serialize)]
struct Report {
    metadata: Metadata,
    metrics: Metrics,
    stages: StageSnapshot,
}

#[derive(Serialize)]
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

#[derive(Serialize)]
struct StageCounter {
    count: u64,
    nanos: u64,
    millis: f64,
    bytes: u64,
    avg_nanos: f64,
}

#[derive(Clone)]
struct PreparedRequest {
    body: PreparedRequestBody,
    expected_response_shape: Option<ProtoShape>,
    request_serialized_size: usize,
    response_serialized_size: usize,
}

#[derive(Clone)]
enum PreparedRequestBody {
    Bytes(Vec<u8>),
    Proto(CompactProfileShape),
}

#[derive(Clone)]
struct WorkloadDescriptor {
    workload_label: String,
    selection_policy: SelectionPolicy,
    request_shape: Option<ProtoShape>,
    response_shape: Option<ProtoShape>,
    payload_size: Option<usize>,
    payload_kind: Option<PayloadKind>,
    request_serialized_size: usize,
    response_serialized_size: usize,
}

struct ProtoPools {
    fleet_small: Vec<CompactProfileShape>,
    fleet_string_heavy: Vec<CompactProfileShape>,
    fleet_response_heavy: Vec<CompactProfileShape>,
}

impl ProtoPools {
    fn new() -> Self {
        let fleet_small = (0..REQUEST_POOL_LEN)
            .map(|seed| build_proto_message(ProtoShape::FleetSmall, seed as u64))
            .collect();
        let fleet_string_heavy = (0..REQUEST_POOL_LEN)
            .map(|seed| build_proto_message(ProtoShape::FleetStringHeavy, seed as u64))
            .collect();
        let fleet_response_heavy = (0..REQUEST_POOL_LEN)
            .map(|seed| build_proto_message(ProtoShape::FleetResponseHeavy, seed as u64))
            .collect();
        Self {
            fleet_small,
            fleet_string_heavy,
            fleet_response_heavy,
        }
    }

    fn pick(&self, shape: ProtoShape, index: u64) -> CompactProfileShape {
        let pool = match shape {
            ProtoShape::FleetSmall => &self.fleet_small,
            ProtoShape::FleetStringHeavy => &self.fleet_string_heavy,
            ProtoShape::FleetResponseHeavy => &self.fleet_response_heavy,
        };
        pool[index as usize % pool.len()].clone()
    }
}

fn main() -> Result<(), BoxError> {
    let args = Args::parse();
    validate_args(&args)?;
    configure_process_controls(&args)?;
    let runtime = build_runtime(args.runtime)?;
    runtime.block_on(async_main(args))
}

fn validate_args(args: &Args) -> Result<(), BoxError> {
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

fn resolve_accelerated_path_config(args: &Args) -> Result<AcceleratedPathConfig, BoxError> {
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
                lane: Some(
                    args.accelerator_lane
                        .unwrap_or(AcceleratedLane::CodecMemmove),
                ),
                direction: Some(AcceleratedDirection::Bidirectional),
            })
        }
    }
}

fn configure_process_controls(args: &Args) -> Result<(), BoxError> {
    runtime_instrumentation::set_enabled(args.instrumentation == InstrumentationMode::On);
    let (buffer_size, yield_threshold) = effective_buffer_settings(args);
    set_default_buffer_settings_for_process(buffer_size, yield_threshold)?;
    Ok(())
}

fn effective_buffer_settings(args: &Args) -> (Option<usize>, Option<usize>) {
    const HEADER_SIZE: usize = tonic::codec::HEADER_SIZE;
    let workload_size = workload_probe(args).request_serialized_size;
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

fn next_multiple(value: usize, multiple: usize) -> usize {
    value.saturating_add(multiple.saturating_sub(1)) / multiple * multiple
}

fn resolve_run_id(args: &Args) -> String {
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

fn server_report_path(args: &Args) -> Option<&PathBuf> {
    if args.mode == Mode::Server {
        args.server_json_out.as_ref().or(args.json_out.as_ref())
    } else {
        None
    }
}

fn build_runtime(mode: RuntimeMode) -> Result<tokio::runtime::Runtime, BoxError> {
    let mut builder = match mode {
        RuntimeMode::Single => Builder::new_current_thread(),
        RuntimeMode::Multi => {
            let mut builder = Builder::new_multi_thread();
            builder.worker_threads(2);
            builder
        }
    };
    builder.enable_all();
    Ok(builder.build()?)
}

async fn async_main(args: Args) -> Result<(), BoxError> {
    let run_id = resolve_run_id(&args);
    match args.mode {
        Mode::Server => {
            if let Some(core) = args.server_core {
                pin_current_thread(core)?;
            }
            run_server(args, &run_id).await
        }
        Mode::Client => {
            if let Some(core) = args.client_core {
                pin_current_thread(core)?;
            }
            let report = run_client(&args, "client", &run_id).await?;
            emit_report(args.json_out.as_ref(), "client", report)?;
            Ok(())
        }
        Mode::Selftest => run_selftest(args, &run_id).await,
    }
}

async fn run_selftest(args: Args, run_id: &str) -> Result<(), BoxError> {
    let bind: SocketAddr = args.bind.parse()?;
    let shared = Arc::new(SharedState::default());
    let server_metrics = Arc::new(ServerMetrics::new()?);
    let server_args = args.clone();
    let server_shared = shared.clone();
    let server_metrics_clone = server_metrics.clone();
    let server_run_id = run_id.to_string();
    let server_handle = tokio::spawn(async move {
        run_server_with_shutdown(
            server_args,
            bind,
            server_shared,
            server_metrics_clone,
            &server_run_id,
        )
        .await
    });

    tokio::time::sleep(Duration::from_millis(300)).await;

    let report = match run_client(&args, "selftest", run_id).await {
        Ok(report) => report,
        Err(err) => {
            shared.shutdown.notify_waiters();
            let _ = server_handle.await;
            return Err(format!("selftest client execution failed: {err}").into());
        }
    };
    emit_report(args.json_out.as_ref(), "selftest", report)?;
    shared.shutdown.notify_waiters();
    server_handle.await??;
    Ok(())
}

async fn run_server(args: Args, run_id: &str) -> Result<(), BoxError> {
    let bind: SocketAddr = args.bind.parse()?;
    let shared = Arc::new(SharedState::default());
    let server_metrics = Arc::new(ServerMetrics::new()?);
    run_server_with_shutdown(args, bind, shared, server_metrics, run_id).await
}

async fn run_server_with_shutdown(
    args: Args,
    bind: SocketAddr,
    shared: Arc<SharedState>,
    server_metrics: Arc<ServerMetrics>,
    run_id: &str,
) -> Result<(), BoxError> {
    custom_codec::reset_observations();
    runtime_instrumentation::reset();
    let mut svc = ProfileServer::new(EchoSvc {
        compression: args.compression,
        proto_pools: Arc::new(ProtoPools::new()),
        next_proto_index: AtomicU64::new(0),
        shutdown: shared.shutdown.clone(),
        shutdown_after_requests: args.shutdown_after_requests,
        server_metrics: server_metrics.clone(),
    });
    if args.compression == CompressionMode::On {
        svc = svc
            .accept_compressed(CompressionEncoding::Gzip)
            .send_compressed(CompressionEncoding::Gzip);
    }

    Server::builder()
        .add_service(svc)
        .serve_with_shutdown(bind, async move {
            shared.shutdown.notified().await;
        })
        .await?;

    if let Some(path) = server_report_path(&args) {
        let report = build_server_report(&args, &server_metrics, run_id).await?;
        emit_report(Some(path), "server", report)?;
    }
    Ok(())
}

async fn run_client(
    args: &Args,
    endpoint_role: &'static str,
    run_id: &str,
) -> Result<Report, BoxError> {
    let descriptor = workload_probe(args);
    let acceleration = resolve_accelerated_path_config(args)?;
    let endpoint = Endpoint::from_shared(format!("http://{}", args.target))?;
    let channel = endpoint.connect().await?;
    let warmup_deadline = Instant::now() + Duration::from_millis(args.warmup_ms);

    custom_codec::reset_observations();
    run_phase(channel.clone(), args, true, warmup_deadline, None).await?;

    let measure_deadline = Instant::now() + Duration::from_millis(args.measure_ms);
    runtime_instrumentation::reset();
    let metrics = run_phase(channel, args, false, measure_deadline, args.requests).await?;
    let stages = StageSnapshot::from(runtime_instrumentation::snapshot());
    let observed_codec_settings = custom_codec::observed_settings()
        .ok_or("custom codec observations were missing for this run")?;

    let report = Report {
        metadata: Metadata {
            timestamp_unix_s: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            mode: "client",
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

async fn build_server_report(
    args: &Args,
    server_metrics: &Arc<ServerMetrics>,
    run_id: &str,
) -> Result<Report, BoxError> {
    let descriptor = workload_probe(args);
    let acceleration = resolve_accelerated_path_config(args)?;
    let stages = StageSnapshot::from(runtime_instrumentation::snapshot());
    let metrics = server_metrics.snapshot().await;
    let observed_codec_settings = custom_codec::observed_settings()
        .ok_or("custom codec observations were missing for this server run")?;

    let report = Report {
        metadata: Metadata {
            timestamp_unix_s: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            mode: "server",
            endpoint_role: "server",
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

async fn run_phase(
    channel: Channel,
    args: &Args,
    warmup: bool,
    deadline: Instant,
    requests_target: Option<u64>,
) -> Result<Metrics, BoxError> {
    let request_counter = Arc::new(AtomicU64::new(0));
    let bytes_sent = Arc::new(AtomicU64::new(0));
    let bytes_received = Arc::new(AtomicU64::new(0));
    let hist = Arc::new(Mutex::new(Histogram::<u64>::new(3)?));
    let started = Instant::now();

    let mut handles = Vec::with_capacity(args.concurrency);
    for worker_id in 0..args.concurrency {
        let pool = Arc::new(build_request_pool(args, worker_id as u64));
        let hist = hist.clone();
        let request_counter = request_counter.clone();
        let bytes_sent_counter = bytes_sent.clone();
        let bytes_received_counter = bytes_received.clone();
        let mut client = ProfileClient::new(channel.clone());
        if args.compression == CompressionMode::On {
            client = client
                .send_compressed(CompressionEncoding::Gzip)
                .accept_compressed(CompressionEncoding::Gzip);
        }

        handles.push(tokio::spawn(async move {
            loop {
                if Instant::now() >= deadline {
                    break;
                }
                let next = request_counter.fetch_add(1, Ordering::Relaxed);
                if let Some(target) = requests_target {
                    if next >= target {
                        break;
                    }
                }
                let prepared = &pool[next as usize % pool.len()];
                let start = Instant::now();
                let response = client
                    .unary_echo(Request::new(prepared.request()))
                    .await
                    .map_err(|err| format!("worker {worker_id} unary request failed: {err}"))?;
                let elapsed = start.elapsed().as_micros() as u64;
                let response_size =
                    validate_response(prepared, response.get_ref()).map_err(|err| {
                        format!("worker {worker_id} response validation failed: {err}")
                    })? as u64;
                if !warmup {
                    bytes_sent_counter
                        .fetch_add(prepared.request_serialized_size as u64, Ordering::Relaxed);
                    bytes_received_counter.fetch_add(response_size, Ordering::Relaxed);
                    hist.lock().await.record(elapsed.max(1))?;
                }
            }
            Ok::<(), BoxError>(())
        }));
    }

    for handle in handles {
        handle.await??;
    }

    let elapsed = started.elapsed();
    let recorded = if warmup { 0 } else { hist.lock().await.len() };

    if warmup {
        return Ok(Metrics {
            requests_completed: 0,
            bytes_sent: 0,
            bytes_received: 0,
            duration_ms: elapsed.as_secs_f64() * 1000.0,
            throughput_rps: 0.0,
            throughput_mib_s: 0.0,
            latency_us_p50: 0,
            latency_us_p95: 0,
            latency_us_p99: 0,
            latency_us_max: 0,
        });
    }

    let hist = Arc::try_unwrap(hist)
        .map_err(|_| "histogram still shared")?
        .into_inner();
    let duration_s = elapsed.as_secs_f64().max(1e-9);
    let sent = bytes_sent.load(Ordering::Relaxed);
    let received = bytes_received.load(Ordering::Relaxed);
    Ok(Metrics {
        requests_completed: recorded,
        bytes_sent: sent,
        bytes_received: received,
        duration_ms: elapsed.as_secs_f64() * 1000.0,
        throughput_rps: recorded as f64 / duration_s,
        throughput_mib_s: received as f64 / duration_s / (1024.0 * 1024.0),
        latency_us_p50: hist.value_at_quantile(0.50),
        latency_us_p95: hist.value_at_quantile(0.95),
        latency_us_p99: hist.value_at_quantile(0.99),
        latency_us_max: hist.max(),
    })
}

fn emit_report(
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

fn workload_probe(args: &Args) -> WorkloadDescriptor {
    let pool = build_request_pool(args, 0);
    let first = pool.first().expect("workload pool should not be empty");
    let request_shape = match args.rpc {
        RpcMode::UnaryBytes => None,
        RpcMode::UnaryProtoShape => args.proto_shape,
    };
    let response_shape = match args.rpc {
        RpcMode::UnaryBytes => None,
        RpcMode::UnaryProtoShape => Some(resolve_response_shape(
            request_shape.expect("proto shape required for unary-proto-shape"),
            args.response_shape,
        )),
    };
    let selection_policy = match args.rpc {
        RpcMode::UnaryBytes => SelectionPolicy::EchoPayload,
        RpcMode::UnaryProtoShape => match args.response_shape {
            ResponseShape::Same => SelectionPolicy::SameAsRequest,
            _ => SelectionPolicy::ExplicitResponse,
        },
    };
    let workload_label = match args.rpc {
        RpcMode::UnaryBytes => format!(
            "ordinary/unary-bytes/{}-{}",
            serialize_payload_kind(args.payload_kind),
            args.payload_size
        ),
        RpcMode::UnaryProtoShape => format!(
            "ordinary/unary-proto-shape/{}-to-{}",
            serialize_proto_shape(request_shape.expect("proto shape")),
            serialize_proto_shape(response_shape.expect("response shape"))
        ),
    };

    WorkloadDescriptor {
        workload_label,
        selection_policy,
        request_shape,
        response_shape,
        payload_size: match args.rpc {
            RpcMode::UnaryBytes => Some(args.payload_size),
            RpcMode::UnaryProtoShape => None,
        },
        payload_kind: match args.rpc {
            RpcMode::UnaryBytes => Some(args.payload_kind),
            RpcMode::UnaryProtoShape => None,
        },
        request_serialized_size: first.request_serialized_size,
        response_serialized_size: first.response_serialized_size,
    }
}

fn build_request_pool(args: &Args, worker_seed: u64) -> Vec<PreparedRequest> {
    match args.rpc {
        RpcMode::UnaryBytes => (0..REQUEST_POOL_LEN)
            .map(|offset| {
                let payload = make_payload(
                    args.payload_size,
                    args.payload_kind,
                    worker_seed * 100 + offset as u64,
                );
                let request = EchoRequest {
                    body: Some(EchoRequestBody::Payload(payload.clone())),
                    requested_response_shape: ShapeKind::Unspecified as i32,
                };
                let reply = EchoReply {
                    body: Some(EchoReplyBody::Payload(payload.clone())),
                };
                PreparedRequest {
                    body: PreparedRequestBody::Bytes(payload),
                    expected_response_shape: None,
                    request_serialized_size: request.encoded_len(),
                    response_serialized_size: reply.encoded_len(),
                }
            })
            .collect(),
        RpcMode::UnaryProtoShape => {
            let request_shape = args.proto_shape.expect("proto shape required");
            let response_shape = resolve_response_shape(request_shape, args.response_shape);
            (0..REQUEST_POOL_LEN)
                .map(|offset| {
                    let request_message =
                        build_proto_message(request_shape, worker_seed * 100 + offset as u64);
                    let response_message = build_proto_message(
                        response_shape,
                        10_000 + worker_seed * 100 + offset as u64,
                    );
                    let request = EchoRequest {
                        body: Some(EchoRequestBody::ProtoPayload(request_message.clone())),
                        requested_response_shape: response_shape.to_wire(),
                    };
                    let reply = EchoReply {
                        body: Some(EchoReplyBody::ProtoPayload(response_message)),
                    };
                    PreparedRequest {
                        body: PreparedRequestBody::Proto(request_message),
                        expected_response_shape: Some(response_shape),
                        request_serialized_size: request.encoded_len(),
                        response_serialized_size: reply.encoded_len(),
                    }
                })
                .collect()
        }
    }
}

impl PreparedRequest {
    fn request(&self) -> EchoRequest {
        match &self.body {
            PreparedRequestBody::Bytes(payload) => EchoRequest {
                body: Some(EchoRequestBody::Payload(payload.clone())),
                requested_response_shape: ShapeKind::Unspecified as i32,
            },
            PreparedRequestBody::Proto(message) => EchoRequest {
                body: Some(EchoRequestBody::ProtoPayload(message.clone())),
                requested_response_shape: self
                    .expected_response_shape
                    .expect("proto workloads must have a response shape")
                    .to_wire(),
            },
        }
    }
}

fn validate_response(prepared: &PreparedRequest, response: &EchoReply) -> Result<usize, BoxError> {
    match (&prepared.body, response.body.as_ref()) {
        (PreparedRequestBody::Bytes(_), Some(EchoReplyBody::Payload(_))) => {
            Ok(response.encoded_len())
        }
        (PreparedRequestBody::Bytes(_), Some(EchoReplyBody::ProtoPayload(_))) => Err(
            "bytes workload received a proto response instead of an echoed bytes payload".into(),
        ),
        (PreparedRequestBody::Proto(_), Some(EchoReplyBody::Payload(_))) => {
            Err("proto-shape workload received a bytes response instead of a proto shape".into())
        }
        (PreparedRequestBody::Proto(_), Some(EchoReplyBody::ProtoPayload(message))) => {
            let actual = proto_shape_from_message(message).ok_or_else(|| {
                "proto-shape response body was missing a concrete shape".to_string()
            })?;
            let expected = prepared
                .expected_response_shape
                .ok_or_else(|| "proto workload missing expected response shape".to_string())?;
            if actual != expected {
                return Err(format!(
                    "response shape mismatch: expected {}, got {}",
                    serialize_proto_shape(expected),
                    serialize_proto_shape(actual)
                )
                .into());
            }
            let actual_size = response.encoded_len();
            if actual_size != prepared.response_serialized_size {
                return Err(format!(
                    "response serialized size mismatch for {}: expected {}, got {}",
                    serialize_proto_shape(expected),
                    prepared.response_serialized_size,
                    actual_size
                )
                .into());
            }
            Ok(actual_size)
        }
        (_, None) => Err("response body was missing".into()),
    }
}

fn make_payload(size: usize, kind: PayloadKind, seed: u64) -> Vec<u8> {
    match kind {
        PayloadKind::Random => {
            let mut buf = vec![0_u8; size];
            let mut rng = StdRng::seed_from_u64(seed + 1);
            rng.fill_bytes(&mut buf);
            buf
        }
        PayloadKind::Structured => {
            let mut buf = Vec::with_capacity(size);
            let pattern = format!(
                "{{\"id\":{},\"name\":\"tonic-profile\",\"flags\":[1,0,1],\"payload\":\"",
                seed
            );
            while buf.len() < size {
                buf.extend_from_slice(pattern.as_bytes());
                buf.extend_from_slice(b"abcdefghijklmnoqrstuvwxyz0123456789");
            }
            buf.truncate(size);
            buf
        }
        PayloadKind::Repeated => vec![b'R'; size],
    }
}

fn build_proto_message(shape: ProtoShape, seed: u64) -> CompactProfileShape {
    match shape {
        ProtoShape::FleetSmall => CompactProfileShape {
            shape: Some(profile::compact_profile_shape::Shape::FleetSmall(
                FleetSmallShape {
                    id: (seed % 32) + 1,
                    service: format!("svc-{:04}", seed % 10_000),
                    method: format!("unary-{:04}", seed % 10_000),
                    flags: vec![1, 0, 1, (seed % 3) as u32],
                    token: seeded_bytes(seed, 16),
                },
            )),
        },
        ProtoShape::FleetStringHeavy => CompactProfileShape {
            shape: Some(profile::compact_profile_shape::Shape::FleetStringHeavy(
                FleetStringHeavyShape {
                    id: (seed % 32) + 10,
                    labels: (0..6)
                        .map(|idx| fixed_string(&format!("label-{idx}-{}", seed % 97), 28))
                        .collect(),
                    paths: (0..4)
                        .map(|idx| fixed_string(&format!("/fleet/{idx}/{}", seed % 31), 36))
                        .collect(),
                    counters: (0..10).map(|idx| ((seed + idx as u64) % 64) + 1).collect(),
                    leaves: (0..4)
                        .map(|idx| FleetStringLeaf {
                            name: fixed_string(&format!("leaf-{idx}"), 18),
                            value: fixed_string(&format!("value-{}-{idx}", seed % 41), 40),
                            samples: (0..3)
                                .map(|sample| {
                                    fixed_string(
                                        &format!("sample-{idx}-{sample}-{}", seed % 53),
                                        26,
                                    )
                                })
                                .collect(),
                        })
                        .collect(),
                    note: fixed_string(&format!("note-{}", seed % 67), 52),
                },
            )),
        },
        ProtoShape::FleetResponseHeavy => CompactProfileShape {
            shape: Some(profile::compact_profile_shape::Shape::FleetResponseHeavy(
                FleetResponseHeavyShape {
                    tenant_id: (seed % 64) + 100,
                    trace_id: fixed_string(&format!("trace-{:016x}", seed + 100), 32),
                    regions: (0..6)
                        .map(|idx| fixed_string(&format!("region-{idx}-{}", seed % 13), 20))
                        .collect(),
                    entries: (0..12)
                        .map(|idx| FleetResponseEntry {
                            key: fixed_string(&format!("key-{idx}"), 16),
                            value: fixed_string(&format!("value-{idx}-{}", seed % 29), 48),
                            tags: (0..5)
                                .map(|tag| fixed_string(&format!("tag-{idx}-{tag}"), 18))
                                .collect(),
                            samples: (0..10)
                                .map(|sample| ((seed + idx as u64 * 10 + sample as u64) % 96) + 1)
                                .collect(),
                            digest: seeded_bytes(seed + idx as u64, 24),
                        })
                        .collect(),
                    digests: (0..10)
                        .map(|idx| seeded_bytes(seed + idx as u64 + 50, 20))
                        .collect(),
                    histogram: (0..32).map(|idx| ((seed + idx as u64) % 96) + 1).collect(),
                    summary: fixed_string(&format!("response-heavy-summary-{}", seed % 101), 96),
                },
            )),
        },
    }
}

fn fixed_string(seed: &str, len: usize) -> String {
    let mut output = String::with_capacity(len);
    while output.len() < len {
        output.push_str(seed);
        output.push('-');
    }
    output.truncate(len);
    output
}

fn seeded_bytes(seed: u64, len: usize) -> Vec<u8> {
    let mut bytes = vec![0_u8; len];
    let mut rng = StdRng::seed_from_u64(seed + 7);
    rng.fill_bytes(&mut bytes);
    bytes
}

fn proto_shape_from_message(message: &CompactProfileShape) -> Option<ProtoShape> {
    match message.shape.as_ref()? {
        profile::compact_profile_shape::Shape::FleetSmall(_) => Some(ProtoShape::FleetSmall),
        profile::compact_profile_shape::Shape::FleetStringHeavy(_) => {
            Some(ProtoShape::FleetStringHeavy)
        }
        profile::compact_profile_shape::Shape::FleetResponseHeavy(_) => {
            Some(ProtoShape::FleetResponseHeavy)
        }
    }
}

fn proto_shape_from_wire(value: i32) -> Result<Option<ProtoShape>, Status> {
    match ShapeKind::try_from(value).unwrap_or(ShapeKind::Unspecified) {
        ShapeKind::Unspecified => Ok(None),
        ShapeKind::FleetSmall => Ok(Some(ProtoShape::FleetSmall)),
        ShapeKind::FleetStringHeavy => Ok(Some(ProtoShape::FleetStringHeavy)),
        ShapeKind::FleetResponseHeavy => Ok(Some(ProtoShape::FleetResponseHeavy)),
    }
}

fn resolve_response_shape(request_shape: ProtoShape, response_shape: ResponseShape) -> ProtoShape {
    match response_shape {
        ResponseShape::Same => request_shape,
        ResponseShape::FleetSmall => ProtoShape::FleetSmall,
        ResponseShape::FleetStringHeavy => ProtoShape::FleetStringHeavy,
        ResponseShape::FleetResponseHeavy => ProtoShape::FleetResponseHeavy,
    }
}

impl ProtoShape {
    fn to_wire(self) -> i32 {
        match self {
            ProtoShape::FleetSmall => ShapeKind::FleetSmall as i32,
            ProtoShape::FleetStringHeavy => ShapeKind::FleetStringHeavy as i32,
            ProtoShape::FleetResponseHeavy => ShapeKind::FleetResponseHeavy as i32,
        }
    }
}

fn serialize_proto_shape(shape: ProtoShape) -> &'static str {
    match shape {
        ProtoShape::FleetSmall => "fleet-small",
        ProtoShape::FleetStringHeavy => "fleet-string-heavy",
        ProtoShape::FleetResponseHeavy => "fleet-response-heavy",
    }
}

fn serialize_payload_kind(kind: PayloadKind) -> &'static str {
    match kind {
        PayloadKind::Random => "random",
        PayloadKind::Structured => "structured",
        PayloadKind::Repeated => "repeated",
    }
}

fn pin_current_thread(core: usize) -> Result<(), BoxError> {
    unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        libc::CPU_ZERO(&mut set);
        libc::CPU_SET(core, &mut set);
        let result = libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set);
        if result != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    Ok(())
}
