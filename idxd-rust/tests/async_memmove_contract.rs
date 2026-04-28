use std::path::PathBuf;
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use bytes::{Bytes, BytesMut, buf::UninitSlice};
use idxd_rust::{
    AsyncDirectFailureKind, AsyncDsaSession, AsyncLifecycleFailureKind, AsyncMemmoveRequest,
    AsyncMemmoveWorker, AsyncWorkerFailureKind, CompletionSnapshot, DirectAsyncMemmoveRuntime,
    MemmoveError, MemmovePhase, MemmoveRequest, MemmoveValidationConfig, MemmoveValidationReport,
    direct_test_support::ScriptedDirectBackend,
};
use idxd_sys::{DSA_COMP_PAGE_FAULT_NOBOF, DSA_COMP_SUCCESS, EnqcmdSubmission};
use tokio::sync::Notify;
use tokio::time::timeout;

struct FakeWorker {
    calls: Arc<AtomicUsize>,
}

impl AsyncMemmoveWorker for FakeWorker {
    fn memmove(
        &mut self,
        dst: &mut UninitSlice,
        src: &[u8],
    ) -> Result<MemmoveValidationReport, MemmoveError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        dst.copy_from_slice(src);
        MemmoveValidationReport::new("/dev/dsa/test0.0", MemmoveRequest::new(src.len())?, 0, 1)
    }
}

struct ErrorWorker {
    error: Option<MemmoveError>,
}

impl AsyncMemmoveWorker for ErrorWorker {
    fn memmove(
        &mut self,
        _dst: &mut UninitSlice,
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
        _dst: &mut UninitSlice,
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
            released = self.cv.wait(released).expect("gate lock should not poison");
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
        dst: &mut UninitSlice,
        src: &[u8],
    ) -> Result<MemmoveValidationReport, MemmoveError> {
        let call_id = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        self.events.push(WorkerEvent::Started(call_id));

        if call_id == 1 {
            self.first_release.wait();
        }

        dst.copy_from_slice(src);
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
        self.events
            .wait_for_event(WorkerEvent::Finished(call_id))
            .await;
    }

    async fn wait_for_start(&self, call_id: usize) {
        self.events
            .wait_for_event(WorkerEvent::Started(call_id))
            .await;
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

fn owned_request(source: &'static [u8]) -> AsyncMemmoveRequest {
    AsyncMemmoveRequest::new(
        Bytes::from_static(source),
        BytesMut::with_capacity(source.len()),
    )
    .expect("request should validate")
}

#[tokio::test(flavor = "current_thread")]
async fn async_wrapper_returns_owned_destination_on_success() {
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
        .memmove(owned_request(b"\x01\x02\x03\x04"))
        .await
        .expect("fake worker should succeed");

    assert_eq!(result.destination.as_ref(), &[1, 2, 3, 4]);
    assert_eq!(result.report.device_path.to_str(), Some("/dev/dsa/test0.0"));
    assert_eq!(result.report.requested_bytes, 4);
    assert_eq!(result.report.page_fault_retries, 0);
    assert_eq!(result.report.final_status, 1);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn appends_to_destination_spare_capacity_after_existing_prefix() {
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

    let mut destination = BytesMut::from(&b"prefix:"[..]);
    destination.reserve(4);
    let request = AsyncMemmoveRequest::new(Bytes::from_static(b"data"), destination)
        .expect("destination spare capacity should validate");

    assert_eq!(request.requested_bytes(), 4);
    assert_eq!(request.destination_len(), 7);

    let result = session
        .memmove(request)
        .await
        .expect("fake worker should succeed");

    assert_eq!(&result.destination[..], b"prefix:data");
    assert_eq!(result.report.requested_bytes, 4);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn rejects_zero_length_owned_requests_before_worker_dispatch() {
    let err = AsyncMemmoveRequest::new(Bytes::new(), BytesMut::with_capacity(4))
        .expect_err("zero-length requests should fail");

    assert!(matches!(
        err.memmove_error(),
        MemmoveError::InvalidLength {
            requested_len: 0,
            ..
        }
    ));
    let (_error, source, destination) = err.into_parts();
    assert!(source.is_empty());
    assert_eq!(destination.capacity(), 4);
}

#[test]
fn rejects_destination_size_mismatch_before_worker_dispatch() {
    let err = AsyncMemmoveRequest::new(Bytes::from_static(b"data"), BytesMut::with_capacity(3))
        .expect_err("destination sizing mismatches should fail before worker startup");

    assert!(matches!(
        err.memmove_error(),
        MemmoveError::DestinationTooSmall { .. }
    ));
    let (_error, source, destination) = err.into_parts();
    assert_eq!(&source[..], b"data");
    assert_eq!(destination.capacity(), 3);
}

#[test]
fn preserves_invalid_device_path_during_async_open() {
    let err = AsyncDsaSession::open("").expect_err("empty device paths should stay typed");

    assert_eq!(err.kind(), "invalid_device_path");
    assert!(matches!(
        err.memmove_error(),
        Some(MemmoveError::InvalidDevicePath { .. })
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
        .memmove(owned_request(b"\x01\x02\x03"))
        .await
        .expect("first request should succeed");
    let second = session
        .memmove(owned_request(b"\x04\x05\x06\x07"))
        .await
        .expect("second request should also succeed");

    assert_eq!(first.destination.as_ref(), &[1, 2, 3]);
    assert_eq!(second.destination.as_ref(), &[4, 5, 6, 7]);
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
        .memmove(owned_request(b"\x09\x09\x09"))
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
        .memmove(owned_request(b"\x01\x02\x03\x04"))
        .await
        .expect_err("underlying memmove error should be preserved");

    assert_eq!(err.kind(), "completion_timeout");
    assert!(matches!(
        err.memmove_error(),
        Some(MemmoveError::CompletionTimeout {
            phase: MemmovePhase::CompletionPoll,
            page_fault_retries: 2,
            ..
        })
    ));
    let recovered = err
        .into_request()
        .expect("execution errors should recover owned buffers");
    assert_eq!(recovered.destination_len(), 0);
    let (source, destination) = recovered.into_parts();
    assert_eq!(&source[..], b"\x01\x02\x03\x04");
    assert_eq!(destination.len(), 0);
}

#[tokio::test(flavor = "current_thread")]
async fn aborting_after_enqueue_does_not_cancel_worker_and_follow_up_still_succeeds() {
    let (session, harness) = BlockingWorkerHarness::spawn_session();
    let handle = session.handle();
    let aborted_handle = handle.clone();

    let aborted_task =
        tokio::spawn(async move { aborted_handle.memmove(owned_request(b"\x01\x02\x03")).await });

    harness.wait_for_first_start().await;
    aborted_task.abort();
    let join_err = aborted_task
        .await
        .expect_err("aborted awaiter should report task cancellation");
    assert!(
        join_err.is_cancelled(),
        "aborting the awaiter should stay at the Tokio task boundary"
    );

    harness.release_first_request();
    harness.wait_for_finish(1).await;
    harness.assert_calls(1);

    let follow_up = handle
        .memmove(owned_request(b"\x04\x05\x06\x07"))
        .await
        .expect("abandoned reply must not poison later work");

    harness.wait_for_start(2).await;
    harness.wait_for_finish(2).await;
    harness.assert_calls(2);
    assert_eq!(follow_up.destination.as_ref(), &[4, 5, 6, 7]);
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

    let first_task =
        tokio::spawn(async move { first_handle.memmove(owned_request(b"\x08\x09")).await });

    harness.wait_for_first_start().await;

    let second_task =
        tokio::spawn(async move { second_handle.memmove(owned_request(b"\x0a\x0b\x0c")).await });

    harness
        .assert_second_request_stays_queued_until_release()
        .await;

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

    assert_eq!(first.destination.as_ref(), &[8, 9]);
    assert_eq!(second.destination.as_ref(), &[10, 11, 12]);
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
        .memmove(owned_request(b"\x0d\x0e\x0f"))
        .await
        .expect_err("new submissions after shutdown must fail with a lifecycle error");

    assert_eq!(err.kind(), "owner_shutdown");
    assert_eq!(
        err.lifecycle_failure_kind(),
        Some(AsyncLifecycleFailureKind::OwnerShutdown)
    );
    assert!(err.worker_failure_kind().is_none());
    assert!(err.memmove_error().is_none());
    let recovered = err
        .into_request()
        .expect("pre-enqueue lifecycle errors should recover owned request");
    assert_eq!(recovered.requested_bytes(), 3);
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
        .memmove(owned_request(b"\x01\x02\x03"))
        .await
        .expect_err("shut down owners must reject cloned handle use");

    assert_eq!(err.kind(), "owner_shutdown");
    assert_eq!(
        err.lifecycle_failure_kind(),
        Some(AsyncLifecycleFailureKind::OwnerShutdown)
    );
    let recovered = err
        .into_request()
        .expect("pre-enqueue lifecycle errors should recover owned request");
    assert_eq!(recovered.requested_bytes(), 3);
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

fn direct_config() -> MemmoveValidationConfig {
    MemmoveValidationConfig::new("/dev/dsa/test0.0").expect("direct test config")
}

fn owned_mut_request(source: &'static [u8]) -> AsyncMemmoveRequest {
    AsyncMemmoveRequest::new(
        Bytes::from_static(source),
        BytesMut::with_capacity(source.len()),
    )
    .expect("request should validate")
}

#[tokio::test(flavor = "current_thread")]
async fn completion_record_drives_direct_async_completion() {
    let backend = ScriptedDirectBackend::new();
    let runtime = DirectAsyncMemmoveRuntime::new(direct_config(), backend.clone());

    let pending = tokio::spawn({
        let runtime = runtime.clone();
        async move { runtime.memmove(owned_mut_request(b"complete")).await }
    });

    tokio::task::yield_now().await;
    assert_eq!(backend.submissions(), 1);
    assert!(
        timeout(Duration::from_millis(50), async {
            while !pending.is_finished() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .is_err(),
        "direct future must wait for a completion snapshot, not submit success"
    );

    backend.complete(1, CompletionSnapshot::new(DSA_COMP_SUCCESS, 0, 0, 0));

    let result = timeout(Duration::from_secs(1), pending)
        .await
        .expect("completion snapshot should wake the direct future")
        .expect("direct task should not panic")
        .expect("successful completion should produce a result");

    assert_eq!(result.destination.as_ref(), b"complete");
    assert_eq!(result.report.requested_bytes, 8);
    assert_eq!(result.report.final_status, DSA_COMP_SUCCESS);
    assert_eq!(backend.completions(), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn backpressure_exhaustion_reports_retry_budget_without_payload_bytes() {
    let backend = ScriptedDirectBackend::with_submissions([
        EnqcmdSubmission::Rejected,
        EnqcmdSubmission::Rejected,
        EnqcmdSubmission::Rejected,
    ]);
    let runtime =
        DirectAsyncMemmoveRuntime::with_submission_retry_budget(direct_config(), backend, 2);

    let err = runtime
        .memmove(owned_mut_request(b"secret-payload"))
        .await
        .expect_err("bounded rejection should fail instead of spinning forever");

    assert_eq!(err.kind(), "backpressure_exceeded");
    assert_eq!(
        err.direct_failure_kind(),
        Some(AsyncDirectFailureKind::BackpressureExceeded)
    );
    let failure = err
        .direct_failure()
        .expect("direct metadata should be present");
    assert_eq!(failure.requested_bytes(), 14);
    assert_eq!(failure.retry_budget(), 2);
    assert_eq!(failure.retry_count(), 3);
    let message = failure.to_string();
    assert!(err.into_request().is_some());
    assert!(!message.contains("secret-payload"));
    assert!(!message.contains("115, 101, 99"));
}

#[tokio::test(flavor = "current_thread")]
async fn monitor_close_resolves_accepted_direct_operation() {
    let backend = ScriptedDirectBackend::new();
    let runtime = DirectAsyncMemmoveRuntime::new(direct_config(), backend);

    let pending = tokio::spawn({
        let runtime = runtime.clone();
        async move { runtime.memmove(owned_mut_request(b"monitor")).await }
    });

    tokio::task::yield_now().await;
    runtime.close();

    let err = timeout(Duration::from_secs(1), pending)
        .await
        .expect("monitor closure should resolve accepted pending work")
        .expect("direct task should not panic")
        .expect_err("closed monitor should fail the request");

    assert_eq!(err.kind(), "monitor_closed");
    assert_eq!(
        err.direct_failure_kind(),
        Some(AsyncDirectFailureKind::MonitorClosed)
    );
    assert_eq!(
        err.direct_failure()
            .expect("direct failure metadata")
            .requested_bytes(),
        7
    );
}

#[tokio::test(flavor = "current_thread")]
async fn dropped_direct_receiver_does_not_remove_accepted_operation_before_completion() {
    let backend = ScriptedDirectBackend::new();
    let runtime = DirectAsyncMemmoveRuntime::new(direct_config(), backend.clone());

    let pending = tokio::spawn({
        let runtime = runtime.clone();
        async move { runtime.memmove(owned_mut_request(b"drop-me")).await }
    });

    tokio::task::yield_now().await;
    assert_eq!(backend.submissions(), 1);
    pending.abort();
    let join_err = pending
        .await
        .expect_err("aborted direct awaiter should report task cancellation");
    assert!(join_err.is_cancelled());

    backend.complete(1, CompletionSnapshot::new(DSA_COMP_SUCCESS, 0, 0, 0));
    timeout(Duration::from_secs(1), async {
        while backend.completions() == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("monitor should keep operation-owned buffers alive after receiver drop");
}

#[tokio::test(flavor = "current_thread")]
async fn malformed_direct_completion_surfaces_memmove_snapshot_metadata() {
    let backend = ScriptedDirectBackend::new();
    let runtime = DirectAsyncMemmoveRuntime::new(direct_config(), backend.clone());

    let pending = tokio::spawn({
        let runtime = runtime.clone();
        async move { runtime.memmove(owned_mut_request(b"bad")).await }
    });

    tokio::task::yield_now().await;
    backend.complete(
        1,
        CompletionSnapshot::new(DSA_COMP_PAGE_FAULT_NOBOF, 0, 64, 0x1000),
    );

    let err = timeout(Duration::from_secs(1), pending)
        .await
        .expect("malformed completion should resolve")
        .expect("direct task should not panic")
        .expect_err("malformed completion should fail");

    assert_eq!(err.kind(), "malformed_completion");
    assert!(matches!(
        err.memmove_error(),
        Some(MemmoveError::MalformedCompletion {
            phase: MemmovePhase::PageFaultRetry,
            bytes_completed: 64,
            fault_addr: 0x1000,
            ..
        })
    ));
}

#[tokio::test(flavor = "current_thread")]
async fn concurrent_direct_requests_complete_out_of_order() {
    let backend = ScriptedDirectBackend::new();
    let runtime = DirectAsyncMemmoveRuntime::new(direct_config(), backend.clone());

    let first = tokio::spawn({
        let runtime = runtime.clone();
        async move { runtime.memmove(owned_mut_request(b"first")).await }
    });
    let second = tokio::spawn({
        let runtime = runtime.clone();
        async move { runtime.memmove(owned_mut_request(b"second")).await }
    });

    tokio::task::yield_now().await;
    assert_eq!(backend.submissions(), 2);
    backend.complete(2, CompletionSnapshot::new(DSA_COMP_SUCCESS, 0, 0, 0));

    let second_result = timeout(Duration::from_secs(1), second)
        .await
        .expect("second request should complete first")
        .expect("second task should not panic")
        .expect("second request should succeed");
    assert_eq!(second_result.destination.as_ref(), b"second");
    assert!(!first.is_finished());

    backend.complete(1, CompletionSnapshot::new(DSA_COMP_SUCCESS, 0, 0, 0));
    let first_result = timeout(Duration::from_secs(1), first)
        .await
        .expect("first request should complete after its own snapshot")
        .expect("first task should not panic")
        .expect("first request should succeed");
    assert_eq!(first_result.destination.as_ref(), b"first");
}
