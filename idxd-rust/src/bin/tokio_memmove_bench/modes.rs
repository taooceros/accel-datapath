use std::time::{Duration, Instant};

use bytes::{Bytes, BytesMut};
use idxd_rust::{AsyncDsaHandle, AsyncMemmoveRequest};
use tokio::task::JoinSet;

use crate::artifact::BenchmarkResult;
use crate::cli::{BenchmarkMode, CliArgs};
use crate::failure::RowFailure;

#[derive(Debug, Default)]
pub(crate) struct ModeStats {
    completed: u64,
    failed: u64,
    latencies_ns: Vec<u128>,
    first_failure: Option<RowFailure>,
}

impl ModeStats {
    pub(crate) fn record_success(&mut self, latency_ns: u128) {
        self.completed += 1;
        self.latencies_ns.push(latency_ns.max(1));
    }

    pub(crate) fn record_failure(&mut self, failure: RowFailure) {
        self.failed += 1;
        if self.first_failure.is_none() {
            self.first_failure = Some(failure);
        }
    }

    pub(crate) fn into_result(
        self,
        args: &CliArgs,
        mode: BenchmarkMode,
        target: &'static str,
        comparison_target: Option<&'static str>,
        claim_eligible: bool,
        elapsed_ns: u128,
    ) -> BenchmarkResult {
        let simulated_bytes = self.completed.saturating_mul(args.bytes as u64);
        let ops_per_sec = rate_per_second(self.completed, elapsed_ns);
        let bytes_per_sec = rate_per_second(simulated_bytes, elapsed_ns);
        let min_latency_ns = self.latencies_ns.iter().copied().min();
        let max_latency_ns = self.latencies_ns.iter().copied().max();
        let mean_latency_ns = if self.latencies_ns.is_empty() {
            None
        } else {
            Some(self.latencies_ns.iter().copied().sum::<u128>() / self.latencies_ns.len() as u128)
        };
        let first_failure = self.first_failure.as_ref();

        BenchmarkResult {
            mode: mode.as_str(),
            target,
            comparison_target,
            requested_bytes: args.bytes,
            iterations: args.iterations,
            concurrency: args.concurrency,
            duration_ms: args.duration_ms,
            completed_operations: self.completed,
            failed_operations: self.failed,
            elapsed_ns,
            min_latency_ns,
            mean_latency_ns,
            max_latency_ns,
            ops_per_sec,
            bytes_per_sec,
            verdict: if self.failed == 0 && self.completed > 0 {
                "pass"
            } else {
                "fail"
            },
            failure_class: first_failure.map(|failure| failure.failure_class),
            error_kind: first_failure.map(|failure| failure.error_kind),
            direct_failure_kind: first_failure.and_then(|failure| failure.direct_failure_kind),
            validation_phase: first_failure.and_then(|failure| failure.validation_phase),
            validation_error_kind: first_failure.and_then(|failure| failure.validation_error_kind),
            direct_retry_budget: first_failure.and_then(|failure| failure.direct_retry_budget),
            direct_retry_count: first_failure.and_then(|failure| failure.direct_retry_count),
            completion_status: first_failure.and_then(|failure| failure.completion_status.clone()),
            completion_result: first_failure.and_then(|failure| failure.completion_result),
            completion_bytes_completed: first_failure
                .and_then(|failure| failure.completion_bytes_completed),
            completion_fault_addr: first_failure
                .and_then(|failure| failure.completion_fault_addr.clone()),
            claim_eligible: claim_eligible && self.failed == 0 && self.completed > 0,
        }
    }
}

pub(crate) async fn run_async_mode(
    args: &CliArgs,
    handle: AsyncDsaHandle,
    mode: BenchmarkMode,
    target: &'static str,
    comparison_target: Option<&'static str>,
    claim_eligible: bool,
) -> BenchmarkResult {
    let start = Instant::now();
    let stats = match mode {
        BenchmarkMode::SingleLatency => single_latency(handle, args.bytes, args.iterations).await,
        BenchmarkMode::ConcurrentSubmissions => {
            concurrent_submissions(handle, args.bytes, args.iterations, args.concurrency).await
        }
        BenchmarkMode::FixedDurationThroughput => {
            fixed_duration_throughput(handle, args.bytes, args.concurrency, args.duration_ms).await
        }
    };
    let elapsed_ns = start.elapsed().as_nanos().max(1);
    stats.into_result(
        args,
        mode,
        target,
        comparison_target,
        claim_eligible,
        elapsed_ns,
    )
}

async fn single_latency(handle: AsyncDsaHandle, bytes: usize, iterations: u64) -> ModeStats {
    let mut stats = ModeStats::default();
    for seed in 0..iterations {
        match submit_one(handle.clone(), bytes, seed).await {
            Ok(latency_ns) => stats.record_success(latency_ns),
            Err(failure) => stats.record_failure(failure),
        }
    }
    stats
}

async fn concurrent_submissions(
    handle: AsyncDsaHandle,
    bytes: usize,
    iterations: u64,
    concurrency: u32,
) -> ModeStats {
    let mut stats = ModeStats::default();
    let mut seed = 0;
    for _ in 0..iterations {
        let mut tasks = JoinSet::new();
        for _ in 0..concurrency {
            tasks.spawn(submit_one(handle.clone(), bytes, seed));
            seed += 1;
        }
        drain_join_set(&mut tasks, &mut stats).await;
    }
    stats
}

async fn fixed_duration_throughput(
    handle: AsyncDsaHandle,
    bytes: usize,
    concurrency: u32,
    duration_ms: u64,
) -> ModeStats {
    let mut stats = ModeStats::default();
    let deadline = Instant::now() + Duration::from_millis(duration_ms);
    let mut tasks = JoinSet::new();
    let mut seed = 0;

    while Instant::now() < deadline {
        while tasks.len() < concurrency as usize && Instant::now() < deadline {
            tasks.spawn(submit_one(handle.clone(), bytes, seed));
            seed += 1;
        }

        if tasks.len() >= concurrency as usize {
            drain_one_join(&mut tasks, &mut stats).await;
        } else {
            tokio::task::yield_now().await;
        }
    }

    drain_join_set(&mut tasks, &mut stats).await;
    stats
}

async fn drain_join_set(tasks: &mut JoinSet<Result<u128, RowFailure>>, stats: &mut ModeStats) {
    while !tasks.is_empty() {
        drain_one_join(tasks, stats).await;
    }
}

async fn drain_one_join(tasks: &mut JoinSet<Result<u128, RowFailure>>, stats: &mut ModeStats) {
    match tasks.join_next().await {
        Some(Ok(Ok(latency_ns))) => stats.record_success(latency_ns),
        Some(Ok(Err(failure))) => stats.record_failure(failure),
        Some(Err(_join_error)) => stats.record_failure(RowFailure::join_error()),
        None => {}
    }
}

async fn submit_one(handle: AsyncDsaHandle, bytes: usize, seed: u64) -> Result<u128, RowFailure> {
    let request = build_request(bytes, seed)?;
    let start = Instant::now();
    handle
        .memmove(request)
        .await
        .map_err(|error| RowFailure::async_error(&error))?;
    Ok(start.elapsed().as_nanos().max(1))
}

fn build_request(bytes: usize, seed: u64) -> Result<AsyncMemmoveRequest, RowFailure> {
    let source = Bytes::from(deterministic_source(bytes, seed));
    let destination = BytesMut::with_capacity(bytes);
    AsyncMemmoveRequest::new(source, destination).map_err(|error| RowFailure::request(error.kind()))
}

pub(crate) fn deterministic_source(bytes: usize, seed: u64) -> Vec<u8> {
    (0..bytes)
        .map(|offset| seed.wrapping_add(offset as u64).to_le_bytes()[0])
        .collect()
}

fn rate_per_second(value: u64, elapsed_ns: u128) -> Option<f64> {
    if value == 0 {
        None
    } else {
        Some((value as f64) * 1_000_000_000.0 / (elapsed_ns.max(1) as f64))
    }
}
