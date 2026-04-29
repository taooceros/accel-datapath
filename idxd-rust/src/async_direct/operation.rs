use std::sync::{Mutex, atomic::Ordering};

use bytes::{Bytes, BytesMut, buf::UninitSlice};
use idxd_sys::{DsaHwDesc, EnqcmdSubmission, touch_fault_page};
use tokio::sync::oneshot;

use crate::async_session::{AsyncMemmoveError, AsyncMemmoveRequest, AsyncMemmoveResult};
use crate::direct_memmove::{DirectMemmoveState, verify_initialized_destination};
use crate::{CompletionAction, CompletionSnapshot, DsaConfig, MemmoveRequest};

use super::{
    AsyncDirectFailure, AsyncDirectFailureKind, DirectMemmoveBackend, RuntimeInner,
    SUBMISSION_BACKOFF,
};

pub(super) struct PendingOperation {
    id: u64,
    requested_bytes: usize,
    config: DsaConfig,
    source: Mutex<Option<Bytes>>,
    destination: Mutex<Option<BytesMut>>,
    state: Mutex<DirectMemmoveState>,
    last_snapshot: Mutex<Option<CompletionSnapshot>>,
    reply_tx: Mutex<Option<oneshot::Sender<Result<AsyncMemmoveResult, AsyncMemmoveError>>>>,
}

impl std::fmt::Debug for PendingOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingOperation")
            .field("id", &self.id)
            .field("requested_bytes", &self.requested_bytes)
            .finish_non_exhaustive()
    }
}

impl PendingOperation {
    pub(super) fn id(&self) -> u64 {
        self.id
    }

    pub(super) fn requested_bytes(&self) -> usize {
        self.requested_bytes
    }

    pub(super) fn new(
        id: u64,
        request: AsyncMemmoveRequest,
        config: DsaConfig,
        reply_tx: oneshot::Sender<Result<AsyncMemmoveResult, AsyncMemmoveError>>,
    ) -> Result<Self, AsyncMemmoveError> {
        let (source, mut destination) = request.into_parts();
        let requested_bytes = source.len();
        let request =
            MemmoveRequest::for_buffers(destination.spare_capacity_mut().len(), requested_bytes)
                .map_err(|source_error| AsyncMemmoveError::Memmove {
                    source: source_error,
                    request: Some(
                        AsyncMemmoveRequest::new(source.clone(), destination.clone())
                            .expect("request was already constructor-validated"),
                    ),
                })?;

        // SAFETY: `source` and `destination` are stored in this operation and
        // remain owned until terminal completion or pre-acceptance recovery. The
        // raw destination pointer targets the current spare capacity and the
        // operation never reallocates the destination while hardware may own a
        // descriptor.
        let state = unsafe {
            DirectMemmoveState::new(
                source.as_ptr(),
                destination.spare_capacity_mut().as_mut_ptr().cast::<u8>(),
                request,
            )
        };

        Ok(Self {
            id,
            requested_bytes,
            config,
            source: Mutex::new(Some(source)),
            destination: Mutex::new(Some(destination)),
            state: Mutex::new(state),
            last_snapshot: Mutex::new(None),
            reply_tx: Mutex::new(Some(reply_tx)),
        })
    }

    pub(super) fn reset_and_fill_descriptor(&self) {
        self.state
            .lock()
            .expect("direct memmove state poisoned")
            .reset_and_fill_descriptor();
    }

    pub(super) fn with_descriptor<R>(&self, f: impl FnOnce(&DsaHwDesc) -> R) -> R {
        let state = self.state.lock().expect("direct memmove state poisoned");
        f(state.descriptor())
    }

    pub(super) fn completion_snapshot<B>(&self, backend: &B) -> Option<CompletionSnapshot>
    where
        B: DirectMemmoveBackend,
    {
        let state = self.state.lock().expect("direct memmove state poisoned");
        backend.completion_snapshot(self.id, &state)
    }

    pub(super) async fn handle_snapshot<B>(
        &self,
        inner: &RuntimeInner<B>,
        snapshot: CompletionSnapshot,
    ) -> bool
    where
        B: DirectMemmoveBackend,
    {
        *self.last_snapshot.lock().expect("snapshot lock poisoned") = Some(snapshot);

        let action = {
            let state = self.state.lock().expect("direct memmove state poisoned");
            state.classify_snapshot(&self.config, snapshot)
        };

        match action {
            Ok(CompletionAction::Success) => {
                self.finish_success(inner, snapshot.status);
                true
            }
            Ok(CompletionAction::Retry(retry)) => {
                {
                    let mut state = self.state.lock().expect("direct memmove state poisoned");
                    touch_fault_page(state.completion());
                    state.apply_retry(retry);
                    state.reset_and_fill_descriptor();
                }

                match self.submit_continuation(inner).await {
                    Ok(()) => false,
                    Err(error) => {
                        self.finish(Err(error));
                        true
                    }
                }
            }
            Err(error) => {
                self.finish(Err(AsyncMemmoveError::Memmove {
                    source: error,
                    request: self.recover_request(),
                }));
                true
            }
        }
    }

    async fn submit_continuation<B>(&self, inner: &RuntimeInner<B>) -> Result<(), AsyncMemmoveError>
    where
        B: DirectMemmoveBackend,
    {
        let mut rejected = 0;
        loop {
            if inner.closed.load(Ordering::Acquire) {
                return Err(AsyncMemmoveError::DirectFailure {
                    failure: AsyncDirectFailure::new(
                        AsyncDirectFailureKind::MonitorClosed,
                        self.requested_bytes,
                        inner.submission_retry_budget,
                        self.retry_count(),
                        self.snapshot_for_error(),
                    ),
                    request: self.recover_request(),
                });
            }

            let submission =
                self.with_descriptor(|descriptor| inner.backend.submit(self.id, descriptor));
            match submission {
                EnqcmdSubmission::Accepted => return Ok(()),
                EnqcmdSubmission::Rejected => {
                    rejected += 1;
                    if rejected > inner.submission_retry_budget {
                        return Err(AsyncMemmoveError::DirectFailure {
                            failure: AsyncDirectFailure::new(
                                AsyncDirectFailureKind::BackpressureExceeded,
                                self.requested_bytes,
                                inner.submission_retry_budget,
                                self.retry_count(),
                                self.snapshot_for_error(),
                            ),
                            request: self.recover_request(),
                        });
                    }
                    if rejected % 4 == 0 {
                        tokio::time::sleep(SUBMISSION_BACKOFF).await;
                    } else {
                        tokio::task::yield_now().await;
                    }
                }
            }
        }
    }

    fn finish_success<B>(&self, inner: &RuntimeInner<B>, final_status: u8)
    where
        B: DirectMemmoveBackend,
    {
        let source = self
            .source
            .lock()
            .expect("source lock poisoned")
            .take()
            .expect("source should be present until terminal completion");
        let mut destination = self
            .destination
            .lock()
            .expect("destination lock poisoned")
            .take()
            .expect("destination should be present until terminal completion");
        let original_len = destination.len();

        {
            let spare = destination.spare_capacity_mut();
            let dst: &mut UninitSlice = (&mut spare[..self.requested_bytes]).into();
            inner
                .backend
                .initialize_success_destination(self.id, dst, &source);
        }

        let report = {
            let state = self.state.lock().expect("direct memmove state poisoned");
            state.success_report(inner.config.device_path(), final_status)
        };

        let result = report
            .and_then(|report| {
                // SAFETY: Terminal success means hardware or the test backend has
                // initialized exactly `requested_bytes` bytes in the destination
                // spare capacity. The slice is used only for validation, and the
                // destination length is advanced only after validation succeeds.
                let initialized_dst = unsafe {
                    std::slice::from_raw_parts(
                        destination.as_ptr().add(original_len),
                        self.requested_bytes,
                    )
                };
                verify_initialized_destination(
                    &inner.config,
                    MemmoveRequest::new(self.requested_bytes)?,
                    &report,
                    initialized_dst,
                    &source,
                )?;

                // SAFETY: Post-copy verification above read exactly the
                // initialized bytes written by the terminally successful
                // operation, so exposing the appended range is now safe.
                unsafe {
                    destination.set_len(original_len + self.requested_bytes);
                }

                Ok(AsyncMemmoveResult {
                    destination,
                    report,
                })
            })
            .map_err(|source| AsyncMemmoveError::Memmove {
                source,
                request: None,
            });

        self.finish(result);
    }

    fn retry_count(&self) -> u32 {
        self.state
            .lock()
            .expect("direct memmove state poisoned")
            .retries()
    }

    fn snapshot_for_error(&self) -> Option<CompletionSnapshot> {
        *self.last_snapshot.lock().expect("snapshot lock poisoned")
    }

    pub(super) fn finish(&self, result: Result<AsyncMemmoveResult, AsyncMemmoveError>) {
        if let Some(reply_tx) = self.reply_tx.lock().expect("reply lock poisoned").take() {
            let _ = reply_tx.send(result);
        }
    }

    pub(super) fn recover_request(&self) -> Option<AsyncMemmoveRequest> {
        let source = self.source.lock().expect("source lock poisoned").take()?;
        let destination = self
            .destination
            .lock()
            .expect("destination lock poisoned")
            .take()?;
        AsyncMemmoveRequest::new(source, destination).ok()
    }
}
