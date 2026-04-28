//! Thin Intel DSA memmove bridge for `tonic-profile`.
//!
//! This crate deliberately stays narrow: it opens one IDXD work queue, submits
//! one memmove descriptor at a time through `idxd-sys`, retries recoverable
//! page faults, verifies copied bytes, and maps queue-open/completion failures
//! into typed Rust errors.
//!
//! Synchronous callers pass explicit source and destination slices to
//! `DsaSession::memmove`; request validation always treats the source length as
//! the requested transfer size and only requires destination capacity to be at
//! least that large. Async callers should use `AsyncMemmoveRequest::new` when
//! work must cross Tokio tasks or the worker thread: requests own a `bytes::Bytes`
//! source and a caller-provided `bytes::BytesMut` destination, and
//! `AsyncMemmoveResult` returns the owned destination plus validation report.
//! The async API intentionally has no public allocation convenience constructor
//! and no borrowed copy-back helper; destination allocation and ownership stay
//! explicit at the call site.

mod async_session;
mod direct_memmove;
mod validation;

pub use async_session::{
    AsyncDsaHandle, AsyncDsaSession, AsyncLifecycleFailureKind, AsyncMemmoveError,
    AsyncMemmoveRequest, AsyncMemmoveRequestError, AsyncMemmoveResult, AsyncMemmoveWorker,
    AsyncWorkerFailureKind,
};
pub use validation::{
    COMPLETION_TIMEOUT_STATUS, CompletionAction, CompletionSnapshot, DEFAULT_DEVICE_PATH,
    DEFAULT_MAX_PAGE_FAULT_RETRIES, MAX_MEMMOVE_BYTES, MemmoveError, MemmovePhase, MemmoveRequest,
    MemmoveRetry, MemmoveValidationConfig, MemmoveValidationReport, classify_memmove_completion,
};

use std::path::Path;

use bytes::buf::UninitSlice;
use direct_memmove::{DirectMemmoveState, verify_initialized_destination};
use idxd_sys::{WqPortal, poll_completion, touch_fault_page};

/// Thin reusable session over one mapped DSA work queue.
pub struct DsaSession {
    config: MemmoveValidationConfig,
    portal: WqPortal,
}

impl DsaSession {
    /// Open a DSA work queue and keep it mapped for repeated memmoves.
    pub fn open<P: AsRef<Path>>(device_path: P) -> Result<Self, MemmoveError> {
        Self::open_with_retries(device_path, DEFAULT_MAX_PAGE_FAULT_RETRIES)
    }

    pub fn open_default() -> Result<Self, MemmoveError> {
        Self::open(DEFAULT_DEVICE_PATH)
    }

    pub fn open_with_retries<P: AsRef<Path>>(
        device_path: P,
        max_page_fault_retries: u32,
    ) -> Result<Self, MemmoveError> {
        let config = MemmoveValidationConfig::with_retries(device_path, max_page_fault_retries)?;
        let portal =
            WqPortal::open(config.device_path()).map_err(|source| MemmoveError::QueueOpen {
                device_path: config.device_path().to_path_buf(),
                phase: MemmovePhase::QueueOpen,
                source,
            })?;

        Ok(Self { config, portal })
    }

    pub fn device_path(&self) -> &Path {
        self.config.device_path()
    }

    pub fn max_page_fault_retries(&self) -> u32 {
        self.config.max_page_fault_retries()
    }

    pub fn validation_config(&self) -> &MemmoveValidationConfig {
        &self.config
    }

    /// Submit one memmove over the mapped work queue.
    pub fn memmove(
        &self,
        dst: &mut [u8],
        src: &[u8],
    ) -> Result<MemmoveValidationReport, MemmoveError> {
        let request = MemmoveRequest::for_buffers(dst.len(), src.len())?;
        let report = self.memmove_inner(dst.as_mut_ptr(), src.as_ptr(), request)?;
        verify_initialized_destination(self.validation_config(), request, &report, dst, src)?;

        Ok(report)
    }

    /// Submit one memmove into caller-owned uninitialized writable capacity.
    pub(crate) fn memmove_uninit(
        &self,
        dst: &mut UninitSlice,
        src: &[u8],
    ) -> Result<MemmoveValidationReport, MemmoveError> {
        let request = MemmoveRequest::for_buffers(dst.len(), src.len())?;
        let report = self.memmove_inner(dst.as_mut_ptr(), src.as_ptr(), request)?;

        // SAFETY: A successful DSA memmove initializes exactly `request.len()`
        // bytes starting at `dst.as_mut_ptr()`. The validation above guarantees
        // that the exposed prefix is in bounds, and this read happens only after
        // success so the bytes are initialized for post-copy verification.
        let initialized_dst =
            unsafe { std::slice::from_raw_parts(dst.as_mut_ptr(), request.len()) };
        verify_initialized_destination(
            self.validation_config(),
            request,
            &report,
            initialized_dst,
            src,
        )?;

        Ok(report)
    }

    fn memmove_inner(
        &self,
        dst: *mut u8,
        src: *const u8,
        request: MemmoveRequest,
    ) -> Result<MemmoveValidationReport, MemmoveError> {
        // SAFETY: `DsaSession::memmove` and `memmove_uninit` validated that the
        // source and destination ranges cover `request.len()` bytes. Both calls
        // keep those buffers borrowed for this entire synchronous operation, so
        // the descriptor and completion record inside `state` cannot outlive the
        // memory referenced by hardware.
        let mut state = unsafe { DirectMemmoveState::new(src, dst, request) };

        loop {
            state.reset_and_fill_descriptor();

            unsafe {
                self.portal.submit(state.descriptor());
            }

            let polled_status = poll_completion(state.completion());
            let snapshot = CompletionSnapshot::from_record(state.completion(), polled_status);
            match state.classify_snapshot(self.validation_config(), snapshot)? {
                CompletionAction::Success => {
                    return state.success_report(self.device_path(), snapshot.status);
                }
                CompletionAction::Retry(retry) => {
                    touch_fault_page(state.completion());
                    state.apply_retry(retry);
                }
            }
        }
    }
}
