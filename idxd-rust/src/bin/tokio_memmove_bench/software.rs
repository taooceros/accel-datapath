use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

use bytes::buf::UninitSlice;
use idxd_rust::{AsyncDsaSession, DirectMemmoveBackend, DsaConfig};
use idxd_sys::{DSA_COMP_SUCCESS, DsaHwDesc, EnqcmdSubmission};

use crate::artifact::{BenchmarkArtifact, SCHEMA_VERSION, SOFTWARE_TARGET};
use crate::cli::{Backend, CliArgs};
use crate::modes::run_async_mode;

#[derive(Debug, Clone)]
struct SoftwareDirectBackend {
    inner: Arc<SoftwareBackendInner>,
}

#[derive(Debug, Default)]
struct SoftwareBackendInner {
    submitted_op_ids: Mutex<Vec<u64>>,
    successful_copies: AtomicU64,
}

impl SoftwareDirectBackend {
    fn new() -> Self {
        Self {
            inner: Arc::new(SoftwareBackendInner::default()),
        }
    }
}

impl DirectMemmoveBackend for SoftwareDirectBackend {
    fn submit(&self, op_id: u64, descriptor: &DsaHwDesc) -> EnqcmdSubmission {
        self.inner
            .submitted_op_ids
            .lock()
            .expect("software backend submission registry poisoned")
            .push(op_id);

        let completion_addr = descriptor.completion_addr() as *mut u8;
        if !completion_addr.is_null() {
            // SAFETY: The direct runtime gave the backend a descriptor whose completion
            // address points at the operation-owned completion record. The diagnostic
            // backend only publishes the terminal success status byte; payload bytes are
            // copied later by initialize_success_destination, preserving the runtime's
            // success-copy boundary.
            unsafe {
                std::ptr::write_volatile(completion_addr, DSA_COMP_SUCCESS);
            }
        }

        EnqcmdSubmission::Accepted
    }

    fn initialize_success_destination(&self, _op_id: u64, dst: &mut UninitSlice, src: &[u8]) {
        self.inner.successful_copies.fetch_add(1, Ordering::SeqCst);
        dst.copy_from_slice(src);
    }
}

pub(crate) async fn software_artifact(args: &CliArgs) -> BenchmarkArtifact {
    let config = match DsaConfig::builder()
        .device_path(args.device_path.clone())
        .build()
    {
        Ok(config) => config,
        Err(error) => {
            return top_level_failure_artifact(
                args,
                "validation",
                error.kind(),
                Some("preflight"),
                Some(error.kind()),
            );
        }
    };
    let backend = SoftwareDirectBackend::new();
    let session = match AsyncDsaSession::spawn_with_direct_backend(config, backend) {
        Ok(session) => session,
        Err(error) => {
            return top_level_failure_artifact(args, "async_direct", error.kind(), None, None);
        }
    };
    let handle = session.handle();

    let mut results = Vec::with_capacity(args.suite.modes().len());
    for mode in args.suite.modes() {
        results
            .push(run_async_mode(args, handle.clone(), *mode, SOFTWARE_TARGET, None, false).await);
    }

    drop(session);

    let first_failure = results.iter().find(|result| result.verdict != "pass");
    BenchmarkArtifact {
        schema_version: SCHEMA_VERSION,
        ok: first_failure.is_none(),
        verdict: if first_failure.is_none() {
            "pass"
        } else {
            "fail"
        },
        device_path: args.device_path.display().to_string(),
        backend: Backend::Software.as_str(),
        claim_eligible: false,
        suite: args.suite.as_str(),
        runtime_flavor: "current_thread",
        worker_threads: 1,
        requested_bytes: args.bytes,
        iterations: args.iterations,
        concurrency: args.concurrency,
        duration_ms: args.duration_ms,
        failure_class: first_failure.and_then(|result| result.failure_class),
        error_kind: first_failure.and_then(|result| result.error_kind),
        direct_failure_kind: first_failure.and_then(|result| result.direct_failure_kind),
        validation_phase: first_failure.and_then(|result| result.validation_phase),
        validation_error_kind: first_failure.and_then(|result| result.validation_error_kind),
        direct_retry_budget: first_failure.and_then(|result| result.direct_retry_budget),
        direct_retry_count: first_failure.and_then(|result| result.direct_retry_count),
        completion_status: first_failure.and_then(|result| result.completion_status.clone()),
        completion_result: first_failure.and_then(|result| result.completion_result),
        completion_bytes_completed: first_failure
            .and_then(|result| result.completion_bytes_completed),
        completion_fault_addr: first_failure
            .and_then(|result| result.completion_fault_addr.clone()),
        results,
    }
}

fn top_level_failure_artifact(
    args: &CliArgs,
    failure_class: &'static str,
    error_kind: &'static str,
    validation_phase: Option<&'static str>,
    validation_error_kind: Option<&'static str>,
) -> BenchmarkArtifact {
    BenchmarkArtifact {
        schema_version: SCHEMA_VERSION,
        ok: false,
        verdict: "fail",
        device_path: args.device_path.display().to_string(),
        backend: Backend::Software.as_str(),
        claim_eligible: false,
        suite: args.suite.as_str(),
        runtime_flavor: "current_thread",
        worker_threads: 1,
        requested_bytes: args.bytes,
        iterations: args.iterations,
        concurrency: args.concurrency,
        duration_ms: args.duration_ms,
        failure_class: Some(failure_class),
        error_kind: Some(error_kind),
        direct_failure_kind: None,
        validation_phase,
        validation_error_kind,
        direct_retry_budget: None,
        direct_retry_count: None,
        completion_status: None,
        completion_result: None,
        completion_bytes_completed: None,
        completion_fault_addr: None,
        results: Vec::new(),
    }
}
