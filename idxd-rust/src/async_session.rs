use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};
use std::thread::{self, JoinHandle};

use bytes::{Bytes, BytesMut, buf::UninitSlice};
use snafu::Snafu;
use tokio::sync::{mpsc, oneshot};

use crate::{
    AsyncDirectFailure, AsyncDirectFailureKind, DEFAULT_MAX_PAGE_FAULT_RETRIES,
    DirectAsyncMemmoveRuntime, DirectMemmoveBackend, DirectPortalBackend, DsaConfig, DsaSession,
    MemmoveError, MemmoveRequest, MemmoveValidationReport,
};

const LIFECYCLE_RUNNING: u8 = 0;
const LIFECYCLE_SHUTDOWN_REQUESTED: u8 = 1;
const LIFECYCLE_SHUTDOWN_COMPLETE: u8 = 2;

/// Owned memmove request that can safely cross the async boundary.
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
    pub fn new(source: Bytes, mut destination: BytesMut) -> Result<Self, AsyncMemmoveRequestError> {
        let writable_capacity = destination.spare_capacity_mut().len();
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

    pub fn into_parts(self) -> (Bytes, BytesMut) {
        (self.source, self.destination)
    }
}

/// Recoverable constructor failure for an owned async memmove request.
///
/// This error preserves typed [`MemmoveError`] diagnostics and owns the rejected
/// buffers so callers can inspect lengths or retry without payload logging.
#[derive(Debug, Snafu)]
#[snafu(display("invalid async memmove request: {error}"))]
pub struct AsyncMemmoveRequestError {
    #[snafu(source)]
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

/// Async memmove error that preserves typed lifecycle, direct-runtime,
/// legacy-worker-fixture, and underlying `MemmoveError` failures.
#[derive(Debug, Snafu)]
pub enum AsyncMemmoveError {
    #[snafu(display("async memmove execution failure: {source}"))]
    Memmove {
        source: MemmoveError,
        request: Option<AsyncMemmoveRequest>,
    },

    #[snafu(display("async memmove lifecycle failure: {kind}"))]
    LifecycleFailure {
        kind: AsyncLifecycleFailureKind,
        request: Option<AsyncMemmoveRequest>,
    },

    #[snafu(display("async memmove worker failure: {kind}"))]
    WorkerFailure {
        kind: AsyncWorkerFailureKind,
        request: Option<AsyncMemmoveRequest>,
    },

    #[snafu(display("async direct memmove failure: {failure}"))]
    DirectFailure {
        #[snafu(source)]
        failure: AsyncDirectFailure,
        request: Option<AsyncMemmoveRequest>,
    },
}

impl AsyncMemmoveError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Memmove { source, .. } => source.kind(),
            Self::LifecycleFailure { kind, .. } => kind.as_str(),
            Self::WorkerFailure { kind, .. } => kind.as_str(),
            Self::DirectFailure { failure, .. } => failure.kind().as_str(),
        }
    }

    pub fn lifecycle_failure_kind(&self) -> Option<AsyncLifecycleFailureKind> {
        match self {
            Self::LifecycleFailure { kind, .. } => Some(*kind),
            Self::Memmove { .. } | Self::WorkerFailure { .. } | Self::DirectFailure { .. } => None,
        }
    }

    pub fn worker_failure_kind(&self) -> Option<AsyncWorkerFailureKind> {
        match self {
            Self::WorkerFailure { kind, .. } => Some(*kind),
            Self::Memmove { .. } | Self::LifecycleFailure { .. } | Self::DirectFailure { .. } => {
                None
            }
        }
    }

    pub fn direct_failure_kind(&self) -> Option<AsyncDirectFailureKind> {
        match self {
            Self::DirectFailure { failure, .. } => Some(failure.kind()),
            Self::Memmove { .. } | Self::LifecycleFailure { .. } | Self::WorkerFailure { .. } => {
                None
            }
        }
    }

    pub fn direct_failure(&self) -> Option<&AsyncDirectFailure> {
        match self {
            Self::DirectFailure { failure, .. } => Some(failure),
            Self::Memmove { .. } | Self::LifecycleFailure { .. } | Self::WorkerFailure { .. } => {
                None
            }
        }
    }

    pub fn memmove_error(&self) -> Option<&MemmoveError> {
        match self {
            Self::Memmove { source, .. } => Some(source),
            Self::LifecycleFailure { .. }
            | Self::WorkerFailure { .. }
            | Self::DirectFailure { .. } => None,
        }
    }

    pub fn into_request(self) -> Option<AsyncMemmoveRequest> {
        match self {
            Self::Memmove { request, .. }
            | Self::LifecycleFailure { request, .. }
            | Self::WorkerFailure { request, .. }
            | Self::DirectFailure { request, .. } => request,
        }
    }
}

impl From<MemmoveError> for AsyncMemmoveError {
    fn from(source: MemmoveError) -> Self {
        Self::Memmove {
            source,
            request: None,
        }
    }
}

/// Abstraction used by the legacy async worker thread. The public default path
/// no longer uses this trait; it remains as a hidden compatibility seam for
/// host-independent fixtures that model the old synchronous wrapper.
pub trait AsyncMemmoveWorker: Send + 'static {
    fn memmove(
        &mut self,
        dst: &mut UninitSlice,
        src: &[u8],
    ) -> Result<MemmoveValidationReport, MemmoveError>;
}

impl AsyncMemmoveWorker for DsaSession {
    fn memmove(
        &mut self,
        dst: &mut UninitSlice,
        src: &[u8],
    ) -> Result<MemmoveValidationReport, MemmoveError> {
        DsaSession::memmove_uninit(self, dst, src)
    }
}

type DriverFuture<'a> =
    Pin<Box<dyn Future<Output = Result<AsyncMemmoveResult, AsyncMemmoveError>> + Send + 'a>>;

trait AsyncMemmoveDriver: Send + Sync + 'static {
    fn memmove<'a>(&'a self, request: AsyncMemmoveRequest) -> DriverFuture<'a>;
    fn close(&self);
}

#[derive(Debug)]
struct DirectRuntimeDriver<B> {
    runtime: DirectAsyncMemmoveRuntime<B>,
}

impl<B> DirectRuntimeDriver<B> {
    fn new(runtime: DirectAsyncMemmoveRuntime<B>) -> Self {
        Self { runtime }
    }
}

impl<B> AsyncMemmoveDriver for DirectRuntimeDriver<B>
where
    B: DirectMemmoveBackend,
{
    fn memmove<'a>(&'a self, request: AsyncMemmoveRequest) -> DriverFuture<'a> {
        Box::pin(async move { self.runtime.memmove(request).await })
    }

    fn close(&self) {
        // Owner shutdown only prevents new public submissions. Accepted direct
        // operations remain owned by the runtime and are resolved by completion
        // observation; the runtime is closed by Drop when the last handle goes
        // away.
    }
}

enum WorkerCommand {
    Memmove {
        request: AsyncMemmoveRequest,
        reply_tx: oneshot::Sender<Result<AsyncMemmoveResult, AsyncMemmoveError>>,
    },
    Shutdown,
}

#[derive(Debug)]
struct WorkerRuntimeDriver {
    request_tx: mpsc::UnboundedSender<WorkerCommand>,
}

impl WorkerRuntimeDriver {
    fn classify_send_failure(request: Option<AsyncMemmoveRequest>) -> AsyncMemmoveError {
        AsyncMemmoveError::WorkerFailure {
            kind: AsyncWorkerFailureKind::RequestChannelClosed,
            request,
        }
    }

    fn classify_reply_failure() -> AsyncMemmoveError {
        AsyncMemmoveError::WorkerFailure {
            kind: AsyncWorkerFailureKind::ResponseChannelClosed,
            request: None,
        }
    }
}

impl AsyncMemmoveDriver for WorkerRuntimeDriver {
    fn memmove<'a>(&'a self, request: AsyncMemmoveRequest) -> DriverFuture<'a> {
        Box::pin(async move {
            let (reply_tx, reply_rx) = oneshot::channel();

            let send_error = match self
                .request_tx
                .send(WorkerCommand::Memmove { request, reply_tx })
            {
                Ok(()) => None,
                Err(err) => Some(err.0),
            };

            if let Some(WorkerCommand::Memmove { request, .. }) = send_error {
                return Err(Self::classify_send_failure(Some(request)));
            }

            reply_rx.await.map_err(|_| Self::classify_reply_failure())?
        })
    }

    fn close(&self) {
        let _ = self.request_tx.send(WorkerCommand::Shutdown);
    }
}

struct SharedAsyncState {
    driver: Arc<dyn AsyncMemmoveDriver>,
    lifecycle: AtomicU8,
}

impl std::fmt::Debug for SharedAsyncState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedAsyncState")
            .field("lifecycle", &self.lifecycle_state())
            .finish_non_exhaustive()
    }
}

impl SharedAsyncState {
    fn lifecycle_state(&self) -> u8 {
        self.lifecycle.load(Ordering::Acquire)
    }

    fn is_shutdown_requested(&self) -> bool {
        self.lifecycle_state() >= LIFECYCLE_SHUTDOWN_REQUESTED
    }

    fn mark_shutdown_requested(&self) -> bool {
        self.lifecycle
            .compare_exchange(
                LIFECYCLE_RUNNING,
                LIFECYCLE_SHUTDOWN_REQUESTED,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    fn mark_shutdown_complete(&self) {
        self.lifecycle
            .store(LIFECYCLE_SHUTDOWN_COMPLETE, Ordering::Release);
    }
}

/// Cloneable Tokio-facing handle over one async DSA runtime.
///
/// Cloned handles compose naturally in ordinary Tokio code such as
/// `tokio::join!` or spawned tasks. Publicly opened sessions submit through the
/// direct async runtime, share one WQ portal/monitor, and resolve futures from
/// per-operation completion record observation instead of a blocking worker
/// thread. Use `memmove` with an [`AsyncMemmoveRequest`] for spawn-friendly
/// owned work; callers provide the owned [`Bytes`] source and [`BytesMut`]
/// destination explicitly.
#[derive(Debug, Clone)]
pub struct AsyncDsaHandle {
    shared: Arc<SharedAsyncState>,
}

impl AsyncDsaHandle {
    /// Submit one owned request through the shared direct async runtime.
    ///
    /// Once a request is accepted by the direct runtime, dropping or aborting
    /// the awaiting Tokio task does not cancel the operation-owned descriptor,
    /// completion record, or buffers. The monitor keeps the operation alive
    /// until terminal completion or runtime cleanup.
    pub async fn memmove(
        &self,
        request: AsyncMemmoveRequest,
    ) -> Result<AsyncMemmoveResult, AsyncMemmoveError> {
        if self.shared.is_shutdown_requested() {
            return Err(AsyncMemmoveError::LifecycleFailure {
                kind: AsyncLifecycleFailureKind::OwnerShutdown,
                request: Some(request),
            });
        }

        self.shared.driver.memmove(request).await
    }
}

/// Explicit owner/shutdown control for one shared async DSA runtime.
#[derive(Debug)]
pub struct AsyncDsaSession {
    handle: AsyncDsaHandle,
    worker_thread: Option<JoinHandle<()>>,
}

#[bon::bon]
impl AsyncDsaSession {
    pub fn open<P: AsRef<Path>>(device_path: P) -> Result<Self, AsyncMemmoveError> {
        Self::open_with_retries(device_path, DEFAULT_MAX_PAGE_FAULT_RETRIES)
    }

    pub fn open_with_retries<P: AsRef<Path>>(
        device_path: P,
        max_page_fault_retries: u32,
    ) -> Result<Self, AsyncMemmoveError> {
        let config = DsaConfig::with_retries(device_path, max_page_fault_retries)?;
        Self::open_config(config)
    }

    /// Open the public direct async runtime from an already-normalized config.
    ///
    /// The generated `AsyncDsaSession::builder().open()` path is kept as a
    /// named way to pass a prebuilt config into live direct-backend opening
    /// while preserving queue-open diagnostics and direct-runtime failure
    /// mapping. The hidden direct-backend fixture seam remains separate in
    /// `spawn_with_direct_backend` so tests can inject deterministic backends
    /// without going through a live work queue.
    #[builder(start_fn = builder, finish_fn = open)]
    pub fn open_config(
        #[builder(default)] dsa_config: DsaConfig,
    ) -> Result<Self, AsyncMemmoveError> {
        let backend = DirectPortalBackend::open(dsa_config.device_path())?;
        let runtime =
            DirectAsyncMemmoveRuntime::try_new(dsa_config, backend).map_err(|failure| {
                AsyncMemmoveError::DirectFailure {
                    failure,
                    request: None,
                }
            })?;
        Ok(Self::from_driver(
            Arc::new(DirectRuntimeDriver::new(runtime)),
            None,
        ))
    }

    /// Spawn a legacy blocking-worker fixture from a custom factory.
    ///
    /// This hidden seam is retained for older host-independent tests. It is not
    /// used by `open` or `open_with_retries`, so the public path cannot
    /// silently fall back to synchronous `DsaSession::memmove_uninit` execution.
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
            Ok(Ok(())) => Ok(Self::from_driver(
                Arc::new(WorkerRuntimeDriver { request_tx }),
                Some(worker_thread),
            )),
            Ok(Err(err)) => {
                let _ = worker_thread.join();
                Err(err.into())
            }
            Err(_) => {
                let _ = worker_thread.join();
                Err(AsyncMemmoveError::WorkerFailure {
                    kind: AsyncWorkerFailureKind::WorkerInitClosed,
                    request: None,
                })
            }
        }
    }

    #[doc(hidden)]
    pub fn spawn_with_direct_backend<B>(
        config: DsaConfig,
        backend: B,
    ) -> Result<Self, AsyncMemmoveError>
    where
        B: DirectMemmoveBackend,
    {
        let runtime = DirectAsyncMemmoveRuntime::try_new(config, backend).map_err(|failure| {
            AsyncMemmoveError::DirectFailure {
                failure,
                request: None,
            }
        })?;
        Ok(Self::from_driver(
            Arc::new(DirectRuntimeDriver::new(runtime)),
            None,
        ))
    }

    fn from_driver(
        driver: Arc<dyn AsyncMemmoveDriver>,
        worker_thread: Option<JoinHandle<()>>,
    ) -> Self {
        let handle = AsyncDsaHandle {
            shared: Arc::new(SharedAsyncState {
                driver,
                lifecycle: AtomicU8::new(LIFECYCLE_RUNNING),
            }),
        };
        Self {
            handle,
            worker_thread,
        }
    }

    /// Borrow the cloneable Tokio-facing handle.
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

    /// Close the owner and reject future submissions through cloned handles.
    ///
    /// Direct-runtime submissions that were already accepted remain owned by
    /// the direct runtime and complete from their completion records. The hidden
    /// legacy worker fixture still receives a shutdown command so old tests can
    /// join its worker thread deterministically.
    pub fn shutdown(mut self) -> Result<(), AsyncMemmoveError> {
        self.shutdown_inner()
    }

    fn shutdown_inner(&mut self) -> Result<(), AsyncMemmoveError> {
        if !self.handle.shared.mark_shutdown_requested() {
            self.handle.shared.mark_shutdown_complete();
            return Ok(());
        }

        self.handle.shared.driver.close();

        if let Some(worker_thread) = self.worker_thread.take() {
            worker_thread
                .join()
                .map_err(|_| AsyncMemmoveError::WorkerFailure {
                    kind: AsyncWorkerFailureKind::WorkerPanicked,
                    request: None,
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
) -> Result<AsyncMemmoveResult, AsyncMemmoveError> {
    let AsyncMemmoveRequest {
        source,
        mut destination,
    } = request;
    let requested_bytes = source.len();
    let original_len = destination.len();

    if destination.spare_capacity_mut().len() < requested_bytes {
        let dst_len = destination.capacity().saturating_sub(destination.len());
        return Err(AsyncMemmoveError::Memmove {
            source: MemmoveError::DestinationTooSmall {
                src_len: requested_bytes,
                dst_len,
            },
            request: Some(AsyncMemmoveRequest {
                source,
                destination,
            }),
        });
    }

    let worker_result = {
        let spare = destination.spare_capacity_mut();
        let dst: &mut UninitSlice = (&mut spare[..requested_bytes]).into();
        worker.memmove(dst, &source)
    };

    let report = match worker_result {
        Ok(report) => report,
        Err(error) => {
            return Err(AsyncMemmoveError::Memmove {
                source: error,
                request: Some(AsyncMemmoveRequest {
                    source,
                    destination,
                }),
            });
        }
    };

    // SAFETY: The worker returned success after writing exactly `requested_bytes`
    // bytes into the current spare capacity slice above. The constructor and
    // worker-side guard verified that spare capacity is at least this large, so
    // advancing from the original initialized length exposes only initialized
    // bytes appended by this memmove.
    unsafe {
        destination.set_len(original_len + requested_bytes);
    }

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
