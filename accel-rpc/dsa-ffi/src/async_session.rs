use std::path::Path;
use std::thread::{self, JoinHandle};

use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

use crate::{
    DsaSession, MemmoveError, MemmoveRequest, MemmoveValidationReport, DEFAULT_DEVICE_PATH,
    DEFAULT_MAX_PAGE_FAULT_RETRIES,
};

/// Owned memmove request that can safely cross the worker-thread boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsyncMemmoveRequest {
    src: Vec<u8>,
    dst_len: usize,
}

impl AsyncMemmoveRequest {
    /// Build an owned request whose destination size matches the source length.
    pub fn new(src: Vec<u8>) -> Result<Self, MemmoveError> {
        let dst_len = src.len();
        Self::with_destination_len(src, dst_len)
    }

    /// Build an owned request while validating the eventual destination size up front.
    pub fn with_destination_len(src: Vec<u8>, dst_len: usize) -> Result<Self, MemmoveError> {
        MemmoveRequest::for_buffers(dst_len, src.len())?;
        Ok(Self { src, dst_len })
    }

    pub fn requested_bytes(&self) -> usize {
        self.src.len()
    }

    pub fn destination_len(&self) -> usize {
        self.dst_len
    }

    pub fn source_bytes(&self) -> &[u8] {
        &self.src
    }
}

/// Owned memmove result returned across the async boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsyncMemmoveResult {
    pub bytes: Vec<u8>,
    pub report: MemmoveValidationReport,
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

    #[error("async memmove worker failure: {kind}")]
    WorkerFailure { kind: AsyncWorkerFailureKind },
}

impl AsyncMemmoveError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Memmove(err) => err.kind(),
            Self::WorkerFailure { kind } => kind.as_str(),
        }
    }

    pub fn worker_failure_kind(&self) -> Option<AsyncWorkerFailureKind> {
        match self {
            Self::WorkerFailure { kind } => Some(*kind),
            Self::Memmove(_) => None,
        }
    }

    pub fn memmove_error(&self) -> Option<&MemmoveError> {
        match self {
            Self::Memmove(err) => Some(err),
            Self::WorkerFailure { .. } => None,
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
}

/// Minimal awaitable wrapper over one worker-owned DSA session.
///
/// This type makes serialization explicit: one worker thread owns one session,
/// requests cross the boundary as owned data, and replies return owned bytes
/// plus the original synchronous validation report.
#[derive(Debug)]
pub struct AsyncDsaSession {
    request_tx: Option<mpsc::Sender<WorkerCommand>>,
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
        let (request_tx, mut request_rx) = mpsc::channel(1);
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
                }
            }
        });

        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                request_tx: Some(request_tx),
                worker_thread: Some(worker_thread),
            }),
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

    /// Submit one owned request through the single worker-owned session.
    pub async fn memmove(
        &self,
        request: AsyncMemmoveRequest,
    ) -> Result<AsyncMemmoveResult, AsyncMemmoveError> {
        let request_tx = self
            .request_tx
            .as_ref()
            .ok_or(AsyncMemmoveError::WorkerFailure {
                kind: AsyncWorkerFailureKind::RequestChannelClosed,
            })?;
        let (reply_tx, reply_rx) = oneshot::channel();

        request_tx
            .send(WorkerCommand::Memmove { request, reply_tx })
            .await
            .map_err(|_| AsyncMemmoveError::WorkerFailure {
                kind: AsyncWorkerFailureKind::RequestChannelClosed,
            })?;

        reply_rx
            .await
            .map_err(|_| AsyncMemmoveError::WorkerFailure {
                kind: AsyncWorkerFailureKind::ResponseChannelClosed,
            })?
            .map_err(AsyncMemmoveError::from)
    }

    /// Close the request channel and wait for the worker thread to exit.
    pub fn shutdown(mut self) -> Result<(), AsyncMemmoveError> {
        let _ = self.request_tx.take();
        if let Some(worker_thread) = self.worker_thread.take() {
            worker_thread
                .join()
                .map_err(|_| AsyncMemmoveError::WorkerFailure {
                    kind: AsyncWorkerFailureKind::WorkerPanicked,
                })?;
        }
        Ok(())
    }
}

impl Drop for AsyncDsaSession {
    fn drop(&mut self) {
        let _ = self.request_tx.take();
    }
}

fn run_memmove<W: AsyncMemmoveWorker>(
    worker: &mut W,
    request: AsyncMemmoveRequest,
) -> Result<AsyncMemmoveResult, MemmoveError> {
    let AsyncMemmoveRequest { src, dst_len } = request;
    let mut dst = vec![0u8; dst_len];
    let report = worker.memmove(&mut dst, &src)?;
    Ok(AsyncMemmoveResult { bytes: dst, report })
}
