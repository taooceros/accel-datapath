use std::collections::HashMap;
use std::path::Path;
use std::sync::{
    Arc, Mutex, Weak,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;

use bytes::{Bytes, BytesMut, buf::UninitSlice};
use idxd_sys::{DsaHwDesc, EnqcmdSubmission, WqPortal, touch_fault_page};
use snafu::Snafu;
use tokio::sync::oneshot;

use crate::async_session::{AsyncMemmoveError, AsyncMemmoveRequest, AsyncMemmoveResult};
use crate::direct_memmove::{DirectMemmoveState, verify_initialized_destination};
use crate::{
    CompletionAction, CompletionSnapshot, MemmoveError, MemmoveRequest, MemmoveValidationConfig,
};

const DEFAULT_SUBMISSION_RETRY_BUDGET: u32 = 64;
const MONITOR_IDLE_BACKOFF: Duration = Duration::from_millis(1);
const SUBMISSION_BACKOFF: Duration = Duration::from_millis(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncDirectFailureKind {
    RegistrationClosed,
    MonitorClosed,
    SubmissionRejected,
    BackpressureExceeded,
    ReceiverDropped,
    RuntimeUnavailable,
}

impl AsyncDirectFailureKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RegistrationClosed => "registration_closed",
            Self::MonitorClosed => "monitor_closed",
            Self::SubmissionRejected => "submission_rejected",
            Self::BackpressureExceeded => "backpressure_exceeded",
            Self::ReceiverDropped => "receiver_dropped",
            Self::RuntimeUnavailable => "runtime_unavailable",
        }
    }
}

impl std::fmt::Display for AsyncDirectFailureKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Snafu)]
#[snafu(display("{kind} requested_bytes={requested_bytes} retry_count={retry_count} retry_budget={retry_budget}{completion_metadata}", completion_metadata = display_completion_snapshot(*completion_snapshot)))]
pub struct AsyncDirectFailure {
    kind: AsyncDirectFailureKind,
    requested_bytes: usize,
    retry_budget: u32,
    retry_count: u32,
    completion_snapshot: Option<CompletionSnapshot>,
}

fn display_completion_snapshot(snapshot: Option<CompletionSnapshot>) -> String {
    snapshot
        .map(|snapshot| {
            format!(
                " completion_status=0x{:02x} completion_result={} bytes_completed={} fault_addr=0x{:x}",
                snapshot.status, snapshot.result, snapshot.bytes_completed, snapshot.fault_addr
            )
        })
        .unwrap_or_default()
}

impl AsyncDirectFailure {
    fn new(
        kind: AsyncDirectFailureKind,
        requested_bytes: usize,
        retry_budget: u32,
        retry_count: u32,
        completion_snapshot: Option<CompletionSnapshot>,
    ) -> Self {
        Self {
            kind,
            requested_bytes,
            retry_budget,
            retry_count,
            completion_snapshot,
        }
    }

    pub fn kind(&self) -> AsyncDirectFailureKind {
        self.kind
    }

    pub fn requested_bytes(&self) -> usize {
        self.requested_bytes
    }

    pub fn retry_budget(&self) -> u32 {
        self.retry_budget
    }

    pub fn retry_count(&self) -> u32 {
        self.retry_count
    }

    pub fn completion_snapshot(&self) -> Option<CompletionSnapshot> {
        self.completion_snapshot
    }
}

pub trait DirectMemmoveBackend: Send + Sync + 'static {
    fn submit(&self, op_id: u64, descriptor: &DsaHwDesc) -> EnqcmdSubmission;

    fn completion_snapshot(
        &self,
        _op_id: u64,
        state: &DirectMemmoveState,
    ) -> Option<CompletionSnapshot> {
        state.completion_snapshot()
    }

    fn initialize_success_destination(&self, _op_id: u64, _dst: &mut UninitSlice, _src: &[u8]) {}
}

pub struct DirectPortalBackend {
    portal: WqPortal,
}

impl DirectPortalBackend {
    pub fn open<P: AsRef<Path>>(device_path: P) -> Result<Self, MemmoveError> {
        WqPortal::open(device_path.as_ref())
            .map(|portal| Self { portal })
            .map_err(|source| MemmoveError::QueueOpen {
                device_path: device_path.as_ref().to_path_buf(),
                phase: crate::MemmovePhase::QueueOpen,
                source,
            })
    }
}

impl DirectMemmoveBackend for DirectPortalBackend {
    fn submit(&self, _op_id: u64, descriptor: &DsaHwDesc) -> EnqcmdSubmission {
        // SAFETY: The direct runtime keeps the operation-owned descriptor,
        // completion record, source, and destination buffers alive in the
        // pending registry until the monitor observes terminal completion.
        unsafe { self.portal.submit_enqcmd_once(descriptor) }
    }
}

#[derive(Debug)]
pub struct DirectAsyncMemmoveRuntime<B> {
    inner: Arc<RuntimeInner<B>>,
}

impl<B> Clone for DirectAsyncMemmoveRuntime<B> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[derive(Debug)]
struct RuntimeInner<B> {
    config: MemmoveValidationConfig,
    backend: B,
    pending: Mutex<HashMap<u64, Arc<PendingOperation>>>,
    next_id: AtomicU64,
    closed: AtomicBool,
    submission_retry_budget: u32,
}

impl<B> DirectAsyncMemmoveRuntime<B>
where
    B: DirectMemmoveBackend,
{
    pub fn new(config: MemmoveValidationConfig, backend: B) -> Self {
        Self::with_submission_retry_budget(config, backend, DEFAULT_SUBMISSION_RETRY_BUDGET)
    }

    pub fn try_new(
        config: MemmoveValidationConfig,
        backend: B,
    ) -> Result<Self, AsyncDirectFailure> {
        Self::try_with_submission_retry_budget(config, backend, DEFAULT_SUBMISSION_RETRY_BUDGET)
    }

    pub fn with_submission_retry_budget(
        config: MemmoveValidationConfig,
        backend: B,
        submission_retry_budget: u32,
    ) -> Self {
        Self::try_with_submission_retry_budget(config, backend, submission_retry_budget)
            .expect("direct async runtime requires an active Tokio runtime")
    }

    pub fn try_with_submission_retry_budget(
        config: MemmoveValidationConfig,
        backend: B,
        submission_retry_budget: u32,
    ) -> Result<Self, AsyncDirectFailure> {
        let inner = Arc::new(RuntimeInner {
            config,
            backend,
            pending: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            closed: AtomicBool::new(false),
            submission_retry_budget,
        });

        let handle = tokio::runtime::Handle::try_current().map_err(|_| {
            AsyncDirectFailure::new(
                AsyncDirectFailureKind::RuntimeUnavailable,
                0,
                submission_retry_budget,
                0,
                None,
            )
        })?;
        handle.spawn(monitor_completion_records(Arc::downgrade(&inner)));

        Ok(Self { inner })
    }

    pub async fn memmove(
        &self,
        request: AsyncMemmoveRequest,
    ) -> Result<AsyncMemmoveResult, AsyncMemmoveError> {
        if self.inner.closed.load(Ordering::Acquire) {
            return Err(self.direct_error(
                AsyncDirectFailureKind::RegistrationClosed,
                request.requested_bytes(),
                0,
                None,
                Some(request),
            ));
        }

        let op_id = self.inner.next_id.fetch_add(1, Ordering::AcqRel);
        let (reply_tx, reply_rx) = oneshot::channel();
        let operation = Arc::new(PendingOperation::new(
            op_id,
            request,
            self.inner.config.clone(),
            reply_tx,
        )?);

        {
            let mut pending = self
                .inner
                .pending
                .lock()
                .expect("pending registry poisoned");
            if self.inner.closed.load(Ordering::Acquire) {
                let request = operation.recover_request();
                return Err(self.direct_error(
                    AsyncDirectFailureKind::RegistrationClosed,
                    operation.requested_bytes,
                    0,
                    None,
                    request,
                ));
            }
            pending.insert(op_id, Arc::clone(&operation));
        }

        let mut rejected = 0;
        loop {
            operation.reset_and_fill_descriptor();
            let submission = operation
                .with_descriptor(|descriptor| self.inner.backend.submit(op_id, descriptor));

            match submission {
                EnqcmdSubmission::Accepted => break,
                EnqcmdSubmission::Rejected => {
                    rejected += 1;
                    if rejected > self.inner.submission_retry_budget {
                        self.remove_pending(op_id);
                        let request = operation.recover_request();
                        return Err(self.direct_error(
                            AsyncDirectFailureKind::BackpressureExceeded,
                            operation.requested_bytes,
                            rejected,
                            None,
                            request,
                        ));
                    }
                    if rejected % 4 == 0 {
                        tokio::time::sleep(SUBMISSION_BACKOFF).await;
                    } else {
                        tokio::task::yield_now().await;
                    }
                }
            }
        }

        match reply_rx.await {
            Ok(result) => result,
            Err(_) => Err(self.direct_error(
                AsyncDirectFailureKind::MonitorClosed,
                operation.requested_bytes,
                0,
                None,
                None,
            )),
        }
    }

    pub fn close(&self) {
        if self.inner.closed.swap(true, Ordering::AcqRel) {
            return;
        }
        let operations = {
            let mut pending = self
                .inner
                .pending
                .lock()
                .expect("pending registry poisoned");
            pending.drain().map(|(_, op)| op).collect::<Vec<_>>()
        };
        for operation in operations {
            operation.finish(Err(self.direct_error(
                AsyncDirectFailureKind::MonitorClosed,
                operation.requested_bytes,
                0,
                None,
                operation.recover_request(),
            )));
        }
    }

    fn remove_pending(&self, op_id: u64) {
        self.inner
            .pending
            .lock()
            .expect("pending registry poisoned")
            .remove(&op_id);
    }

    fn direct_error(
        &self,
        kind: AsyncDirectFailureKind,
        requested_bytes: usize,
        retry_count: u32,
        completion_snapshot: Option<CompletionSnapshot>,
        request: Option<AsyncMemmoveRequest>,
    ) -> AsyncMemmoveError {
        AsyncMemmoveError::DirectFailure {
            failure: AsyncDirectFailure::new(
                kind,
                requested_bytes,
                self.inner.submission_retry_budget,
                retry_count,
                completion_snapshot,
            ),
            request,
        }
    }
}

impl<B> Drop for DirectAsyncMemmoveRuntime<B> {
    fn drop(&mut self) {
        if Arc::strong_count(&self.inner) != 1 {
            return;
        }
        self.inner.closed.store(true, Ordering::Release);
        let operations = {
            let mut pending = self
                .inner
                .pending
                .lock()
                .expect("pending registry poisoned");
            pending.drain().map(|(_, op)| op).collect::<Vec<_>>()
        };
        for operation in operations {
            operation.finish(Err(AsyncMemmoveError::DirectFailure {
                failure: AsyncDirectFailure::new(
                    AsyncDirectFailureKind::MonitorClosed,
                    operation.requested_bytes,
                    self.inner.submission_retry_budget,
                    0,
                    None,
                ),
                request: operation.recover_request(),
            }));
        }
    }
}

async fn monitor_completion_records<B>(inner: Weak<RuntimeInner<B>>)
where
    B: DirectMemmoveBackend,
{
    loop {
        let Some(inner) = inner.upgrade() else {
            return;
        };

        let operations = {
            let pending = inner.pending.lock().expect("pending registry poisoned");
            pending.values().cloned().collect::<Vec<_>>()
        };

        if operations.is_empty() {
            if inner.closed.load(Ordering::Acquire) {
                return;
            }
            drop(inner);
            tokio::time::sleep(MONITOR_IDLE_BACKOFF).await;
            continue;
        }

        for operation in operations {
            if let Some(snapshot) = operation.completion_snapshot(&inner.backend) {
                let terminal = operation.handle_snapshot(&inner, snapshot).await;
                if terminal {
                    inner
                        .pending
                        .lock()
                        .expect("pending registry poisoned")
                        .remove(&operation.id);
                }
            }
        }

        drop(inner);
        tokio::task::yield_now().await;
    }
}

struct PendingOperation {
    id: u64,
    requested_bytes: usize,
    config: MemmoveValidationConfig,
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
    fn new(
        id: u64,
        request: AsyncMemmoveRequest,
        config: MemmoveValidationConfig,
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

    fn reset_and_fill_descriptor(&self) {
        self.state
            .lock()
            .expect("direct memmove state poisoned")
            .reset_and_fill_descriptor();
    }

    fn with_descriptor<R>(&self, f: impl FnOnce(&DsaHwDesc) -> R) -> R {
        let state = self.state.lock().expect("direct memmove state poisoned");
        f(state.descriptor())
    }

    fn completion_snapshot<B>(&self, backend: &B) -> Option<CompletionSnapshot>
    where
        B: DirectMemmoveBackend,
    {
        let state = self.state.lock().expect("direct memmove state poisoned");
        backend.completion_snapshot(self.id, &state)
    }

    async fn handle_snapshot<B>(
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

    fn finish(&self, result: Result<AsyncMemmoveResult, AsyncMemmoveError>) {
        if let Some(reply_tx) = self.reply_tx.lock().expect("reply lock poisoned").take() {
            let _ = reply_tx.send(result);
        }
    }

    fn recover_request(&self) -> Option<AsyncMemmoveRequest> {
        let source = self.source.lock().expect("source lock poisoned").take()?;
        let destination = self
            .destination
            .lock()
            .expect("destination lock poisoned")
            .take()?;
        AsyncMemmoveRequest::new(source, destination).ok()
    }
}

#[doc(hidden)]
pub mod test_support {
    use std::collections::{HashMap, VecDeque};
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    use bytes::buf::UninitSlice;
    use idxd_sys::{DsaHwDesc, EnqcmdSubmission};

    use crate::CompletionSnapshot;

    use super::DirectMemmoveBackend;

    #[derive(Debug, Clone)]
    pub struct ScriptedDirectBackend {
        inner: Arc<ScriptedInner>,
    }

    #[derive(Debug, Default)]
    struct ScriptedInner {
        submissions: AtomicUsize,
        completions: AtomicUsize,
        scripts: Mutex<VecDeque<EnqcmdSubmission>>,
        snapshots: Mutex<HashMap<u64, CompletionSnapshot>>,
        copy_on_success: bool,
    }

    impl ScriptedDirectBackend {
        pub fn new() -> Self {
            Self {
                inner: Arc::new(ScriptedInner {
                    copy_on_success: true,
                    ..ScriptedInner::default()
                }),
            }
        }

        pub fn with_submissions(submissions: impl IntoIterator<Item = EnqcmdSubmission>) -> Self {
            let backend = Self::new();
            *backend.inner.scripts.lock().expect("script lock poisoned") =
                submissions.into_iter().collect();
            backend
        }

        pub fn submissions(&self) -> usize {
            self.inner.submissions.load(Ordering::SeqCst)
        }

        pub fn completions(&self) -> usize {
            self.inner.completions.load(Ordering::SeqCst)
        }

        pub fn complete(&self, op_id: u64, snapshot: CompletionSnapshot) {
            self.inner
                .snapshots
                .lock()
                .expect("snapshot lock poisoned")
                .insert(op_id, snapshot);
        }

        pub fn clear_completion(&self, op_id: u64) {
            self.inner
                .snapshots
                .lock()
                .expect("snapshot lock poisoned")
                .remove(&op_id);
        }

        pub fn zero_success_copy() -> Self {
            Self {
                inner: Arc::new(ScriptedInner {
                    copy_on_success: false,
                    ..ScriptedInner::default()
                }),
            }
        }
    }

    impl Default for ScriptedDirectBackend {
        fn default() -> Self {
            Self::new()
        }
    }

    impl DirectMemmoveBackend for ScriptedDirectBackend {
        fn submit(&self, _op_id: u64, _descriptor: &DsaHwDesc) -> EnqcmdSubmission {
            self.inner.submissions.fetch_add(1, Ordering::SeqCst);
            self.inner
                .scripts
                .lock()
                .expect("script lock poisoned")
                .pop_front()
                .unwrap_or(EnqcmdSubmission::Accepted)
        }

        fn completion_snapshot(
            &self,
            op_id: u64,
            _state: &super::DirectMemmoveState,
        ) -> Option<CompletionSnapshot> {
            self.inner
                .snapshots
                .lock()
                .expect("snapshot lock poisoned")
                .remove(&op_id)
        }

        fn initialize_success_destination(&self, _op_id: u64, dst: &mut UninitSlice, src: &[u8]) {
            self.inner.completions.fetch_add(1, Ordering::SeqCst);
            if self.inner.copy_on_success {
                dst.copy_from_slice(src);
            } else {
                let zeros = vec![0; src.len()];
                dst.copy_from_slice(&zeros);
            }
        }
    }
}
