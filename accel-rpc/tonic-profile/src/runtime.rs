use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use hdrhistogram::Histogram;
use tokio::runtime::Builder;
use tokio::sync::Mutex;
use tonic::codec::CompressionEncoding;
use tonic::transport::{Channel, Endpoint, Server};
use tonic::Request;

use crate::cli::{
    effective_buffer_settings, resolve_accelerated_path_config, resolve_run_id, AcceleratedPath,
    Args, CompressionMode, InstrumentationMode, Mode, RuntimeMode,
};
use crate::custom_codec;
use crate::profile::profile_client::ProfileClient;
use crate::report::{
    build_client_report, build_server_report, emit_report, server_report_path, Metrics, Report,
};
use crate::runtime_instrumentation;
use crate::service::{build_profile_server, ServerMetrics, SharedState};
use crate::workload::{build_request_pool, validate_response, workload_probe};
use crate::BoxError;

pub(crate) fn run(args: Args) -> Result<(), BoxError> {
    configure_process_controls(&args)?;
    let runtime = build_runtime(args.runtime)?;
    runtime.block_on(async_main(args))
}

fn configure_process_controls(args: &Args) -> Result<(), BoxError> {
    runtime_instrumentation::set_enabled(args.instrumentation == InstrumentationMode::On);
    let workload_size = workload_probe(args).request_serialized_size;
    let (buffer_size, yield_threshold) = effective_buffer_settings(args, workload_size);
    custom_codec::set_process_default_buffer_settings(buffer_size, yield_threshold)
        .map_err(|err| -> BoxError { err.into() })?;

    let acceleration = resolve_accelerated_path_config(args)?;
    let selected_path = match acceleration.selected_path {
        AcceleratedPath::Software => custom_codec::AcceleratedCopyPath::Software,
        AcceleratedPath::Idxd => custom_codec::AcceleratedCopyPath::Idxd,
    };
    custom_codec::set_process_default_acceleration(selected_path, acceleration.device_path)
        .map_err(|err| -> BoxError { err.into() })?;
    custom_codec::preflight_acceleration().map_err(|err| -> BoxError { err.into() })?;
    Ok(())
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
    let endpoint = Endpoint::from_shared(format!("http://{}", args.target))?;
    let channel = endpoint.connect().await?;
    let warmup_deadline = Instant::now() + Duration::from_millis(args.warmup_ms);

    custom_codec::reset_observations();
    run_phase(channel.clone(), args, true, warmup_deadline, None).await?;

    let measure_deadline = Instant::now() + Duration::from_millis(args.measure_ms);
    runtime_instrumentation::reset();
    let metrics = run_phase(channel, args, false, measure_deadline, args.requests).await?;
    build_client_report(args, endpoint_role, run_id, metrics)
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
