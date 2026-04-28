use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};
use std::thread::{self, JoinHandle};

use bytes::{Bytes, BytesMut};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

use crate::{
    DEFAULT_DEVICE_PATH, DEFAULT_MAX_PAGE_FAULT_RETRIES, DsaSession, MemmoveError, MemmoveRequest,
    MemmoveValidationReport,
};

const LIFECYCLE_RUNNING: u8 = 0;
const LIFECYCLE_SHUTDOWN_REQUESTED: u8 = 1;
const LIFECYCLE_SHUTDOWN_COMPLETE: u8 = 2;

/// Owned memmove request that can safely cross the worker-thread boundary.
///
/// The source length is the requested transfer size. The destination is caller
/// supplied as an owned [`BytesMut`] whose current spare capacity is the async
/// write target; on success the owned async result returns that destination plus
/// validation metadata. Callers allocate and retain destination ownership
/// explicitly by constructing [`Bytes`] and [`BytesMut`] before enqueue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsyncMemmoveRequest {
    source: Bytes,
    destination: BytesMut,
}

impl AsyncMemmoveRequest {
    /// Build an owned request from explicit source and destination buffers.
    ///
    /// Validation is synchronous and happens before enqueue. If the source is
    /// empty or the destination lacks enough spare writable capacity, the
    /// rejected buffers are returned in [`AsyncMemmoveRequestError`].
    pub fn new(source: Bytes, destination: BytesMut) -> Result<Self, AsyncMemmoveRequestError> {
        let writable_capacity = destination.capacity().saturating_sub(destination.len());
        if let Err(error) = MemmoveRequest::for_buffers(writable_capacity, source.len()) {
            return Err(AsyncMemmoveRequestError {
                error,
                source_buffer: source,
                destination,
            });
        }

        Ok(Self {
            source,
            destination,
        })
    }

    pub fn requested_bytes(&self) -> usize {
        self.source.len()
    }

    pub fn destination_len(&self) -> usize {
        self.destination.len()
    }

    pub fn destination_capacity(&self) -> usize {
        self.destination.capacity()
    }

    pub fn destination_writable_capacity(&self) -> usize {
        self.destination
            .capacity()
            .saturating_sub(self.destination.len())
    }
}

/// Recoverable constructor failure for an owned async memmove request.
///
/// This error preserves typed [`MemmoveError`] diagnostics and owns the rejected
/// buffers so callers can inspect lengths or retry without payload logging.
#[derive(Debug, Error)]
#[error("invalid async memmove request: {error}")]
pub struct AsyncMemmoveRequestError {
    error: MemmoveError,
    source_buffer: Bytes,
    destination: BytesMut,
}

impl AsyncMemmoveRequestError {
    pub fn kind(&self) -> &'static str {
        self.error.kind()
    }

    pub fn memmove_error(&self) -> &MemmoveError {
        &self.error
    }

    pub fn requested_bytes(&self) -> usize {
        self.source_buffer.len()
    }

    pub fn destination_len(&self) -> usize {
        self.destination.len()
    }

    pub fn destination_capacity(&self) -> usize {
        self.destination.capacity()
    }

    pub fn destination_writable_capacity(&self) -> usize {
        self.destination
            .capacity()
            .saturating_sub(self.destination.len())
    }

    pub fn into_parts(self) -> (MemmoveError, Bytes, BytesMut) {
        (self.error, self.source_buffer, self.destination)
    }
}

/// Owned memmove result returned across the async boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsyncMemmoveResult {
    pub destination: BytesMut,
    pub report: MemmoveValidationReport,
}

/// Explicit owner/lifecycle failure kinds that are distinct from worker failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncLifecycleFailureKind {
    OwnerShutdown,
}

impl AsyncLifecycleFailureKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OwnerShutdown => "owner_shutdown",
        }
    }
}

impl std::fmt::Display for AsyncLifecycleFailureKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Narrow async-structural failure kinds. Real DSA failures remain `MemmoveError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncWorkerFailureKind {
    WorkerInitClosed,
    RequestChannelClosed,
    ResponseChannelClosed,
    WorkerPanicked,
}

impl AsyncWorkerFailureKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WorkerInitClosed => "worker_init_closed",
            Self::RequestChannelClosed => "request_channel_closed",
            Self::ResponseChannelClosed => "response_channel_closed",
            Self::WorkerPanicked => "worker_panicked",
        }
    }
}

impl std::fmt::Display for AsyncWorkerFailureKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Async wrapper error that preserves the underlying synchronous `MemmoveError`.
#[derive(Debug, Error)]
pub enum AsyncMemmoveError {
    #[error(transparent)]
    Memmove(#[from] MemmoveError),

    #[error("async memmove lifecycle failure: {kind}")]
    LifecycleFailure { kind: AsyncLifecycleFailureKind },

    #[error("async memmove worker failure: {kind}")]
    WorkerFailure { kind: AsyncWorkerFailureKind },
}

impl AsyncMemmoveError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Memmove(err) => err.kind(),
            Self::LifecycleFailure { kind } => kind.as_str(),
            Self::WorkerFailure { kind } => kind.as_str(),
        }
    }

    pub fn lifecycle_failure_kind(&self) -> Option<AsyncLifecycleFailureKind> {
        match self {
            Self::LifecycleFailure { kind } => Some(*kind),
            Self::Memmove(_) | Self::WorkerFailure { .. } => None,
        }
    }

    pub fn worker_failure_kind(&self) -> Option<AsyncWorkerFailureKind> {
        match self {
            Self::WorkerFailure { kind } => Some(*kind),
            Self::Memmove(_) | Self::LifecycleFailure { .. } => None,
        }
    }

    pub fn memmove_error(&self) -> Option<&MemmoveError> {
        match self {
            Self::Memmove(err) => Some(err),
            Self::LifecycleFailure { .. } | Self::WorkerFailure { .. } => None,
        }
    }
}

/// Abstraction used by the async worker thread. `DsaSession` remains the only
/// real low-level submission path; tests can swap in a host-independent fake.
pub trait AsyncMemmoveWorker: Send + 'static {
    fn memmove(
        &mut self,
        dst: &mut [u8],
        src: &[u8],
    ) -> Result<MemmoveValidationReport, MemmoveError>;
}

impl AsyncMemmoveWorker for DsaSession {
    fn memmove(
        &mut self,
        dst: &mut [u8],
        src: &[u8],
    ) -> Result<MemmoveValidationReport, MemmoveError> {
        DsaSession::memmove(self, dst, src)
    }
}

enum WorkerCommand {
    Memmove {
        request: AsyncMemmoveRequest,
        reply_tx: oneshot::Sender<Result<AsyncMemmoveResult, MemmoveError>>,
    },
    Shutdown,
}

#[derive(Debug)]
struct SharedWorkerState {
    request_tx: mpsc::UnboundedSender<WorkerCommand>,
    lifecycle: AtomicU8,
}

impl SharedWorkerState {
    fn lifecycle_state(&self) -> u8 {
        self.lifecycle.load(Ordering::Acquire)
    }

    fn is_shutdown_requested(&self) -> bool {
        self.lifecycle_state() >= LIFECYCLE_SHUTDOWN_REQUESTED
    }

    fn mark_shutdown_requested(&self) {
        self.lifecycle
            .store(LIFECYCLE_SHUTDOWN_REQUESTED, Ordering::Release);
    }

    fn mark_shutdown_complete(&self) {
        self.lifecycle
            .store(LIFECYCLE_SHUTDOWN_COMPLETE, Ordering::Release);
    }
}

/// Cloneable Tokio-facing handle over one worker-owned `DsaSession`.
///
/// Cloned handles compose naturally in ordinary Tokio code such as
/// `tokio::join!` or spawned tasks, but they still share one worker thread and
/// one session. Requests cross the boundary as owned data, queue FIFO, and
/// execute one at a time; cloning the handle never duplicates hardware
/// ownership or implies parallel execution. Use `memmove` with an
/// [`AsyncMemmoveRequest`] for spawn-friendly owned work; callers provide the
/// owned [`Bytes`] source and [`BytesMut`] destination explicitly.
#[derive(Debug, Clone)]
pub struct AsyncDsaHandle {
    shared: Arc<SharedWorkerState>,
}

impl AsyncDsaHandle {
    /// Submit one owned request through the shared worker-owned session.
    ///
    /// Once the request has been enqueued successfully, dropping or aborting
    /// the awaiting Tokio task does not cancel the worker-side memmove. The
    /// worker still finishes the request and later submissions can continue to
    /// use the shared handle.
    pub async fn memmove(
        &self,
        request: AsyncMemmoveRequest,
    ) -> Result<AsyncMemmoveResult, AsyncMemmoveError> {
        if self.shared.is_shutdown_requested() {
            return Err(AsyncMemmoveError::LifecycleFailure {
                kind: AsyncLifecycleFailureKind::OwnerShutdown,
            });
        }

        let (reply_tx, reply_rx) = oneshot::channel();

        self.shared
            .request_tx
            .send(WorkerCommand::Memmove { request, reply_tx })
            .map_err(|_| self.classify_send_failure())?;

        reply_rx
            .await
            .map_err(|_| self.classify_reply_failure())?
            .map_err(AsyncMemmoveError::from)
    }

    fn classify_send_failure(&self) -> AsyncMemmoveError {
        if self.shared.is_shutdown_requested() {
            AsyncMemmoveError::LifecycleFailure {
                kind: AsyncLifecycleFailureKind::OwnerShutdown,
            }
        } else {
            AsyncMemmoveError::WorkerFailure {
                kind: AsyncWorkerFailureKind::RequestChannelClosed,
            }
        }
    }

    fn classify_reply_failure(&self) -> AsyncMemmoveError {
        if self.shared.is_shutdown_requested() {
            AsyncMemmoveError::LifecycleFailure {
                kind: AsyncLifecycleFailureKind::OwnerShutdown,
            }
        } else {
            AsyncMemmoveError::WorkerFailure {
                kind: AsyncWorkerFailureKind::ResponseChannelClosed,
            }
        }
    }
}

/// Explicit owner/shutdown control for one shared async DSA worker.
#[derive(Debug)]
pub struct AsyncDsaSession {
    handle: AsyncDsaHandle,
    worker_thread: Option<JoinHandle<()>>,
}

impl AsyncDsaSession {
    pub fn open<P: AsRef<Path>>(device_path: P) -> Result<Self, AsyncMemmoveError> {
        Self::open_with_retries(device_path, DEFAULT_MAX_PAGE_FAULT_RETRIES)
    }

    pub fn open_default() -> Result<Self, AsyncMemmoveError> {
        Self::open(DEFAULT_DEVICE_PATH)
    }

    pub fn open_with_retries<P: AsRef<Path>>(
        device_path: P,
        max_page_fault_retries: u32,
    ) -> Result<Self, AsyncMemmoveError> {
        let device_path = device_path.as_ref().to_path_buf();
        Self::spawn_with_factory(move || {
            DsaSession::open_with_retries(&device_path, max_page_fault_retries)
        })
    }

    /// Spawn a worker from a custom factory. This is public so integration tests
    /// can prove contract behavior without requiring DSA hardware.
    #[doc(hidden)]
    pub fn spawn_with_factory<F, W>(factory: F) -> Result<Self, AsyncMemmoveError>
    where
        F: FnOnce() -> Result<W, MemmoveError> + Send + 'static,
        W: AsyncMemmoveWorker,
    {
        let (request_tx, mut request_rx) = mpsc::unbounded_channel();
        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);

        let worker_thread = thread::spawn(move || {
            let mut worker = match factory() {
                Ok(worker) => {
                    let _ = ready_tx.send(Ok(()));
                    worker
                }
                Err(err) => {
                    let _ = ready_tx.send(Err(err));
                    return;
                }
            };

            while let Some(command) = request_rx.blocking_recv() {
                match command {
                    WorkerCommand::Memmove { request, reply_tx } => {
                        let result = run_memmove(&mut worker, request);
                        let _ = reply_tx.send(result);
                    }
                    WorkerCommand::Shutdown => break,
                }
            }
        });

        match ready_rx.recv() {
            Ok(Ok(())) => {
                let handle = AsyncDsaHandle {
                    shared: Arc::new(SharedWorkerState {
                        request_tx,
                        lifecycle: AtomicU8::new(LIFECYCLE_RUNNING),
                    }),
                };
                Ok(Self {
                    handle,
                    worker_thread: Some(worker_thread),
                })
            }
            Ok(Err(err)) => {
                let _ = worker_thread.join();
                Err(err.into())
            }
            Err(_) => {
                let _ = worker_thread.join();
                Err(AsyncMemmoveError::WorkerFailure {
                    kind: AsyncWorkerFailureKind::WorkerInitClosed,
                })
            }
        }
    }

    /// Borrow the cloneable Tokio-facing handle.
    ///
    /// Every clone still feeds the same worker-owned `DsaSession`, so Tokio
    /// callers can share the handle freely without widening the ownership
    /// boundary or changing the one-worker serialization contract.
    pub fn handle(&self) -> AsyncDsaHandle {
        self.handle.clone()
    }

    /// Backward-compatible convenience that delegates through the shared handle.
    pub async fn memmove(
        &self,
        request: AsyncMemmoveRequest,
    ) -> Result<AsyncMemmoveResult, AsyncMemmoveError> {
        self.handle.memmove(request).await
    }

    /// Close the shared worker explicitly and wait for the worker thread to exit.
    ///
    /// Shutdown is drain-then-stop: already-queued requests are allowed to run
    /// to completion before the worker exits, and later submissions through any
    /// cloned handle fail with `owner_shutdown`.
    pub fn shutdown(mut self) -> Result<(), AsyncMemmoveError> {
        self.shutdown_inner()
    }

    fn shutdown_inner(&mut self) -> Result<(), AsyncMemmoveError> {
        if self.worker_thread.is_none() {
            self.handle.shared.mark_shutdown_complete();
            return Ok(());
        }

        self.handle.shared.mark_shutdown_requested();
        let _ = self.handle.shared.request_tx.send(WorkerCommand::Shutdown);

        if let Some(worker_thread) = self.worker_thread.take() {
            worker_thread
                .join()
                .map_err(|_| AsyncMemmoveError::WorkerFailure {
                    kind: AsyncWorkerFailureKind::WorkerPanicked,
                })?;
        }

        self.handle.shared.mark_shutdown_complete();
        Ok(())
    }
}

impl Drop for AsyncDsaSession {
    fn drop(&mut self) {
        let _ = self.shutdown_inner();
    }
}

fn run_memmove<W: AsyncMemmoveWorker>(
    worker: &mut W,
    request: AsyncMemmoveRequest,
) -> Result<AsyncMemmoveResult, MemmoveError> {
    let AsyncMemmoveRequest {
        source,
        mut destination,
    } = request;
    let report = worker.memmove(&mut destination, &source)?;
    Ok(AsyncMemmoveResult {
        destination,
        report,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_constructor_accepts_owned_bytes_and_spare_destination_capacity() {
        let source = Bytes::from_static(b"abcd");
        let destination = BytesMut::with_capacity(source.len());

        let request = AsyncMemmoveRequest::new(source, destination).expect("request validates");

        assert_eq!(request.requested_bytes(), 4);
        assert_eq!(request.destination_len(), 0);
        assert_eq!(request.destination_writable_capacity(), 4);
    }

    #[test]
    fn request_constructor_recovers_empty_source_buffers() {
        let source = Bytes::new();
        let destination = BytesMut::with_capacity(4);

        let error = AsyncMemmoveRequest::new(source, destination).expect_err("empty source fails");

        assert_eq!(error.kind(), "invalid_length");
        let (memmove_error, recovered_source, recovered_destination) = error.into_parts();
        assert!(matches!(memmove_error, MemmoveError::InvalidLength { .. }));
        assert!(recovered_source.is_empty());
        assert_eq!(recovered_destination.capacity(), 4);
    }

    #[test]
    fn request_constructor_recovers_under_capacity_destination() {
        let source = Bytes::from_static(b"abcd");
        let destination = BytesMut::with_capacity(source.len() - 1);

        let error =
            AsyncMemmoveRequest::new(source, destination).expect_err("short destination fails");

        assert_eq!(error.kind(), "destination_too_small");
        let (memmove_error, recovered_source, recovered_destination) = error.into_parts();
        assert!(matches!(
            memmove_error,
            MemmoveError::DestinationTooSmall {
                src_len: 4,
                dst_len: 3
            }
        ));
        assert_eq!(recovered_source.as_ref(), b"abcd");
        assert_eq!(recovered_destination.capacity(), 3);
    }
}
