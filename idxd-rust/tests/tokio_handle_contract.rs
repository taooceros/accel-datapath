use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use bytes::{Bytes, BytesMut, buf::UninitSlice};
use idxd_rust::{
    AsyncDsaSession, AsyncLifecycleFailureKind, AsyncMemmoveError, AsyncMemmoveRequest,
    AsyncMemmoveWorker, MemmoveError, MemmoveRequest, MemmoveValidationReport,
};
use tokio::sync::Notify;
use tokio::time::timeout;

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
            "unexpected worker event {unexpected:?} appeared while request 1 was still blocked: {:?}",
            self.snapshot()
        );
    }
}

struct BlockingWorker {
    calls: Arc<AtomicUsize>,
    active_calls: Arc<AtomicUsize>,
    max_active: Arc<AtomicUsize>,
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
        let active = self.active_calls.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_active.fetch_max(active, Ordering::SeqCst);
        self.events.push(WorkerEvent::Started(call_id));

        if call_id == 1 {
            self.first_release.wait();
        }

        dst.copy_from_slice(src);
        self.events.push(WorkerEvent::Finished(call_id));
        self.active_calls.fetch_sub(1, Ordering::SeqCst);

        MemmoveValidationReport::new("/dev/dsa/test0.0", MemmoveRequest::new(src.len())?, 0, 1)
    }
}

struct BlockingWorkerHarness {
    calls: Arc<AtomicUsize>,
    factory_calls: Arc<AtomicUsize>,
    max_active: Arc<AtomicUsize>,
    events: Arc<EventLog>,
    first_release: Arc<ReleaseGate>,
}

impl BlockingWorkerHarness {
    fn spawn_session() -> (AsyncDsaSession, Self) {
        let calls = Arc::new(AtomicUsize::new(0));
        let factory_calls = Arc::new(AtomicUsize::new(0));
        let active_calls = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));
        let events = Arc::new(EventLog::default());
        let first_release = Arc::new(ReleaseGate::default());

        let session = AsyncDsaSession::spawn_with_factory({
            let calls = Arc::clone(&calls);
            let factory_calls = Arc::clone(&factory_calls);
            let active_calls = Arc::clone(&active_calls);
            let max_active = Arc::clone(&max_active);
            let events = Arc::clone(&events);
            let first_release = Arc::clone(&first_release);
            move || {
                factory_calls.fetch_add(1, Ordering::SeqCst);
                Ok(BlockingWorker {
                    calls: Arc::clone(&calls),
                    active_calls: Arc::clone(&active_calls),
                    max_active: Arc::clone(&max_active),
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
                max_active,
                events,
                first_release,
            },
        )
    }

    async fn wait_for_first_start(&self) {
        self.events.wait_for_event(WorkerEvent::Started(1)).await;
    }

    async fn assert_second_request_stays_queued_until_release(&self) {
        self.events
            .assert_event_absent_for(WorkerEvent::Started(2), Duration::from_millis(100))
            .await;
    }

    fn release_first_request(&self) {
        self.first_release.release();
    }

    fn assert_serialized(&self) {
        assert_eq!(
            self.factory_calls.load(Ordering::SeqCst),
            1,
            "cloned handles should share one worker-owned session"
        );
        assert_eq!(
            self.calls.load(Ordering::SeqCst),
            2,
            "both overlapped requests should reach the same worker"
        );
        assert_eq!(
            self.max_active.load(Ordering::SeqCst),
            1,
            "single-worker contract should never execute more than one request at once"
        );
        assert_eq!(
            self.events.snapshot(),
            vec![
                WorkerEvent::Started(1),
                WorkerEvent::Finished(1),
                WorkerEvent::Started(2),
                WorkerEvent::Finished(2),
            ],
            "worker event order should make queue serialization explicit"
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
async fn cloned_handles_compose_with_tokio_join_and_serialize_through_one_worker() {
    let (session, harness) = BlockingWorkerHarness::spawn_session();
    let first_handle = session.handle();
    let second_handle = first_handle.clone();

    let joined = tokio::spawn(async move {
        tokio::join!(
            first_handle.memmove(owned_request(b"\x01\x02\x03")),
            second_handle.memmove(owned_request(b"\x04\x05\x06\x07"))
        )
    });

    harness.wait_for_first_start().await;
    harness
        .assert_second_request_stays_queued_until_release()
        .await;
    assert!(
        !joined.is_finished(),
        "join! composition should still be waiting on the blocked first request"
    );

    harness.release_first_request();

    let (first, second) = timeout(Duration::from_secs(1), joined)
        .await
        .expect("join! composition should complete after release")
        .expect("join! driver task should not panic");
    let first = first.expect("first cloned handle should succeed");
    let second = second.expect("second cloned handle should succeed");

    assert_eq!(first.destination, vec![1, 2, 3]);
    assert_eq!(second.destination, vec![4, 5, 6, 7]);
    harness.assert_serialized();

    session.shutdown().expect("owner shutdown should succeed");
}

#[tokio::test(flavor = "current_thread")]
async fn cloned_handles_compose_in_spawned_tasks_and_still_share_one_worker() {
    let (session, harness) = BlockingWorkerHarness::spawn_session();
    let first_handle = session.handle();
    let second_handle = first_handle.clone();

    let first_task =
        tokio::spawn(async move { first_handle.memmove(owned_request(b"\x08\x09")).await });
    let second_task =
        tokio::spawn(async move { second_handle.memmove(owned_request(b"\x0a\x0b\x0c")).await });

    harness.wait_for_first_start().await;
    harness
        .assert_second_request_stays_queued_until_release()
        .await;
    assert!(
        !first_task.is_finished() && !second_task.is_finished(),
        "spawned composition should stay pending until the blocked worker call is released"
    );

    harness.release_first_request();

    let (first, second) = timeout(Duration::from_secs(1), async {
        (
            first_task
                .await
                .expect("first spawned task should not panic"),
            second_task
                .await
                .expect("second spawned task should not panic"),
        )
    })
    .await
    .expect("spawned-task composition should complete after release");

    let first = first.expect("first spawned handle should succeed");
    let second = second.expect("second spawned handle should succeed");

    assert_eq!(first.destination, vec![8, 9]);
    assert_eq!(second.destination, vec![10, 11, 12]);
    harness.assert_serialized();

    session.shutdown().expect("owner shutdown should succeed");
}

#[tokio::test(flavor = "current_thread")]
async fn dropping_one_clone_does_not_shut_down_another_clone() {
    let (session, harness) = BlockingWorkerHarness::spawn_session();
    let retained_handle = session.handle();
    let dropped_handle = retained_handle.clone();

    drop(dropped_handle);
    harness.release_first_request();

    let result = retained_handle
        .memmove(owned_request(b"\x08\x09"))
        .await
        .expect("remaining clone should keep working");

    assert_eq!(result.destination, vec![8, 9]);

    session.shutdown().expect("owner shutdown should succeed");
}

#[tokio::test(flavor = "current_thread")]
async fn explicit_owner_shutdown_is_distinct_from_worker_failure() {
    let (session, _harness) = BlockingWorkerHarness::spawn_session();
    let handle = session.handle();

    session.shutdown().expect("owner shutdown should succeed");

    let err = handle
        .memmove(owned_request(b"\x01\x02\x03"))
        .await
        .expect_err("use after owner shutdown must fail structurally");

    assert_eq!(err.kind(), "owner_shutdown");
    assert_eq!(
        err.lifecycle_failure_kind(),
        Some(AsyncLifecycleFailureKind::OwnerShutdown)
    );
    assert!(err.worker_failure_kind().is_none());
    assert!(err.memmove_error().is_none());
    assert!(err.into_request().is_some());
}
