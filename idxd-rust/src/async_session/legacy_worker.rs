//! Hidden legacy async worker fixture.
//!
//! The public async path uses the direct completion-driven runtime. This module
//! keeps the old blocking-worker fixture isolated so tests can still model the
//! compatibility failure classes without making that path look like the active
//! implementation.

use std::sync::Arc;
use std::thread::{self, JoinHandle};

use bytes::buf::UninitSlice;
use tokio::sync::{mpsc, oneshot};

use crate::{DsaSession, MemmoveError, MemmoveValidationReport};

use super::{
    AsyncMemmoveDriver, AsyncMemmoveError, AsyncMemmoveRequest, AsyncMemmoveResult, DriverFuture,
};

/// Narrow async-structural failure kinds reserved for the legacy worker fixture.
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

enum WorkerCommand {
    Memmove {
        request: AsyncMemmoveRequest,
        reply_tx: oneshot::Sender<Result<AsyncMemmoveResult, AsyncMemmoveError>>,
    },
    Shutdown,
}

#[derive(Debug)]
pub(super) struct WorkerRuntimeDriver {
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

pub(super) fn spawn_worker_fixture<F, W>(
    factory: F,
) -> Result<(Arc<dyn AsyncMemmoveDriver>, JoinHandle<()>), AsyncMemmoveError>
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
        Ok(Ok(())) => Ok((Arc::new(WorkerRuntimeDriver { request_tx }), worker_thread)),
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
