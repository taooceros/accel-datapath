use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Condvar, Mutex,
};
use std::time::Duration;

use dsa_ffi::{
    AsyncDsaSession, AsyncLifecycleFailureKind, AsyncMemmoveError, AsyncMemmoveRequest,
    AsyncMemmoveWorker, AsyncWorkerFailureKind, MemmoveError, MemmovePhase, MemmoveRequest,
    MemmoveValidationReport,
};
use tokio::sync::Notify;
use tokio::time::timeout;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkerEvent {
    Started(usize),
    Finished(usize),
}

#[derive(Default)]
struct ReleaseGate {
    released: Mutex<bool>,
    cv: Condvar,
}

impl ReleaseGate {
    fn wait(&self) {
        let mut released = self.released.lock().expect("gate lock should not poison");
        while !*released {
            released = self
                .cv
                .wait(released)
                .expect("gate lock should not poison");
        }
    }

    fn release(&self) {
        let mut released = self.released.lock().expect("gate lock should not poison");
        *released = true;
        self.cv.notify_all();
    }
}

#[derive(Default)]
struct EventLog {
    events: Mutex<Vec<WorkerEvent>>,
    notify: Notify,
}

impl EventLog {
    fn push(&self, event: WorkerEvent) {
        self.events
            .lock()
            .expect("event log lock should not poison")
            .push(event);
        self.notify.notify_waiters();
    }

    fn snapshot(&self) -> Vec<WorkerEvent> {
        self.events
            .lock()
            .expect("event log lock should not poison")
            .clone()
    }

    async fn wait_for_event(&self, expected: WorkerEvent) {
        timeout(Duration::from_secs(1), async {
            loop {
                if self.snapshot().contains(&expected) {
                    return;
                }
                self.notify.notified().await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("timed out waiting for worker event {expected:?}"));
    }

    async fn assert_event_absent_for(&self, unexpected: WorkerEvent, duration: Duration) {
        let result = timeout(duration, async {
            loop {
                if self.snapshot().contains(&unexpected) {
                    return;
                }
                self.notify.notified().await;
            }
        })
        .await;

        assert!(
            result.is_err(),
            "unexpected worker event {unexpected:?} appeared: {:?}",
            self.snapshot()
        );
    }
}

struct BlockingWorker {
    calls: Arc<AtomicUsize>,
    events: Arc<EventLog>,
    first_release: Arc<ReleaseGate>,
}

impl AsyncMemmoveWorker for BlockingWorker {
    fn memmove(
        &mut self,
        dst: &mut [u8],
        src: &[u8],
    ) -> Result<MemmoveValidationReport, MemmoveError> {
        let call_id = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        self.events.push(WorkerEvent::Started(call_id));

        if call_id == 1 {
            self.first_release.wait();
        }

        dst[..src.len()].copy_from_slice(src);
        self.events.push(WorkerEvent::Finished(call_id));

        MemmoveValidationReport::new("/dev/dsa/test0.0", MemmoveRequest::new(src.len())?, 0, 1)
    }
}

struct BlockingWorkerHarness {
    calls: Arc<AtomicUsize>,
    factory_calls: Arc<AtomicUsize>,
    events: Arc<EventLog>,
    first_release: Arc<ReleaseGate>,
}

impl BlockingWorkerHarness {
    fn spawn_session() -> (AsyncDsaSession, Self) {
        let calls = Arc::new(AtomicUsize::new(0));
        let factory_calls = Arc::new(AtomicUsize::new(0));
        let events = Arc::new(EventLog::default());
        let first_release = Arc::new(ReleaseGate::default());

        let session = AsyncDsaSession::spawn_with_factory({
            let calls = Arc::clone(&calls);
            let factory_calls = Arc::clone(&factory_calls);
            let events = Arc::clone(&events);
            let first_release = Arc::clone(&first_release);
            move || {
                factory_calls.fetch_add(1, Ordering::SeqCst);
                Ok(BlockingWorker {
                    calls: Arc::clone(&calls),
                    events: Arc::clone(&events),
                    first_release: Arc::clone(&first_release),
                })
            }
        })
        .expect("worker should start");

        (
            session,
            Self {
                calls,
                factory_calls,
                events,
                first_release,
            },
        )
    }

    async fn wait_for_first_start(&self) {
        self.events.wait_for_event(WorkerEvent::Started(1)).await;
    }

    async fn wait_for_finish(&self, call_id: usize) {
        self.events.wait_for_event(WorkerEvent::Finished(call_id)).await;
    }

    async fn wait_for_start(&self, call_id: usize) {
        self.events.wait_for_event(WorkerEvent::Started(call_id)).await;
    }

    async fn assert_second_request_stays_queued_until_release(&self) {
        self.events
            .assert_event_absent_for(WorkerEvent::Started(2), Duration::from_millis(100))
            .await;
    }

    fn release_first_request(&self) {
        self.first_release.release();
    }

    fn assert_calls(&self, expected: usize) {
        assert_eq!(
            self.calls.load(Ordering::SeqCst),
            expected,
            "worker should execute the expected number of requests"
        );
        assert_eq!(
            self.factory_calls.load(Ordering::SeqCst),
            1,
            "contract tests should still use one worker-owned session"
        );
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

#[tokio::test(flavor = "current_thread")]
async fn aborting_after_enqueue_does_not_cancel_worker_and_follow_up_still_succeeds() {
    let (session, harness) = BlockingWorkerHarness::spawn_session();
    let handle = session.handle();
    let aborted_handle = handle.clone();

    let aborted_task = tokio::spawn(async move {
        aborted_handle
            .memmove(AsyncMemmoveRequest::new(vec![1, 2, 3]).unwrap())
            .await
    });

    harness.wait_for_first_start().await;
    aborted_task.abort();
    let join_err = aborted_task
        .await
        .expect_err("aborted awaiter should report task cancellation");
    assert!(join_err.is_cancelled(), "aborting the awaiter should stay at the Tokio task boundary");

    harness.release_first_request();
    harness.wait_for_finish(1).await;
    harness.assert_calls(1);

    let follow_up = handle
        .memmove(AsyncMemmoveRequest::new(vec![4, 5, 6, 7]).unwrap())
        .await
        .expect("abandoned reply must not poison later work");

    harness.wait_for_start(2).await;
    harness.wait_for_finish(2).await;
    harness.assert_calls(2);
    assert_eq!(follow_up.bytes, vec![4, 5, 6, 7]);
    assert_eq!(
        harness.events.snapshot(),
        vec![
            WorkerEvent::Started(1),
            WorkerEvent::Finished(1),
            WorkerEvent::Started(2),
            WorkerEvent::Finished(2),
        ],
        "worker should drain the aborted request and keep later submissions healthy"
    );

    session.shutdown().expect("owner shutdown should succeed");
}

#[tokio::test(flavor = "current_thread")]
async fn shutdown_drains_queued_work_before_refusing_new_submissions() {
    let (session, harness) = BlockingWorkerHarness::spawn_session();
    let first_handle = session.handle();
    let second_handle = first_handle.clone();
    let post_shutdown_handle = first_handle.clone();

    let first_task = tokio::spawn(async move {
        first_handle
            .memmove(AsyncMemmoveRequest::new(vec![8, 9]).unwrap())
            .await
    });

    harness.wait_for_first_start().await;

    let second_task = tokio::spawn(async move {
        second_handle
            .memmove(AsyncMemmoveRequest::new(vec![10, 11, 12]).unwrap())
            .await
    });

    harness.assert_second_request_stays_queued_until_release().await;

    let shutdown_thread = std::thread::spawn(move || session.shutdown());
    std::thread::sleep(Duration::from_millis(20));
    assert!(
        !shutdown_thread.is_finished(),
        "shutdown should wait for already-queued work instead of cutting the queue short"
    );

    harness.release_first_request();

    let first = timeout(Duration::from_secs(1), first_task)
        .await
        .expect("first queued request should finish after release")
        .expect("first task should not panic")
        .expect("first queued request should succeed");
    let second = timeout(Duration::from_secs(1), second_task)
        .await
        .expect("second queued request should drain before shutdown completes")
        .expect("second task should not panic")
        .expect("second queued request should succeed");
    shutdown_thread
        .join()
        .expect("shutdown thread should not panic")
        .expect("shutdown should complete after queued work drains");

    assert_eq!(first.bytes, vec![8, 9]);
    assert_eq!(second.bytes, vec![10, 11, 12]);
    harness.assert_calls(2);
    assert_eq!(
        harness.events.snapshot(),
        vec![
            WorkerEvent::Started(1),
            WorkerEvent::Finished(1),
            WorkerEvent::Started(2),
            WorkerEvent::Finished(2),
        ],
        "queued work should drain before the worker observes shutdown"
    );

    let err = post_shutdown_handle
        .memmove(AsyncMemmoveRequest::new(vec![13, 14, 15]).unwrap())
        .await
        .expect_err("new submissions after shutdown must fail with a lifecycle error");

    assert_eq!(err.kind(), "owner_shutdown");
    assert_eq!(
        err.lifecycle_failure_kind(),
        Some(AsyncLifecycleFailureKind::OwnerShutdown)
    );
    assert!(matches!(
        err,
        AsyncMemmoveError::LifecycleFailure {
            kind: AsyncLifecycleFailureKind::OwnerShutdown,
        }
    ));
    assert!(err.worker_failure_kind().is_none());
    assert!(err.memmove_error().is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn handle_use_after_explicit_shutdown_is_a_lifecycle_error() {
    let session = AsyncDsaSession::spawn_with_factory(|| {
        Ok(FakeWorker {
            calls: Arc::new(AtomicUsize::new(0)),
        })
    })
    .expect("worker should start");
    let handle = session.handle();

    session
        .shutdown()
        .expect("idle worker should shut down cleanly");

    let err = handle
        .memmove(AsyncMemmoveRequest::new(vec![1, 2, 3]).unwrap())
        .await
        .expect_err("shut down owners must reject cloned handle use");

    assert_eq!(err.kind(), "owner_shutdown");
    assert_eq!(
        err.lifecycle_failure_kind(),
        Some(AsyncLifecycleFailureKind::OwnerShutdown)
    );
    assert!(matches!(
        err,
        AsyncMemmoveError::LifecycleFailure {
            kind: AsyncLifecycleFailureKind::OwnerShutdown,
        }
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
