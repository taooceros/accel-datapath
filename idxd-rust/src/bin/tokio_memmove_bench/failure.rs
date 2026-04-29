use idxd_rust::{AsyncMemmoveError, MemmoveError, MemmovePhase};

#[derive(Debug, Clone)]
pub(crate) struct RowFailure {
    pub(crate) failure_class: &'static str,
    pub(crate) error_kind: &'static str,
    pub(crate) direct_failure_kind: Option<&'static str>,
    pub(crate) validation_phase: Option<&'static str>,
    pub(crate) validation_error_kind: Option<&'static str>,
    pub(crate) direct_retry_budget: Option<u32>,
    pub(crate) direct_retry_count: Option<u32>,
    pub(crate) completion_status: Option<String>,
    pub(crate) completion_result: Option<u8>,
    pub(crate) completion_bytes_completed: Option<u32>,
    pub(crate) completion_fault_addr: Option<String>,
}

impl RowFailure {
    pub(crate) fn request(error_kind: &'static str) -> Self {
        Self {
            failure_class: "validation",
            error_kind,
            direct_failure_kind: None,
            validation_phase: Some("request_construction"),
            validation_error_kind: Some(error_kind),
            direct_retry_budget: None,
            direct_retry_count: None,
            completion_status: None,
            completion_result: None,
            completion_bytes_completed: None,
            completion_fault_addr: None,
        }
    }

    pub(crate) fn async_error(error: &AsyncMemmoveError) -> Self {
        let direct_failure_kind = error.direct_failure_kind().map(|kind| kind.as_str());
        let direct_failure = error.direct_failure();
        let completion_snapshot = direct_failure.and_then(|failure| failure.completion_snapshot());
        let failure_class = if direct_failure_kind.is_some() {
            "async_direct"
        } else if error.lifecycle_failure_kind().is_some() {
            "async_lifecycle"
        } else if error.worker_failure_kind().is_some() {
            "async_worker"
        } else if error
            .memmove_error()
            .is_some_and(|err| err.kind() == "queue_open")
        {
            "queue_open"
        } else {
            "memmove"
        };
        Self {
            failure_class,
            error_kind: error.kind(),
            direct_failure_kind,
            validation_phase: error
                .memmove_error()
                .and_then(|err| err.phase())
                .map(phase_name),
            validation_error_kind: error.memmove_error().map(|err| err.kind()),
            direct_retry_budget: direct_failure.map(|failure| failure.retry_budget()),
            direct_retry_count: direct_failure.map(|failure| failure.retry_count()),
            completion_status: completion_snapshot.map(|snapshot| hex_status(snapshot.status)),
            completion_result: completion_snapshot.map(|snapshot| snapshot.result),
            completion_bytes_completed: completion_snapshot
                .map(|snapshot| snapshot.bytes_completed),
            completion_fault_addr: completion_snapshot.map(|snapshot| hex_u64(snapshot.fault_addr)),
        }
    }

    pub(crate) fn sync_error(error: &MemmoveError, failure_class: &'static str) -> Self {
        Self {
            failure_class,
            error_kind: error.kind(),
            direct_failure_kind: None,
            validation_phase: error.phase().map(phase_name),
            validation_error_kind: Some(error.kind()),
            direct_retry_budget: None,
            direct_retry_count: error.page_fault_retries(),
            completion_status: error.final_status().map(hex_status),
            completion_result: None,
            completion_bytes_completed: None,
            completion_fault_addr: None,
        }
    }

    pub(crate) fn join_error() -> Self {
        Self {
            failure_class: "tokio_join",
            error_kind: "join_error",
            direct_failure_kind: None,
            validation_phase: None,
            validation_error_kind: None,
            direct_retry_budget: None,
            direct_retry_count: None,
            completion_status: None,
            completion_result: None,
            completion_bytes_completed: None,
            completion_fault_addr: None,
        }
    }
}

fn phase_name(phase: MemmovePhase) -> &'static str {
    match phase {
        MemmovePhase::QueueOpen => "queue_open",
        MemmovePhase::CompletionPoll => "completion_poll",
        MemmovePhase::PageFaultRetry => "page_fault_retry",
        MemmovePhase::PostCopyVerify => "post_copy_verify",
    }
}

fn hex_status(status: u8) -> String {
    format!("0x{status:02x}")
}

fn hex_u64(value: u64) -> String {
    format!("0x{value:x}")
}
