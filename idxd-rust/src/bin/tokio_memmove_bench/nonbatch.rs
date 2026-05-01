use std::hint::spin_loop;
use std::time::{Duration, Instant};

use idxd_rust::{
    CompletionAction, CompletionSnapshot, DsaConfig, MemmoveError, MemmovePhase, MemmoveRequest,
    classify_memmove_completion,
};
use idxd_sys::{
    DSA_COMP_NONE, DSA_COMP_STATUS_MASK, DSA_COMP_SUCCESS, DsaCompletionRecord, DsaHwDesc,
    WqPortal, reset_completion, reset_completion_status, touch_fault_page,
};

use crate::artifact::{BenchmarkResult, HARDWARE_ASYNC_TARGET};
use crate::cli::{BenchmarkMode, CliArgs};
use crate::failure::RowFailure;

const SUBMISSION_REJECTION_LIMIT: u32 = 1_000_000;

pub(crate) fn run_nonbatch_submission_mode(args: &CliArgs) -> BenchmarkResult {
    let failure_start = Instant::now();
    match run_nonbatch_submission_inner(args) {
        Ok(run) => success_result(args, run.completed, run.elapsed_ns),
        Err(failure) => failure_result(args, failure, failure_start.elapsed().as_nanos().max(1)),
    }
}

fn run_nonbatch_submission_inner(args: &CliArgs) -> Result<NonBatchRun, RowFailure> {
    let request =
        MemmoveRequest::new(args.bytes).map_err(|error| RowFailure::request(error.kind()))?;
    let config = DsaConfig::builder()
        .device_path(args.device_path.clone())
        .max_page_fault_retries(args.max_page_fault_retries)
        .async_validation_mode(args.validation_mode)
        .build()
        .map_err(|error| RowFailure::sync_error(&error, "validation"))?;
    let portal = WqPortal::open(config.device_path()).map_err(|source| {
        RowFailure::sync_error(
            &MemmoveError::QueueOpen {
                device_path: config.device_path().to_path_buf(),
                phase: MemmovePhase::QueueOpen,
                source,
            },
            "queue_open",
        )
    })?;

    let mut slots = build_slots(args.concurrency, args.bytes, request)?;
    for slot in &mut slots {
        slot.prepare_full();
        submit_until_accepted(&portal, slot)?;
    }
    let start = Instant::now();
    let deadline = start + Duration::from_millis(args.duration_ms);

    let mut completed = 0u64;
    while Instant::now() < deadline {
        let mut made_progress = false;
        for slot in &mut slots {
            match slot.observe(&config)? {
                SlotObservation::Pending => {}
                SlotObservation::Retry => {
                    made_progress = true;
                    submit_until_accepted(&portal, slot)?;
                }
                SlotObservation::Success => {
                    made_progress = true;
                    completed = completed.saturating_add(1);
                    slot.prepare_full();
                    submit_until_accepted(&portal, slot)?;
                }
            }
        }
        if !made_progress {
            spin_loop();
        }
    }

    let elapsed_ns = start.elapsed().as_nanos().max(1);
    drain_inflight(&slots);
    Ok(NonBatchRun {
        completed,
        elapsed_ns,
    })
}

#[derive(Debug, Clone, Copy)]
struct NonBatchRun {
    completed: u64,
    elapsed_ns: u128,
}

fn build_slots(
    concurrency: u32,
    bytes: usize,
    request: MemmoveRequest,
) -> Result<Vec<NonBatchSlot>, RowFailure> {
    let mut slots = Vec::with_capacity(concurrency as usize);
    for seed in 0..concurrency as u64 {
        slots.push(NonBatchSlot::new(bytes, seed, request));
    }
    Ok(slots)
}

struct NonBatchSlot {
    desc: DsaHwDesc,
    comp: DsaCompletionRecord,
    source: Box<[u8]>,
    destination: Box<[u8]>,
    request: MemmoveRequest,
    src_offset: usize,
    dst_offset: usize,
    remaining: usize,
    retries: u32,
    descriptor_matches_full: bool,
}

impl NonBatchSlot {
    fn new(bytes: usize, seed: u64, request: MemmoveRequest) -> Self {
        let source = deterministic_source(bytes, seed).into_boxed_slice();
        let destination = vec![0; bytes].into_boxed_slice();
        let len = request.len();
        let mut slot = Self {
            desc: DsaHwDesc::default(),
            comp: DsaCompletionRecord::default(),
            source,
            destination,
            request,
            src_offset: 0,
            dst_offset: 0,
            remaining: len,
            retries: 0,
            descriptor_matches_full: false,
        };
        slot.prefault_buffers();
        slot
    }

    #[inline(always)]
    fn prepare_full(&mut self) {
        self.src_offset = 0;
        self.dst_offset = 0;
        self.remaining = self.request.len();
        self.retries = 0;
        if self.descriptor_matches_full {
            reset_completion_status(&mut self.comp);
        } else {
            self.fill_descriptor();
        }
    }

    #[inline(always)]
    fn fill_descriptor(&mut self) {
        reset_completion(&mut self.comp);
        let src = self.source.as_ptr().wrapping_add(self.src_offset);
        let dst = self.destination.as_mut_ptr().wrapping_add(self.dst_offset);
        self.desc
            .fill_memmove_to_memory(src, dst, self.remaining as u32);
        self.desc.set_completion(&mut self.comp);
        self.descriptor_matches_full =
            self.src_offset == 0 && self.dst_offset == 0 && self.remaining == self.request.len();
    }

    #[inline(always)]
    fn observe(&mut self, config: &DsaConfig) -> Result<SlotObservation, RowFailure> {
        let raw_status = self.comp.status();
        if raw_status == DSA_COMP_NONE {
            return Ok(SlotObservation::Pending);
        }

        let status = raw_status & DSA_COMP_STATUS_MASK;
        if status == DSA_COMP_SUCCESS {
            return Ok(SlotObservation::Success);
        }

        let snapshot = CompletionSnapshot::new(
            status,
            self.comp.result(),
            self.comp.bytes_completed(),
            self.comp.fault_addr(),
        );
        match classify_memmove_completion(config, self.remaining, snapshot, self.retries) {
            Ok(CompletionAction::Retry(retry)) => {
                touch_fault_page(&self.comp);
                self.src_offset = self.src_offset.saturating_add(retry.next_src_offset);
                self.dst_offset = self.dst_offset.saturating_add(retry.next_dst_offset);
                self.remaining = retry.remaining_bytes;
                self.retries = self.retries.saturating_add(1);
                self.fill_descriptor();
                Ok(SlotObservation::Retry)
            }
            Ok(CompletionAction::Success) => Ok(SlotObservation::Success),
            Err(error) => Err(RowFailure::sync_error(&error, "memmove")),
        }
    }

    #[inline(always)]
    fn prefault_buffers(&mut self) {
        touch_read_only(&self.source);
        touch_writable(&mut self.destination);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotObservation {
    Pending,
    Retry,
    Success,
}

#[inline(always)]
fn submit_until_accepted(portal: &WqPortal, slot: &NonBatchSlot) -> Result<(), RowFailure> {
    let mut rejections = 0u32;
    loop {
        // SAFETY: `slot.desc.as_desc64_ptr()` points at the slot-owned 64-byte
        // descriptor. The slot also owns the completion record and buffers for
        // the accepted operation lifetime and is not reused until completion.
        if unsafe { portal.submit_enqcmd_desc64(slot.desc.as_desc64_ptr()) } {
            return Ok(());
        }
        rejections = rejections.saturating_add(1);
        if rejections >= SUBMISSION_REJECTION_LIMIT {
            return Err(RowFailure::message("submission", "enqcmd_rejection_limit"));
        }
        spin_loop();
    }
}

fn drain_inflight(slots: &[NonBatchSlot]) {
    for slot in slots {
        while slot.comp.status() == DSA_COMP_NONE {
            spin_loop();
        }
    }
}

fn deterministic_source(bytes: usize, seed: u64) -> Vec<u8> {
    (0..bytes)
        .map(|index| ((index as u64).wrapping_mul(31).wrapping_add(seed) & 0xFF) as u8)
        .collect()
}

fn touch_read_only(bytes: &[u8]) {
    if bytes.is_empty() {
        return;
    }
    let page = 4096;
    let mut index = 0;
    while index < bytes.len() {
        // SAFETY: `index` is within the slice bounds and the pointer is derived
        // from a live immutable slice. A volatile read is used only to force the
        // OS to install a readable PTE before DMA; it does not create aliases or
        // mutate the buffer.
        unsafe {
            std::ptr::read_volatile(bytes.as_ptr().add(index));
        }
        index = index.saturating_add(page);
    }
    // SAFETY: The slice is non-empty, so `len - 1` is in bounds. This ensures a
    // small buffer, or the tail after the last page stride, is also prefaulted.
    unsafe {
        std::ptr::read_volatile(bytes.as_ptr().add(bytes.len() - 1));
    }
}

fn touch_writable(bytes: &mut [u8]) {
    if bytes.is_empty() {
        return;
    }
    let page = 4096;
    let mut index = 0;
    while index < bytes.len() {
        // SAFETY: `index` is within the unique mutable slice bounds. The volatile
        // read+write touches the existing initialized byte to force a writable PTE
        // before DMA without changing the logical buffer contents.
        unsafe {
            let ptr = bytes.as_mut_ptr().add(index);
            std::ptr::write_volatile(ptr, std::ptr::read_volatile(ptr));
        }
        index = index.saturating_add(page);
    }
    // SAFETY: The slice is non-empty, so `len - 1` is in bounds. The same
    // read+write prefaults the final page/tail byte.
    unsafe {
        let ptr = bytes.as_mut_ptr().add(bytes.len() - 1);
        std::ptr::write_volatile(ptr, std::ptr::read_volatile(ptr));
    }
}

fn success_result(args: &CliArgs, completed: u64, elapsed_ns: u128) -> BenchmarkResult {
    BenchmarkResult {
        mode: BenchmarkMode::RawNonBatchSubmissionThroughput.as_str(),
        target: HARDWARE_ASYNC_TARGET,
        comparison_target: None,
        requested_bytes: args.bytes,
        iterations: args.iterations,
        concurrency: args.concurrency,
        duration_ms: args.duration_ms,
        completed_operations: completed,
        failed_operations: 0,
        elapsed_ns,
        min_latency_ns: None,
        mean_latency_ns: None,
        max_latency_ns: None,
        ops_per_sec: Some(rate(completed, elapsed_ns)),
        bytes_per_sec: Some(byte_rate(args.bytes, completed, elapsed_ns)),
        verdict: if completed > 0 { "pass" } else { "fail" },
        failure_class: None,
        error_kind: None,
        direct_failure_kind: None,
        validation_phase: None,
        validation_error_kind: None,
        direct_retry_budget: None,
        direct_retry_count: None,
        completion_status: None,
        completion_result: None,
        completion_bytes_completed: None,
        completion_fault_addr: None,
        claim_eligible: completed > 0,
    }
}

fn failure_result(args: &CliArgs, failure: RowFailure, elapsed_ns: u128) -> BenchmarkResult {
    BenchmarkResult {
        mode: BenchmarkMode::RawNonBatchSubmissionThroughput.as_str(),
        target: HARDWARE_ASYNC_TARGET,
        comparison_target: None,
        requested_bytes: args.bytes,
        iterations: args.iterations,
        concurrency: args.concurrency,
        duration_ms: args.duration_ms,
        completed_operations: 0,
        failed_operations: 1,
        elapsed_ns,
        min_latency_ns: None,
        mean_latency_ns: None,
        max_latency_ns: None,
        ops_per_sec: None,
        bytes_per_sec: None,
        verdict: "fail",
        failure_class: Some(failure.failure_class),
        error_kind: Some(failure.error_kind),
        direct_failure_kind: failure.direct_failure_kind,
        validation_phase: failure.validation_phase,
        validation_error_kind: failure.validation_error_kind,
        direct_retry_budget: failure.direct_retry_budget,
        direct_retry_count: failure.direct_retry_count,
        completion_status: failure.completion_status,
        completion_result: failure.completion_result,
        completion_bytes_completed: failure.completion_bytes_completed,
        completion_fault_addr: failure.completion_fault_addr,
        claim_eligible: false,
    }
}

fn rate(completed: u64, elapsed_ns: u128) -> f64 {
    completed as f64 / (elapsed_ns as f64 / 1_000_000_000.0)
}

fn byte_rate(bytes: usize, completed: u64, elapsed_ns: u128) -> f64 {
    let total_bytes = bytes as f64 * completed as f64;
    total_bytes / (elapsed_ns as f64 / 1_000_000_000.0)
}
