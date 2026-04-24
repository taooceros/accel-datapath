use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use dsa_ffi::{
    AsyncDsaSession, AsyncMemmoveError, AsyncMemmoveRequest, AsyncMemmoveWorker,
    AsyncWorkerFailureKind, MemmoveError, MemmovePhase, MemmoveRequest, MemmoveValidationReport,
};

struct FakeWorker {
    calls: Arc<AtomicUsize>,
}

impl AsyncMemmoveWorker for FakeWorker {
    fn memmove(
        &mut self,
        dst: &mut [u8],
        src: &[u8],
    ) -> Result<MemmoveValidationReport, MemmoveError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        dst[..src.len()].copy_from_slice(src);
        MemmoveValidationReport::new("/dev/dsa/test0.0", MemmoveRequest::new(src.len())?, 0, 1)
    }
}

struct ErrorWorker {
    error: Option<MemmoveError>,
}

impl AsyncMemmoveWorker for ErrorWorker {
    fn memmove(
        &mut self,
        _dst: &mut [u8],
        _src: &[u8],
    ) -> Result<MemmoveValidationReport, MemmoveError> {
        Err(self
            .error
            .take()
            .expect("test should only send one request"))
    }
}

struct PanicWorker;

impl AsyncMemmoveWorker for PanicWorker {
    fn memmove(
        &mut self,
        _dst: &mut [u8],
        _src: &[u8],
    ) -> Result<MemmoveValidationReport, MemmoveError> {
        panic!("worker dropped before replying");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn async_wrapper_returns_owned_bytes_on_success() {
    let calls = Arc::new(AtomicUsize::new(0));
    let session = AsyncDsaSession::spawn_with_factory({
        let calls = Arc::clone(&calls);
        move || {
            Ok(FakeWorker {
                calls: Arc::clone(&calls),
            })
        }
    })
    .expect("worker should start");

    let result = session
        .memmove(AsyncMemmoveRequest::new(vec![1, 2, 3, 4]).expect("request should validate"))
        .await
        .expect("fake worker should succeed");

    assert_eq!(result.bytes, vec![1, 2, 3, 4]);
    assert_eq!(result.report.device_path.to_str(), Some("/dev/dsa/test0.0"));
    assert_eq!(result.report.requested_bytes, 4);
    assert_eq!(result.report.page_fault_retries, 0);
    assert_eq!(result.report.final_status, 1);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn rejects_zero_length_owned_requests_before_worker_dispatch() {
    let err = AsyncMemmoveRequest::new(Vec::new()).expect_err("zero-length requests should fail");

    assert!(matches!(
        err,
        MemmoveError::InvalidLength {
            requested_len: 0,
            ..
        }
    ));
}

#[test]
fn rejects_destination_size_mismatch_before_worker_dispatch() {
    let err = AsyncMemmoveRequest::with_destination_len(vec![1, 2, 3, 4], 3)
        .expect_err("destination sizing mismatches should fail before worker startup");

    assert!(matches!(
        err,
        MemmoveError::DestinationTooSmall {
            src_len: 4,
            dst_len: 3,
        }
    ));
}

#[test]
fn preserves_invalid_device_path_during_async_open() {
    let err = AsyncDsaSession::open("").expect_err("empty device paths should stay typed");

    assert_eq!(err.kind(), "invalid_device_path");
    assert!(matches!(
        err,
        AsyncMemmoveError::Memmove(MemmoveError::InvalidDevicePath { .. })
    ));
}

#[tokio::test(flavor = "current_thread")]
async fn reuses_one_worker_for_repeated_sequential_requests() {
    let calls = Arc::new(AtomicUsize::new(0));
    let session = AsyncDsaSession::spawn_with_factory({
        let calls = Arc::clone(&calls);
        move || {
            Ok(FakeWorker {
                calls: Arc::clone(&calls),
            })
        }
    })
    .expect("worker should start");

    let first = session
        .memmove(AsyncMemmoveRequest::new(vec![1, 2, 3]).unwrap())
        .await
        .expect("first request should succeed");
    let second = session
        .memmove(AsyncMemmoveRequest::new(vec![4, 5, 6, 7]).unwrap())
        .await
        .expect("second request should also succeed");

    assert_eq!(first.bytes, vec![1, 2, 3]);
    assert_eq!(second.bytes, vec![4, 5, 6, 7]);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn surfaces_worker_init_channel_failure_as_structural_error() {
    let err = AsyncDsaSession::spawn_with_factory(|| -> Result<FakeWorker, MemmoveError> {
        panic!("worker init panic for contract test");
    })
    .expect_err("worker init panic should surface structurally");

    assert_eq!(err.kind(), "worker_init_closed");
    assert_eq!(
        err.worker_failure_kind(),
        Some(AsyncWorkerFailureKind::WorkerInitClosed)
    );
    assert!(err.memmove_error().is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn surfaces_dropped_worker_reply_channel_as_structural_error() {
    let session =
        AsyncDsaSession::spawn_with_factory(|| Ok(PanicWorker)).expect("worker should start");

    let err = session
        .memmove(AsyncMemmoveRequest::new(vec![9, 9, 9]).unwrap())
        .await
        .expect_err("worker panic should close the reply channel");

    assert_eq!(err.kind(), "response_channel_closed");
    assert_eq!(
        err.worker_failure_kind(),
        Some(AsyncWorkerFailureKind::ResponseChannelClosed)
    );
    assert!(err.memmove_error().is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn preserves_underlying_completion_timeout_error() {
    let session = AsyncDsaSession::spawn_with_factory(|| {
        Ok(ErrorWorker {
            error: Some(MemmoveError::CompletionTimeout {
                device_path: PathBuf::from("/dev/dsa/test0.0"),
                phase: MemmovePhase::CompletionPoll,
                page_fault_retries: 2,
            }),
        })
    })
    .expect("worker should start");

    let err = session
        .memmove(AsyncMemmoveRequest::new(vec![1, 2, 3, 4]).unwrap())
        .await
        .expect_err("underlying memmove error should be preserved");

    assert_eq!(err.kind(), "completion_timeout");
    assert!(matches!(
        err,
        AsyncMemmoveError::Memmove(MemmoveError::CompletionTimeout {
            phase: MemmovePhase::CompletionPoll,
            page_fault_retries: 2,
            ..
        })
    ));
}

#[test]
fn shutdowns_cleanly_after_idle_state() {
    let calls = Arc::new(AtomicUsize::new(0));
    let session = AsyncDsaSession::spawn_with_factory({
        let calls = Arc::clone(&calls);
        move || {
            Ok(FakeWorker {
                calls: Arc::clone(&calls),
            })
        }
    })
    .expect("worker should start");

    session
        .shutdown()
        .expect("idle worker should shut down cleanly");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
