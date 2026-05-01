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
    min_latency_ns: Option<u128>,
    max_latency_ns: Option<u128>,
    total_latency_ns: u128,
    first_failure: Option<RowFailure>,
}

impl ModeStats {
    pub(crate) fn record_success(&mut self, latency_ns: u128) {
        let latency_ns = latency_ns.max(1);
        self.completed += 1;
        self.total_latency_ns = self.total_latency_ns.saturating_add(latency_ns);
        self.min_latency_ns = Some(
            self.min_latency_ns
                .map(|current| current.min(latency_ns))
                .unwrap_or(latency_ns),
        );
        self.max_latency_ns = Some(
            self.max_latency_ns
                .map(|current| current.max(latency_ns))
                .unwrap_or(latency_ns),
        );
    }

    pub(crate) fn record_failure(&mut self, failure: RowFailure) {
        self.failed += 1;
        if self.first_failure.is_none() {
            self.first_failure = Some(failure);
        }
    }

    fn merge(&mut self, other: ModeStats) {
        self.completed = self.completed.saturating_add(other.completed);
        self.failed = self.failed.saturating_add(other.failed);
        self.total_latency_ns = self.total_latency_ns.saturating_add(other.total_latency_ns);
        self.min_latency_ns = match (self.min_latency_ns, other.min_latency_ns) {
            (Some(left), Some(right)) => Some(left.min(right)),
            (Some(value), None) | (None, Some(value)) => Some(value),
            (None, None) => None,
        };
        self.max_latency_ns = match (self.max_latency_ns, other.max_latency_ns) {
            (Some(left), Some(right)) => Some(left.max(right)),
            (Some(value), None) | (None, Some(value)) => Some(value),
            (None, None) => None,
        };
        if self.first_failure.is_none() {
            self.first_failure = other.first_failure;
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
        let mean_latency_ns = if self.completed == 0 {
            None
        } else {
            Some(self.total_latency_ns / self.completed as u128)
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
            min_latency_ns: self.min_latency_ns,
            mean_latency_ns,
            max_latency_ns: self.max_latency_ns,
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
        BenchmarkMode::RawAsyncThroughput => {
            raw_async_throughput(handle, args.bytes, args.concurrency, args.duration_ms).await
        }
        BenchmarkMode::RawNonBatchSubmissionThroughput => {
            unreachable!("non-batch submission throughput uses the hardware slot-ring runner")
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

async fn raw_async_throughput(
    handle: AsyncDsaHandle,
    bytes: usize,
    concurrency: u32,
    duration_ms: u64,
) -> ModeStats {
    let deadline = Instant::now() + Duration::from_millis(duration_ms);
    let source = Bytes::from(deterministic_source(bytes, 0));
    let mut tasks = JoinSet::new();

    for _slot in 0..concurrency {
        tasks.spawn(raw_throughput_slot(
            handle.clone(),
            source.clone(),
            bytes,
            deadline,
        ));
    }

    let mut stats = ModeStats::default();
    while let Some(joined) = tasks.join_next().await {
        match joined {
            Ok(slot_stats) => stats.merge(slot_stats),
            Err(_join_error) => stats.record_failure(RowFailure::join_error()),
        }
    }
    stats
}

async fn raw_throughput_slot(
    handle: AsyncDsaHandle,
    source: Bytes,
    bytes: usize,
    deadline: Instant,
) -> ModeStats {
    let mut stats = ModeStats::default();
    let mut slot = RequestSlot::new(source, bytes);

    while Instant::now() < deadline {
        let op_start = Instant::now();
        match slot.submit(&handle).await {
            Ok(()) => stats.record_success(op_start.elapsed().as_nanos().max(1)),
            Err(failure) => {
                stats.record_failure(failure);
                break;
            }
        }
    }

    stats
}

#[derive(Debug)]
pub(crate) struct RequestSlot {
    source: Bytes,
    destination: BytesMut,
}

impl RequestSlot {
    pub(crate) fn new(source: Bytes, bytes: usize) -> Self {
        Self {
            source,
            destination: BytesMut::with_capacity(bytes),
        }
    }

    async fn submit(&mut self, handle: &AsyncDsaHandle) -> Result<(), RowFailure> {
        let destination = std::mem::take(&mut self.destination);
        let request = AsyncMemmoveRequest::new(self.source.clone(), destination)
            .map_err(|error| RowFailure::request(error.kind()))?;
        let result = handle
            .memmove(request)
            .await
            .map_err(|error| RowFailure::async_error(&error))?;
        let mut destination = result.destination;
        destination.clear();
        self.destination = destination;
        Ok(())
    }
}

pub(crate) fn build_request(bytes: usize, seed: u64) -> Result<AsyncMemmoveRequest, RowFailure> {
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
