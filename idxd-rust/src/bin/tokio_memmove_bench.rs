use std::{
    env,
    error::Error,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use idxd_rust::{DsaCompletionRecord, DsaCompletionStatus, DsaEngine};
use tokio::{runtime::Builder, task::JoinSet};

const DEFAULT_TOTAL_BYTES: usize = 1 << 30;
const DEFAULT_MESSAGE_BYTES: usize = 4096;
const DEFAULT_CONCURRENCY: usize = 64;
const DEVICE_ENV: &str = "IDXD_RUST_DSA_WQ";

#[derive(Debug, Clone)]
struct Config {
    device: PathBuf,
    total_bytes: usize,
    message_bytes: usize,
    concurrency: usize,
    threads: usize,
    verify: bool,
    json: bool,
}

#[derive(Debug, Clone, Copy)]
struct WorkerReport {
    ops: usize,
    bytes: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    let Some(config) = parse_config()? else {
        print_help();
        return Ok(());
    };

    let runtime = Builder::new_multi_thread()
        .worker_threads(config.threads)
        .enable_all()
        .build()?;

    runtime.block_on(run_benchmark(config))
}

async fn run_benchmark(config: Config) -> Result<(), Box<dyn Error>> {
    let engine = Arc::new(DsaEngine::open(&config.device)?);
    let message = Arc::<[u8]>::from(vec![0x5a; config.message_bytes]);
    let total_ops = config.total_bytes / config.message_bytes;
    if total_ops == 0 {
        return Err("total bytes must cover at least one message".into());
    }

    let workers = config.concurrency.min(total_ops);
    let base_ops = total_ops / workers;
    let remainder = total_ops % workers;

    let start = Instant::now();
    let mut tasks = JoinSet::new();

    for worker in 0..workers {
        let engine = Arc::clone(&engine);
        let message = Arc::clone(&message);
        let ops = base_ops + usize::from(worker < remainder);
        let verify = config.verify;
        tasks.spawn(async move { run_worker(engine, message, ops, verify).await });
    }

    let mut completed_ops = 0usize;
    let mut completed_bytes = 0usize;
    while let Some(result) = tasks.join_next().await {
        let report = result.map_err(|err| format!("tokio worker join failed: {err}"))??;
        completed_ops += report.ops;
        completed_bytes += report.bytes;
    }

    let elapsed = start.elapsed();
    print_report(&config, completed_ops, completed_bytes, elapsed);
    Ok(())
}

async fn run_worker(
    engine: Arc<DsaEngine>,
    message: Arc<[u8]>,
    ops: usize,
    verify: bool,
) -> Result<WorkerReport, String> {
    let mut dst = vec![0u8; message.len()];
    touch_pages(&mut dst);

    for _ in 0..ops {
        if verify {
            dst.fill(0);
        }

        let completion = engine.memmove(message.as_ref(), &mut dst).await;
        ensure_success(completion)?;

        if verify && dst.as_slice() != message.as_ref() {
            return Err("memmove verification failed".to_string());
        }
    }

    Ok(WorkerReport {
        ops,
        bytes: ops * message.len(),
    })
}

fn touch_pages(buffer: &mut [u8]) {
    let page = 4096;
    for index in (0..buffer.len()).step_by(page) {
        buffer[index] = buffer[index].wrapping_add(1);
    }
    if let Some(last) = buffer.last_mut() {
        *last = last.wrapping_add(1);
    }
}

fn ensure_success(completion: DsaCompletionRecord) -> Result<(), String> {
    let status = DsaCompletionStatus::mask(completion.status());
    if status == DsaCompletionStatus::Success.as_u8() {
        Ok(())
    } else {
        Err(format!(
            "DSA completion status={:#04x} result={:#04x} bytes_completed={} fault_addr={:#x}",
            completion.status(),
            completion.result(),
            completion.bytes_completed(),
            completion.fault_addr()
        ))
    }
}

fn print_report(config: &Config, ops: usize, bytes: usize, elapsed: Duration) {
    let seconds = elapsed.as_secs_f64();
    let ops_per_second = ops as f64 / seconds;
    let ns_per_op = seconds * 1e9 / ops as f64;
    let gb_per_second = bytes as f64 / seconds / 1e9;
    let strategy = "tokio_naive_direct_descriptor";
    let batch_n = 1usize;

    if config.json {
        println!(
            "{{\"strategy\":\"{}\",\"batch_n\":{},\"device\":\"{}\",\"total_bytes\":{},\"message_bytes\":{},\"concurrency\":{},\"threads\":{},\"ops\":{},\"elapsed_seconds\":{:.9},\"ops_per_second\":{:.3},\"ns_per_op\":{:.3},\"gb_per_second\":{:.6},\"verify\":{}}}",
            strategy,
            batch_n,
            config.device.display(),
            config.total_bytes,
            config.message_bytes,
            config.concurrency,
            config.threads,
            ops,
            seconds,
            ops_per_second,
            ns_per_op,
            gb_per_second,
            config.verify,
        );
    } else {
        println!("idxd-rust Tokio memmove benchmark");
        println!("  strategy:      {}", strategy);
        println!("  batch_n:       {}", batch_n);
        println!("  device:        {}", config.device.display());
        println!("  total bytes:   {}", config.total_bytes);
        println!("  message bytes: {}", config.message_bytes);
        println!("  concurrency:   {}", config.concurrency);
        println!("  runtime tasks: {}", config.threads);
        println!("  verify:        {}", config.verify);
        println!("  ops:           {}", ops);
        println!("  elapsed:       {:.6} s", seconds);
        println!("  throughput:    {:.3} ops/s", ops_per_second);
        println!("  latency:       {:.3} ns/op", ns_per_op);
        println!("  bandwidth:     {:.6} GB/s", gb_per_second);
    }
}

fn parse_config() -> Result<Option<Config>, Box<dyn Error>> {
    let mut device = env::var_os(DEVICE_ENV).map(PathBuf::from);
    let mut total_bytes = DEFAULT_TOTAL_BYTES;
    let mut message_bytes = DEFAULT_MESSAGE_BYTES;
    let mut concurrency = DEFAULT_CONCURRENCY;
    let mut threads = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1);
    let mut verify = false;
    let mut json = false;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(None),
            "--device" => device = Some(PathBuf::from(next_value(&mut args, "--device")?)),
            "--total-bytes" => total_bytes = parse_usize(&next_value(&mut args, "--total-bytes")?)?,
            "--message-bytes" => {
                message_bytes = parse_usize(&next_value(&mut args, "--message-bytes")?)?
            }
            "--concurrency" => concurrency = parse_usize(&next_value(&mut args, "--concurrency")?)?,
            "--threads" => threads = parse_usize(&next_value(&mut args, "--threads")?)?,
            "--verify" => verify = true,
            "--json" => json = true,
            other => return Err(format!("unknown argument: {other}").into()),
        }
    }

    let device = device.ok_or_else(|| {
        format!("missing --device <path> or {DEVICE_ENV}=/dev/dsa/wqX.Y environment variable")
    })?;

    if total_bytes == 0 {
        return Err("--total-bytes must be non-zero".into());
    }
    if message_bytes == 0 {
        return Err("--message-bytes must be non-zero".into());
    }
    if concurrency == 0 {
        return Err("--concurrency must be non-zero".into());
    }
    if threads == 0 {
        return Err("--threads must be non-zero".into());
    }

    Ok(Some(Config {
        device,
        total_bytes,
        message_bytes,
        concurrency,
        threads,
        verify,
        json,
    }))
}

fn next_value(args: &mut impl Iterator<Item = String>, name: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("missing value for {name}"))
}

fn parse_usize(value: &str) -> Result<usize, Box<dyn Error>> {
    value
        .parse::<usize>()
        .map_err(|err| format!("invalid integer {value:?}: {err}").into())
}

fn print_help() {
    println!(
        "idxd-rust Tokio memmove benchmark\n\n\
Usage:\n  tokio_memmove_bench --device /dev/dsa/wqX.Y [options]\n\n\
Options:\n  --device <path>          DSA work queue path; defaults to IDXD_RUST_DSA_WQ\n  --total-bytes <bytes>    Total logical bytes to copy [default: {DEFAULT_TOTAL_BYTES}]\n  --message-bytes <bytes>  Bytes per memmove operation [default: {DEFAULT_MESSAGE_BYTES}]\n  --concurrency <n>        Number of Tokio worker tasks / outstanding operations [default: {DEFAULT_CONCURRENCY}]\n  --threads <n>            Tokio runtime worker threads [default: available parallelism]\n  --verify                 Check destination bytes after each operation\n  --json                   Emit one JSON object\n  -h, --help               Print this help\n\n\
Run hardware benchmarks through launch, for example:\n  IDXD_RUST_DSA_WQ=/dev/dsa/wq0.0 launch cargo run --release --manifest-path ./Cargo.toml -p idxd-rust --bin tokio_memmove_bench -- --total-bytes 1073741824 --message-bytes 4096 --concurrency 64\n"
    );
}
