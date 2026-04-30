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
//! work must cross Tokio tasks: requests own a `bytes::Bytes` source and a
//! caller-provided `bytes::BytesMut` destination, and `AsyncMemmoveResult`
//! returns the owned destination plus validation report after direct completion
//! record observation. The async API intentionally has no public allocation
//! convenience constructor and no borrowed copy-back helper; destination
//! allocation and ownership stay explicit at the call site.
//!
//! `IdxdSession<Accel>` is the generic IDXD architecture direction for the sealed `Dsa`
//! and `Iax`/`Iaa` marker families. It opens one work queue and now carries narrow
//! representative operations: `IdxdSession<Dsa>::memmove` reuses the same blocking DSA
//! lifecycle as `DsaSession`, while `IdxdSession<Iax>::crc64`/`IdxdSession<Iaa>::crc64`
//! use an IAX-owned descriptor/completion interpreter. This is intentionally not full
//! DSA/IAX coverage and does not introduce a public operation trait hierarchy or runtime
//! accelerator dispatch.

mod async_direct;
mod async_session;
mod direct_memmove;
mod iax_crc64;
mod lifecycle;
mod session;
mod validation;

#[doc(hidden)]
pub use async_direct::test_support as direct_test_support;
pub use async_direct::{
    AsyncDirectFailure, AsyncDirectFailureKind, DirectAsyncMemmoveRuntime, DirectMemmoveBackend,
    DirectPortalBackend,
};
pub use async_session::{
    AsyncDsaHandle, AsyncDsaSession, AsyncLifecycleFailureKind, AsyncMemmoveError,
    AsyncMemmoveRequest, AsyncMemmoveRequestError, AsyncMemmoveResult, AsyncMemmoveWorker,
    AsyncWorkerFailureKind,
};
pub use iax_crc64::{
    IAX_CRC64_COMPLETION_TIMEOUT_STATUS, IaxCrc64Error, IaxCrc64Phase, IaxCrc64Report,
    IaxCrc64Result, MAX_IAX_CRC64_BYTES,
};
pub use session::{Accelerator, Dsa, Iaa, Iax, IdxdSession, IdxdSessionConfig, IdxdSessionError};
pub use validation::{
    COMPLETION_TIMEOUT_STATUS, CompletionAction, CompletionSnapshot, DEFAULT_DEVICE_PATH,
    DEFAULT_MAX_PAGE_FAULT_RETRIES, DsaConfig, MAX_MEMMOVE_BYTES, MemmoveError, MemmovePhase,
    MemmoveRequest, MemmoveRetry, MemmoveValidationReport, classify_memmove_completion,
};

use std::path::Path;

use bytes::buf::UninitSlice;
use direct_memmove::{run_direct_memmove, verify_initialized_destination};
use idxd_sys::WqPortal;

/// Thin reusable session over one mapped DSA work queue.
pub struct DsaSession {
    config: DsaConfig,
    portal: WqPortal,
}

#[bon::bon]
impl DsaSession {
    /// Open a DSA work queue and keep it mapped for repeated memmoves.
    pub fn open<P: AsRef<Path>>(device_path: P) -> Result<Self, MemmoveError> {
        Self::open_with_retries(device_path, DEFAULT_MAX_PAGE_FAULT_RETRIES)
    }

    pub fn open_with_retries<P: AsRef<Path>>(
        device_path: P,
        max_page_fault_retries: u32,
    ) -> Result<Self, MemmoveError> {
        let config = DsaConfig::builder()
            .device_path(device_path.as_ref().to_path_buf())
            .max_page_fault_retries(max_page_fault_retries)
            .build()?;
        Self::open_config(config)
    }

    /// Open a DSA work queue from an already-normalized DSA config.
    ///
    /// The generated `DsaSession::builder().open()` path is kept as a named
    /// way to supply a prebuilt config while preserving the same queue-open
    /// device path and phase diagnostics as the direct constructor helpers.
    #[builder(start_fn = builder, finish_fn = open)]
    pub fn open_config(#[builder(default)] dsa_config: DsaConfig) -> Result<Self, MemmoveError> {
        let portal =
            WqPortal::open(dsa_config.device_path()).map_err(|source| MemmoveError::QueueOpen {
                device_path: dsa_config.device_path().to_path_buf(),
                phase: MemmovePhase::QueueOpen,
                source,
            })?;

        Ok(Self {
            config: dsa_config,
            portal,
        })
    }

    pub fn device_path(&self) -> &Path {
        self.config.device_path()
    }

    pub fn max_page_fault_retries(&self) -> u32 {
        self.config.max_page_fault_retries()
    }

    pub fn dsa_config(&self) -> &DsaConfig {
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
        verify_initialized_destination(self.dsa_config(), request, &report, dst, src)?;

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
        verify_initialized_destination(self.dsa_config(), request, &report, initialized_dst, src)?;

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
        // the lifecycle-owned descriptor and completion record cannot outlive
        // the memory referenced by hardware.
        unsafe { run_direct_memmove(&self.portal, self.dsa_config(), src, dst, request) }
    }
}
