//! Compatibility DSA memmove session kept separate from the generic IDXD seam.
//!
//! This module contains the established `DsaSession` API so the crate root can
//! stay focused on the current generic `IdxdSession<Accel>` direction while
//! preserving downstream compatibility.

use std::path::Path;

use bytes::buf::UninitSlice;
use idxd_sys::WqPortal;

use crate::direct_memmove::{run_direct_memmove, verify_initialized_destination};
use crate::validation::{
    DEFAULT_MAX_PAGE_FAULT_RETRIES, DsaConfig, MemmoveCompletion, MemmoveError, MemmoveRequest,
    MemmoveValidationReport,
};

/// Thin reusable compatibility session over one mapped DSA work queue.
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
                phase: crate::MemmovePhase::QueueOpen,
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
        let completion = MemmoveCompletion::from(&report);
        verify_initialized_destination(self.dsa_config(), request, &completion, dst, src)?;

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
        let completion = MemmoveCompletion::from(&report);
        verify_initialized_destination(
            self.dsa_config(),
            request,
            &completion,
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
        // the lifecycle-owned descriptor and completion record cannot outlive
        // the memory referenced by hardware.
        unsafe { run_direct_memmove(&self.portal, self.dsa_config(), src, dst, request) }
    }
}
