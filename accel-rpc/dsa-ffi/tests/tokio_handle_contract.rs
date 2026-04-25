use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use dsa_ffi::{
    AsyncDsaSession, AsyncLifecycleFailureKind, AsyncMemmoveError, AsyncMemmoveRequest,
    AsyncMemmoveWorker, MemmoveError, MemmoveRequest, MemmoveValidationReport,
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

#[tokio::test(flavor = "current_thread")]
async fn cloned_handles_share_one_worker_without_duplicating_ownership() {
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
    let first_handle = session.handle();
    let second_handle = first_handle.clone();

    let first = first_handle
        .memmove(AsyncMemmoveRequest::new(vec![1, 2, 3]).unwrap())
        .await
        .expect("first cloned handle should succeed");
    let second = second_handle
        .memmove(AsyncMemmoveRequest::new(vec![4, 5, 6, 7]).unwrap())
        .await
        .expect("second cloned handle should also succeed");

    assert_eq!(first.bytes, vec![1, 2, 3]);
    assert_eq!(second.bytes, vec![4, 5, 6, 7]);
    assert_eq!(calls.load(Ordering::SeqCst), 2);

    session.shutdown().expect("owner shutdown should succeed");
}

#[tokio::test(flavor = "current_thread")]
async fn dropping_one_clone_does_not_shut_down_another_clone() {
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
    let retained_handle = session.handle();
    let dropped_handle = retained_handle.clone();

    drop(dropped_handle);

    let result = retained_handle
        .memmove(AsyncMemmoveRequest::new(vec![8, 9]).unwrap())
        .await
        .expect("remaining clone should keep working");

    assert_eq!(result.bytes, vec![8, 9]);
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    session.shutdown().expect("owner shutdown should succeed");
}

#[tokio::test(flavor = "current_thread")]
async fn explicit_owner_shutdown_is_distinct_from_worker_failure() {
    let session = AsyncDsaSession::spawn_with_factory(|| {
        Ok(FakeWorker {
            calls: Arc::new(AtomicUsize::new(0)),
        })
    })
    .expect("worker should start");
    let handle = session.handle();

    session.shutdown().expect("owner shutdown should succeed");

    let err = handle
        .memmove(AsyncMemmoveRequest::new(vec![1, 2, 3]).unwrap())
        .await
        .expect_err("use after owner shutdown must fail structurally");

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
