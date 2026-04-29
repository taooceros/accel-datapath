use std::time::Duration;

use bytes::{Bytes, BytesMut};
use idxd_rust::{
    AsyncDsaSession, AsyncLifecycleFailureKind, AsyncMemmoveRequest, CompletionSnapshot, DsaConfig,
    direct_test_support::ScriptedDirectBackend,
};
use idxd_sys::DSA_COMP_SUCCESS;
use tokio::time::timeout;

fn direct_config() -> DsaConfig {
    DsaConfig::builder()
        .device_path("/dev/dsa/test0.0")
        .build()
        .expect("direct test config")
}

fn direct_session(backend: ScriptedDirectBackend) -> AsyncDsaSession {
    AsyncDsaSession::spawn_with_direct_backend(direct_config(), backend)
        .expect("direct test runtime should start")
}

fn owned_request(source: &'static [u8]) -> AsyncMemmoveRequest {
    AsyncMemmoveRequest::new(
        Bytes::from_static(source),
        BytesMut::with_capacity(source.len()),
    )
    .expect("request should validate")
}

async fn wait_for_submissions(backend: &ScriptedDirectBackend, expected: usize) {
    timeout(Duration::from_secs(1), async {
        while backend.submissions() < expected {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap_or_else(|_| {
        panic!(
            "timed out waiting for {expected} direct submissions; saw {}",
            backend.submissions()
        )
    });
}

#[tokio::test(flavor = "current_thread")]
async fn cloned_handles_compose_with_tokio_join_and_complete_out_of_order() {
    let backend = ScriptedDirectBackend::new();
    let session = direct_session(backend.clone());
    let first_handle = session.handle();
    let second_handle = first_handle.clone();

    let joined = tokio::spawn(async move {
        tokio::join!(
            first_handle.memmove(owned_request(b"first")),
            second_handle.memmove(owned_request(b"second"))
        )
    });

    wait_for_submissions(&backend, 2).await;
    assert!(
        timeout(Duration::from_millis(50), async {
            while !joined.is_finished() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .is_err(),
        "public direct futures must wait for completion records, not submit acceptance"
    );

    backend.complete(2, CompletionSnapshot::new(DSA_COMP_SUCCESS, 0, 0, 0));
    tokio::task::yield_now().await;
    assert!(
        !joined.is_finished(),
        "join! should still wait for request 1 even when request 2 completes first"
    );

    backend.complete(1, CompletionSnapshot::new(DSA_COMP_SUCCESS, 0, 0, 0));

    let (first, second) = timeout(Duration::from_secs(1), joined)
        .await
        .expect("completion records should finish join! composition")
        .expect("join! driver task should not panic");
    let first = first.expect("first cloned handle should succeed");
    let second = second.expect("second cloned handle should succeed");

    assert_eq!(first.destination.as_ref(), b"first");
    assert_eq!(second.destination.as_ref(), b"second");
    assert_eq!(backend.submissions(), 2);
    assert_eq!(backend.completions(), 2);

    session.shutdown().expect("owner shutdown should succeed");
}

#[tokio::test(flavor = "current_thread")]
async fn cloned_handles_compose_in_spawned_tasks_without_fifo_worker_serialization() {
    let backend = ScriptedDirectBackend::new();
    let session = direct_session(backend.clone());
    let first_handle = session.handle();
    let second_handle = first_handle.clone();

    let first_task =
        tokio::spawn(async move { first_handle.memmove(owned_request(b"alpha")).await });
    let second_task =
        tokio::spawn(async move { second_handle.memmove(owned_request(b"beta")).await });

    wait_for_submissions(&backend, 2).await;
    backend.complete(2, CompletionSnapshot::new(DSA_COMP_SUCCESS, 0, 0, 0));

    let second = timeout(Duration::from_secs(1), second_task)
        .await
        .expect("second request should complete from its record")
        .expect("second spawned task should not panic")
        .expect("second spawned handle should succeed");
    assert_eq!(second.destination.as_ref(), b"beta");
    assert!(
        !first_task.is_finished(),
        "first task should remain pending until its own completion record appears"
    );

    backend.complete(1, CompletionSnapshot::new(DSA_COMP_SUCCESS, 0, 0, 0));
    let first = timeout(Duration::from_secs(1), first_task)
        .await
        .expect("first request should complete from its record")
        .expect("first spawned task should not panic")
        .expect("first spawned handle should succeed");
    assert_eq!(first.destination.as_ref(), b"alpha");
    assert_eq!(backend.completions(), 2);

    session.shutdown().expect("owner shutdown should succeed");
}

#[tokio::test(flavor = "current_thread")]
async fn dropping_one_clone_does_not_shut_down_another_clone() {
    let backend = ScriptedDirectBackend::new();
    let session = direct_session(backend.clone());
    let retained_handle = session.handle();
    let dropped_handle = retained_handle.clone();

    drop(dropped_handle);

    let pending =
        tokio::spawn(async move { retained_handle.memmove(owned_request(b"alive")).await });
    wait_for_submissions(&backend, 1).await;
    backend.complete(1, CompletionSnapshot::new(DSA_COMP_SUCCESS, 0, 0, 0));

    let result = timeout(Duration::from_secs(1), pending)
        .await
        .expect("remaining clone should complete")
        .expect("remaining clone task should not panic")
        .expect("remaining clone should keep working");

    assert_eq!(result.destination.as_ref(), b"alive");

    session.shutdown().expect("owner shutdown should succeed");
}

#[tokio::test(flavor = "current_thread")]
async fn explicit_owner_shutdown_refuses_new_submissions_without_losing_accepted_work() {
    let backend = ScriptedDirectBackend::new();
    let session = direct_session(backend.clone());
    let inflight_handle = session.handle();
    let post_shutdown_handle = inflight_handle.clone();

    let inflight =
        tokio::spawn(async move { inflight_handle.memmove(owned_request(b"inflight")).await });
    wait_for_submissions(&backend, 1).await;

    session.shutdown().expect("owner shutdown should succeed");

    let err = post_shutdown_handle
        .memmove(owned_request(b"new"))
        .await
        .expect_err("new submissions after owner shutdown must fail structurally");

    assert_eq!(err.kind(), "owner_shutdown");
    assert_eq!(
        err.lifecycle_failure_kind(),
        Some(AsyncLifecycleFailureKind::OwnerShutdown)
    );
    assert!(err.worker_failure_kind().is_none());
    assert!(err.memmove_error().is_none());
    assert!(err.into_request().is_some());

    backend.complete(1, CompletionSnapshot::new(DSA_COMP_SUCCESS, 0, 0, 0));
    let result = timeout(Duration::from_secs(1), inflight)
        .await
        .expect("accepted operation should survive owner shutdown")
        .expect("in-flight task should not panic")
        .expect("accepted operation should still complete from its record");
    assert_eq!(result.destination.as_ref(), b"inflight");
    assert_eq!(backend.completions(), 1);
}
