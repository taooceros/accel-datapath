use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use std::task::{Context, Poll, Waker};
use std::thread::{self, JoinHandle};

use tokio::sync::{mpsc as tokio_mpsc, oneshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoundTripError {
    SpawnJoinFailed,
    OneshotSendFailed,
    OneshotReceiveFailed,
    MpscSendFailed,
    MpscReceiveFailed,
    CrossThreadWakeWorkerUnavailable,
}

impl fmt::Display for RoundTripError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            RoundTripError::SpawnJoinFailed => "tokio::spawn join failed",
            RoundTripError::OneshotSendFailed => "oneshot round trip did not complete the send path",
            RoundTripError::OneshotReceiveFailed => {
                "oneshot round trip did not complete the receive path"
            }
            RoundTripError::MpscSendFailed => "mpsc round trip did not complete the send path",
            RoundTripError::MpscReceiveFailed => "mpsc round trip did not complete the receive path",
            RoundTripError::CrossThreadWakeWorkerUnavailable => {
                "cross-thread wake worker was unavailable before wake registration completed"
            }
        };
        f.write_str(message)
    }
}

impl std::error::Error for RoundTripError {}

pub async fn spawn_join_round_trip() -> Result<(), RoundTripError> {
    tokio::spawn(async {}).await.map_err(|_| RoundTripError::SpawnJoinFailed)?;
    Ok(())
}

pub async fn oneshot_completion_round_trip() -> Result<(), RoundTripError> {
    let (tx, rx) = oneshot::channel();
    let sender = async move { tx.send(()).map_err(|_| RoundTripError::OneshotSendFailed) };
    let receiver = async move {
        rx.await
            .map_err(|_| RoundTripError::OneshotReceiveFailed)
            .map(|_| ())
    };

    let (send_result, receive_result) = tokio::join!(sender, receiver);
    send_result?;
    receive_result?;
    Ok(())
}

pub async fn mpsc_round_trip() -> Result<(), RoundTripError> {
    let (tx, mut rx) = tokio_mpsc::channel(1);
    let sender = async move { tx.send(()).await.map_err(|_| RoundTripError::MpscSendFailed) };
    let receiver = async move {
        rx.recv()
            .await
            .ok_or(RoundTripError::MpscReceiveFailed)
            .map(|_| ())
    };

    let (send_result, receive_result) = tokio::join!(sender, receiver);
    send_result?;
    receive_result?;
    Ok(())
}

pub async fn same_thread_wake_round_trip() -> Result<(), RoundTripError> {
    SameThreadWake::default().await;
    Ok(())
}

#[derive(Default)]
struct SameThreadWake {
    woke: bool,
}

impl Future for SameThreadWake {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.woke {
            Poll::Ready(())
        } else {
            self.woke = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

#[derive(Default)]
struct CrossThreadWakeState {
    ready: AtomicBool,
    waker: Mutex<Option<Waker>>,
}

enum WorkerCommand {
    Wake(Arc<CrossThreadWakeState>),
    Shutdown,
}

pub struct CrossThreadWakeHarness {
    sender: mpsc::Sender<WorkerCommand>,
    worker: Option<JoinHandle<()>>,
}

impl CrossThreadWakeHarness {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel::<WorkerCommand>();
        let worker = thread::Builder::new()
            .name("async-bench-cross-thread-wake".to_string())
            .spawn(move || {
                while let Ok(command) = receiver.recv() {
                    match command {
                        WorkerCommand::Wake(state) => {
                            state.ready.store(true, Ordering::Release);
                            if let Some(waker) = state.waker.lock().expect("waker lock poisoned").take() {
                                waker.wake();
                            }
                        }
                        WorkerCommand::Shutdown => break,
                    }
                }
            })
            .expect("spawn cross-thread wake worker");

        Self {
            sender,
            worker: Some(worker),
        }
    }

    pub async fn round_trip(&self) -> Result<(), RoundTripError> {
        CrossThreadWakeFuture::new(self.sender.clone()).await
    }

    #[cfg(test)]
    fn shutdown_for_test(mut self) -> Self {
        let _ = self.sender.send(WorkerCommand::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self
    }
}

impl Drop for CrossThreadWakeHarness {
    fn drop(&mut self) {
        let _ = self.sender.send(WorkerCommand::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

struct CrossThreadWakeFuture {
    sender: mpsc::Sender<WorkerCommand>,
    state: Arc<CrossThreadWakeState>,
    requested: bool,
}

impl CrossThreadWakeFuture {
    fn new(sender: mpsc::Sender<WorkerCommand>) -> Self {
        Self {
            sender,
            state: Arc::new(CrossThreadWakeState::default()),
            requested: false,
        }
    }
}

impl Future for CrossThreadWakeFuture {
    type Output = Result<(), RoundTripError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.state.ready.load(Ordering::Acquire) {
            return Poll::Ready(Ok(()));
        }

        {
            let mut waker_slot = self.state.waker.lock().expect("waker lock poisoned");
            *waker_slot = Some(cx.waker().clone());
        }

        if !self.requested {
            let state = Arc::clone(&self.state);
            if self.sender.send(WorkerCommand::Wake(state)).is_err() {
                return Poll::Ready(Err(RoundTripError::CrossThreadWakeWorkerUnavailable));
            }
            self.requested = true;
        }

        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "current_thread")]
    async fn same_thread_wake_completes() {
        same_thread_wake_round_trip().await.expect("same-thread wake should complete");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn oneshot_completion_completes() {
        oneshot_completion_round_trip()
            .await
            .expect("oneshot round trip should complete");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mpsc_completion_completes() {
        mpsc_round_trip().await.expect("mpsc round trip should complete");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cross_thread_wake_completes() {
        let harness = CrossThreadWakeHarness::new();
        harness
            .round_trip()
            .await
            .expect("cross-thread wake should complete");
    }

    #[test]
    fn round_trip_errors_are_descriptive() {
        assert_eq!(
            RoundTripError::OneshotReceiveFailed.to_string(),
            "oneshot round trip did not complete the receive path"
        );
        assert_eq!(
            RoundTripError::CrossThreadWakeWorkerUnavailable.to_string(),
            "cross-thread wake worker was unavailable before wake registration completed"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn cross_thread_wake_rejects_missing_worker() {
        let harness = CrossThreadWakeHarness::new().shutdown_for_test();
        let error = harness.round_trip().await.expect_err("closed worker should fail");
        assert_eq!(error, RoundTripError::CrossThreadWakeWorkerUnavailable);
    }
}
