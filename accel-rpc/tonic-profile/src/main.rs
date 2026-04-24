use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use clap::{Parser, ValueEnum};
use hdrhistogram::Histogram;
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};
use serde::Serialize;
use tokio::runtime::Builder;
use tokio::sync::{Mutex, Notify};
use tonic::codec::{instrumentation, set_default_buffer_settings_for_process, CompressionEncoding};
use tonic::transport::{Channel, Endpoint, Server};
use tonic::{Request, Response, Status};

pub mod profile {
    tonic::include_proto!("tonicprofile");
}

use profile::profile_client::ProfileClient;
use profile::profile_server::{Profile, ProfileServer};
use profile::{EchoReply, EchoRequest};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum Mode {
    Server,
    Client,
    Selftest,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
enum RpcMode {
    Unary,
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

#[derive(Parser, Debug, Clone)]
struct Args {
    #[arg(long, value_enum)]
    mode: Mode,

    #[arg(long, value_enum, default_value = "unary")]
    rpc: RpcMode,

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

    #[arg(long, value_enum, default_value = "default")]
    buffer_policy: BufferPolicy,

    #[arg(long)]
    server_core: Option<usize>,

    #[arg(long)]
    client_core: Option<usize>,

    #[arg(long)]
    json_out: Option<PathBuf>,
}

#[derive(Default)]
struct SharedState {
    shutdown: Notify,
}

#[derive(Clone)]
struct EchoSvc {
    compression: CompressionMode,
}

#[tonic::async_trait]
impl Profile for EchoSvc {
    async fn unary_echo(
        &self,
        request: Request<EchoRequest>,
    ) -> Result<Response<EchoReply>, Status> {
        let payload = request.into_inner().payload;
        let mut response = Response::new(EchoReply { payload });
        if self.compression == CompressionMode::On {
            response
                .metadata_mut()
                .insert("x-tonic-profile", "gzip".parse().unwrap());
        }
        Ok(response)
    }
}

#[derive(Serialize)]
struct Metadata {
    timestamp_unix_s: u64,
    mode: &'static str,
    rpc: RpcMode,
    bind: String,
    target: String,
    payload_size: usize,
    payload_kind: PayloadKind,
    compression: CompressionMode,
    concurrency: usize,
    requests_target: Option<u64>,
    warmup_ms: u64,
    measure_ms: u64,
    runtime: RuntimeMode,
    instrumentation: InstrumentationMode,
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
    configure_process_controls(&args);
    let runtime = build_runtime(args.runtime)?;
    runtime.block_on(async_main(args))
}

fn configure_process_controls(args: &Args) {
    instrumentation::set_enabled(args.instrumentation == InstrumentationMode::On);
    let (buffer_size, yield_threshold) = effective_buffer_settings(args);
    set_default_buffer_settings_for_process(buffer_size, yield_threshold);
}

fn effective_buffer_settings(args: &Args) -> (Option<usize>, Option<usize>) {
    const DEFAULT_BUFFER_SIZE: usize = 8 * 1024;
    const DEFAULT_YIELD_THRESHOLD: usize = 32 * 1024;
    const HEADER_SIZE: usize = 5;
    match args.buffer_policy {
        BufferPolicy::Default => (None, None),
        BufferPolicy::Pooled => {
            let frame = args
                .payload_size
                .saturating_add(HEADER_SIZE)
                .max(DEFAULT_BUFFER_SIZE);
            let buffer = next_multiple(frame, DEFAULT_BUFFER_SIZE);
            (Some(buffer), Some(buffer.max(DEFAULT_YIELD_THRESHOLD)))
        }
        BufferPolicy::CopyMinimized => {
            let buffer = args.payload_size.saturating_add(HEADER_SIZE).max(1);
            (Some(buffer), Some(buffer.max(DEFAULT_YIELD_THRESHOLD)))
        }
    }
}

fn next_multiple(value: usize, multiple: usize) -> usize {
    value.saturating_add(multiple.saturating_sub(1)) / multiple * multiple
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
    match args.mode {
        Mode::Server => {
            if let Some(core) = args.server_core {
                pin_current_thread(core)?;
            }
            run_server(args).await
        }
        Mode::Client => {
            if let Some(core) = args.client_core {
                pin_current_thread(core)?;
            }
            let report = run_client(&args).await?;
            emit_report(&args, "client", report)?;
            Ok(())
        }
        Mode::Selftest => run_selftest(args).await,
    }
}

async fn run_selftest(args: Args) -> Result<(), BoxError> {
    let bind: SocketAddr = args.bind.parse()?;
    let shared = Arc::new(SharedState::default());
    let server_args = args.clone();
    let server_shared = shared.clone();
    let server_handle =
        tokio::spawn(
            async move { run_server_with_shutdown(server_args, bind, server_shared).await },
        );

    tokio::time::sleep(Duration::from_millis(300)).await;

    let report = run_client(&args).await?;
    emit_report(&args, "selftest", report)?;
    shared.shutdown.notify_waiters();
    server_handle.await??;
    Ok(())
}

async fn run_server(args: Args) -> Result<(), BoxError> {
    let bind: SocketAddr = args.bind.parse()?;
    let shared = Arc::new(SharedState::default());
    run_server_with_shutdown(args, bind, shared).await
}

async fn run_server_with_shutdown(
    args: Args,
    bind: SocketAddr,
    shared: Arc<SharedState>,
) -> Result<(), BoxError> {
    let mut svc = ProfileServer::new(EchoSvc {
        compression: args.compression,
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
    Ok(())
}

async fn run_client(args: &Args) -> Result<Report, BoxError> {
    let endpoint = Endpoint::from_shared(format!("http://{}", args.target))?;
    let channel = endpoint.connect().await?;
    let warmup_deadline = Instant::now() + Duration::from_millis(args.warmup_ms);

    run_phase(channel.clone(), args, true, warmup_deadline, None).await?;

    let measure_deadline = Instant::now() + Duration::from_millis(args.measure_ms);
    instrumentation::reset();
    let metrics = run_phase(channel, args, false, measure_deadline, args.requests).await?;
    let stages = StageSnapshot::from(instrumentation::snapshot());
    let (effective_codec_buffer_size, effective_codec_yield_threshold) =
        effective_buffer_settings(args);

    Ok(Report {
        metadata: Metadata {
            timestamp_unix_s: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            mode: "client",
            rpc: args.rpc,
            bind: args.bind.clone(),
            target: args.target.clone(),
            payload_size: args.payload_size,
            payload_kind: args.payload_kind,
            compression: args.compression,
            concurrency: args.concurrency,
            requests_target: args.requests,
            warmup_ms: args.warmup_ms,
            measure_ms: args.measure_ms,
            runtime: args.runtime,
            instrumentation: args.instrumentation,
            buffer_policy: args.buffer_policy,
            effective_codec_buffer_size,
            effective_codec_yield_threshold,
            server_core: args.server_core,
            client_core: args.client_core,
        },
        metrics,
        stages,
    })
}

impl From<instrumentation::Snapshot> for StageSnapshot {
    fn from(value: instrumentation::Snapshot) -> Self {
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

impl From<instrumentation::Counter> for StageCounter {
    fn from(value: instrumentation::Counter) -> Self {
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
    let bytes_counter = Arc::new(AtomicU64::new(0));
    let hist = Arc::new(Mutex::new(Histogram::<u64>::new(3)?));
    let started = Instant::now();

    let mut handles = Vec::with_capacity(args.concurrency);
    for worker_id in 0..args.concurrency {
        let payload = Arc::new(make_payload(
            args.payload_size,
            args.payload_kind,
            worker_id as u64,
        ));
        let hist = hist.clone();
        let request_counter = request_counter.clone();
        let bytes_counter = bytes_counter.clone();
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
                let start = Instant::now();
                let response = client
                    .unary_echo(Request::new(EchoRequest {
                        payload: (*payload).clone(),
                    }))
                    .await?;
                let elapsed = start.elapsed().as_micros() as u64;
                let bytes = response.get_ref().payload.len() as u64;
                if !warmup {
                    bytes_counter.fetch_add(bytes, Ordering::Relaxed);
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
    let bytes = bytes_counter.load(Ordering::Relaxed);
    Ok(Metrics {
        requests_completed: recorded,
        bytes_sent: recorded.saturating_mul(args.payload_size as u64),
        bytes_received: bytes,
        duration_ms: elapsed.as_secs_f64() * 1000.0,
        throughput_rps: recorded as f64 / duration_s,
        throughput_mib_s: bytes as f64 / duration_s / (1024.0 * 1024.0),
        latency_us_p50: hist.value_at_quantile(0.50),
        latency_us_p95: hist.value_at_quantile(0.95),
        latency_us_p99: hist.value_at_quantile(0.99),
        latency_us_max: hist.max(),
    })
}

fn emit_report(args: &Args, mode: &'static str, mut report: Report) -> Result<(), BoxError> {
    report.metadata.mode = mode;
    let json = serde_json::to_string_pretty(&report)?;
    if let Some(path) = &args.json_out {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, &json)?;
    }
    println!("{}", json);
    Ok(())
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
