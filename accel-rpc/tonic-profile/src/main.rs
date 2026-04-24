use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use clap::Parser;
use hdrhistogram::Histogram;
use serde::Serialize;
use tokio::runtime::Builder;
use tokio::sync::Mutex;
use tonic::codec::CompressionEncoding;
use tonic::transport::{Channel, Endpoint, Server};
use tonic::Request;

mod cli;
mod custom_codec;
mod runtime_instrumentation;
mod service;
mod workload;

pub mod profile {
    tonic::include_proto!("tonicprofile");
}

use profile::profile_client::ProfileClient;

use cli::{
    AcceleratedDirection, AcceleratedLane, AcceleratedPath, Args, BufferPolicy, CompressionMode,
    InstrumentationMode, Mode, PayloadKind, ProtoShape, RpcMode, RuntimeMode,
    effective_buffer_settings, resolve_accelerated_path_config, resolve_run_id, validate_args,
};
use service::{build_profile_server, ServerMetrics, SharedState};
use workload::{build_request_pool, validate_response, workload_probe, SelectionPolicy};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

fn set_default_buffer_settings_for_process(
    buffer_size: Option<usize>,
    yield_threshold: Option<usize>,
) -> Result<custom_codec::EffectiveBufferSettings, BoxError> {
    custom_codec::set_process_default_buffer_settings(buffer_size, yield_threshold)
        .map_err(Into::into)
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

fn main() -> Result<(), BoxError> {
    let args = Args::parse();
    validate_args(&args)?;
    configure_process_controls(&args)?;
    let runtime = build_runtime(args.runtime)?;
    runtime.block_on(async_main(args))
}

fn configure_process_controls(args: &Args) -> Result<(), BoxError> {
    runtime_instrumentation::set_enabled(args.instrumentation == InstrumentationMode::On);
    let workload_size = workload_probe(args).request_serialized_size;
    let (buffer_size, yield_threshold) = effective_buffer_settings(args, workload_size);
    set_default_buffer_settings_for_process(buffer_size, yield_threshold)?;

    let acceleration = resolve_accelerated_path_config(args)?;
    let selected_path = match acceleration.selected_path {
        AcceleratedPath::Software => custom_codec::AcceleratedCopyPath::Software,
        AcceleratedPath::Idxd => custom_codec::AcceleratedCopyPath::Idxd,
    };
    custom_codec::set_process_default_acceleration(selected_path, acceleration.device_path)?;
    custom_codec::preflight_acceleration().map_err(|err| -> BoxError { err.into() })?;
    Ok(())
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
    let svc = build_profile_server(&args, &shared, &server_metrics);

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
