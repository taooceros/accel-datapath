//! Thin Intel DSA memmove bridge for `tonic-profile`.
//!
//! This crate deliberately stays narrow: it opens one IDXD work queue, submits
//! one memmove descriptor at a time through `idxd-bindings`, retries recoverable
//! page faults, and maps queue-open/completion failures into typed Rust errors.

mod validation;

pub use validation::{
    classify_memmove_completion, CompletionAction, CompletionSnapshot, MemmoveError,
    MemmovePhase, MemmoveRequest, MemmoveRetry, MemmoveValidationConfig,
    MemmoveValidationReport, COMPLETION_TIMEOUT_STATUS, DEFAULT_DEVICE_PATH,
    DEFAULT_MAX_PAGE_FAULT_RETRIES, MAX_MEMMOVE_BYTES,
};

use std::path::Path;

use idxd_bindings::{
    poll_completion, reset_completion, touch_fault_page, DsaCompletionRecord, DsaHwDesc, WqPortal,
};

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
        let portal = WqPortal::open(config.device_path()).map_err(|source| MemmoveError::QueueOpen {
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
        self.memmove_inner(dst.as_mut_ptr(), src.as_ptr(), request)
    }

    fn memmove_inner(
        &self,
        dst: *mut u8,
        src: *const u8,
        request: MemmoveRequest,
    ) -> Result<MemmoveValidationReport, MemmoveError> {
        let mut state = PendingMemmove::new(src, dst, request);
        let mut retries = 0;

        loop {
            let mut desc = DsaHwDesc::default();
            let mut comp = DsaCompletionRecord::default();
            reset_completion(&mut comp);
            state.fill_descriptor(&mut desc, &mut comp);

            unsafe {
                self.portal.submit(&desc);
            }

            let polled_status = poll_completion(&comp);
            let snapshot = CompletionSnapshot::from_record(&comp, polled_status);
            match classify_memmove_completion(
                self.validation_config(),
                state.remaining(),
                snapshot,
                retries,
            )? {
                CompletionAction::Success => {
                    return MemmoveValidationReport::new(
                        self.device_path(),
                        request,
                        retries,
                        snapshot.status,
                    );
                }
                CompletionAction::Retry(retry) => {
                    touch_fault_page(&comp);
                    state.apply_retry(retry);
                    retries += 1;
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PendingMemmove {
    src: *const u8,
    dst: *mut u8,
    remaining: u32,
}

impl PendingMemmove {
    fn new(src: *const u8, dst: *mut u8, request: MemmoveRequest) -> Self {
        Self {
            src,
            dst,
            remaining: request.len() as u32,
        }
    }

    fn remaining(&self) -> usize {
        self.remaining as usize
    }

    fn fill_descriptor(&self, desc: &mut DsaHwDesc, comp: &mut DsaCompletionRecord) {
        desc.fill_memmove(self.src, self.dst, self.remaining);
        desc.set_completion(comp);
    }

    fn apply_retry(&mut self, retry: MemmoveRetry) {
        self.src = self.src.wrapping_add(retry.next_src_offset);
        self.dst = self.dst.wrapping_add(retry.next_dst_offset);
        self.remaining = retry.remaining_bytes as u32;
    }
}
