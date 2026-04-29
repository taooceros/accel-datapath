use std::path::Path;

use idxd_sys::{
    DSA_COMP_NONE, DSA_COMP_STATUS_MASK, DsaCompletionRecord, DsaHwDesc, reset_completion,
};

use crate::{
    CompletionAction, CompletionSnapshot, MemmoveError, MemmovePhase, MemmoveRequest, MemmoveRetry,
    MemmoveValidationConfig, MemmoveValidationReport, classify_memmove_completion,
};

/// Operation-local memmove state that can be submitted now and completed later.
///
/// This type owns the descriptor and completion record that hardware references
/// after ENQCMD acceptance. The source and destination buffers themselves are
/// owned by the eventual async operation; this helper stores only the current
/// raw continuation pointers and remaining byte count so retry descriptors can
/// be rebuilt without duplicating completion classification logic.
#[doc(hidden)]
pub struct DirectMemmoveState {
    desc: DsaHwDesc,
    comp: DsaCompletionRecord,
    src: *const u8,
    dst: *mut u8,
    request: MemmoveRequest,
    remaining: u32,
    retries: u32,
}

// SAFETY: `DirectMemmoveState` owns the descriptor and completion record while
// storing raw continuation pointers into buffers owned by the surrounding
// operation. Moving the state between Tokio worker threads does not invalidate
// those allocation-backed pointers; callers must still keep the operation-owned
// buffers alive until every accepted descriptor has completed.
unsafe impl Send for DirectMemmoveState {}

impl DirectMemmoveState {
    /// Create reusable descriptor/completion state for one validated request.
    ///
    /// # Safety
    /// The caller must keep `src..src + request.len()` allocated and immutable,
    /// and `dst..dst + request.len()` allocated, writable, and not exposed as
    /// initialized destination bytes while hardware may own a submitted
    /// descriptor. The returned state must outlive every accepted descriptor that
    /// references its descriptor and completion record.
    pub(crate) unsafe fn new(src: *const u8, dst: *mut u8, request: MemmoveRequest) -> Self {
        Self {
            desc: DsaHwDesc::default(),
            comp: DsaCompletionRecord::default(),
            src,
            dst,
            request,
            remaining: request.len() as u32,
            retries: 0,
        }
    }

    pub(crate) fn remaining(&self) -> usize {
        self.remaining as usize
    }

    #[allow(dead_code)]
    pub(crate) fn retries(&self) -> u32 {
        self.retries
    }

    pub(crate) fn descriptor(&self) -> &DsaHwDesc {
        &self.desc
    }

    pub(crate) fn completion(&self) -> &DsaCompletionRecord {
        &self.comp
    }

    /// Reset the operation-owned completion record and rebuild the descriptor
    /// for the current source/destination offsets.
    pub(crate) fn reset_and_fill_descriptor(&mut self) {
        reset_completion(&mut self.comp);
        self.desc.fill_memmove(self.src, self.dst, self.remaining);
        self.desc.set_completion(&mut self.comp);
    }

    /// Read the completion record once without polling or spinning.
    #[allow(dead_code)]
    pub(crate) fn completion_snapshot(&self) -> Option<CompletionSnapshot> {
        let status = self.comp.status();
        if status == DSA_COMP_NONE {
            None
        } else {
            Some(CompletionSnapshot::from_record(
                &self.comp,
                status & DSA_COMP_STATUS_MASK,
            ))
        }
    }

    /// Classify an externally supplied snapshot with the shared memmove
    /// completion interpreter.
    pub(crate) fn classify_snapshot(
        &self,
        config: &MemmoveValidationConfig,
        snapshot: CompletionSnapshot,
    ) -> Result<CompletionAction, MemmoveError> {
        classify_memmove_completion(config, self.remaining(), snapshot, self.retries)
    }

    /// Apply a validated retry continuation to the raw descriptor pointers.
    pub(crate) fn apply_retry(&mut self, retry: MemmoveRetry) {
        self.src = self.src.wrapping_add(retry.next_src_offset);
        self.dst = self.dst.wrapping_add(retry.next_dst_offset);
        self.remaining = retry.remaining_bytes as u32;
        self.retries += 1;
    }

    pub(crate) fn success_report<P: AsRef<Path>>(
        &self,
        device_path: P,
        final_status: u8,
    ) -> Result<MemmoveValidationReport, MemmoveError> {
        MemmoveValidationReport::new(device_path, self.request, self.retries, final_status)
    }
}

/// Verify initialized destination bytes after a terminal success without
/// exposing buffer contents in the typed error.
pub(crate) fn verify_initialized_destination(
    config: &MemmoveValidationConfig,
    request: MemmoveRequest,
    report: &MemmoveValidationReport,
    initialized_dst: &[u8],
    src: &[u8],
) -> Result<(), MemmoveError> {
    if let Some(mismatch_offset) = initialized_dst
        .iter()
        .take(request.len())
        .zip(src.iter().take(request.len()))
        .position(|(actual, expected)| actual != expected)
    {
        return Err(MemmoveError::ByteMismatch {
            device_path: config.device_path().to_path_buf(),
            phase: MemmovePhase::PostCopyVerify,
            requested_bytes: request.len(),
            mismatch_offset,
            final_status: report.final_status,
            page_fault_retries: report.page_fault_retries,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use idxd_sys::{DSA_COMP_PAGE_FAULT_NOBOF, DSA_COMP_SUCCESS};

    use super::*;

    fn test_config() -> MemmoveValidationConfig {
        MemmoveValidationConfig::builder()
            .device_path("/dev/dsa/wq0.0")
            .max_page_fault_retries(1)
            .build()
            .expect("test config")
    }

    fn state_for(src: &[u8], dst: &mut [u8]) -> DirectMemmoveState {
        let request = MemmoveRequest::for_buffers(dst.len(), src.len()).expect("valid request");
        // SAFETY: The test-owned slices outlive the state and are never submitted
        // to hardware. The raw pointers are used only for descriptor field
        // construction and offset assertions.
        unsafe { DirectMemmoveState::new(src.as_ptr(), dst.as_mut_ptr(), request) }
    }

    #[test]
    fn direct_state_owns_aligned_descriptor_and_completion_record() {
        let src = [0x11; 64];
        let mut dst = [0; 64];
        let mut state = state_for(&src, &mut dst);

        state.reset_and_fill_descriptor();

        assert_eq!(state.descriptor().src_addr(), src.as_ptr() as u64);
        assert_eq!(state.descriptor().dst_addr(), dst.as_mut_ptr() as u64);
        assert_eq!(state.descriptor().xfer_size(), 64);
        assert_eq!(
            state.descriptor().completion_addr(),
            state.completion() as *const DsaCompletionRecord as u64
        );
        assert_eq!(state.completion_snapshot(), None);
    }

    #[test]
    fn retry_offsets_rebuild_continuation_descriptor() {
        let src = [0x22; 128];
        let mut dst = [0; 128];
        let mut state = state_for(&src, &mut dst);

        state.apply_retry(MemmoveRetry {
            next_src_offset: 32,
            next_dst_offset: 32,
            remaining_bytes: 96,
        });
        state.reset_and_fill_descriptor();

        assert_eq!(state.retries(), 1);
        assert_eq!(state.remaining(), 96);
        assert_eq!(state.descriptor().src_addr(), unsafe {
            src.as_ptr().add(32) as u64
        });
        assert_eq!(state.descriptor().dst_addr(), unsafe {
            dst.as_mut_ptr().add(32) as u64
        });
        assert_eq!(state.descriptor().xfer_size(), 96);
    }

    #[test]
    fn snapshot_classification_reuses_validation_interpreter() {
        let src = [0x33; 128];
        let mut dst = [0; 128];
        let state = state_for(&src, &mut dst);

        let action = state
            .classify_snapshot(
                &test_config(),
                CompletionSnapshot::new(DSA_COMP_PAGE_FAULT_NOBOF, 0, 16, 0x1000),
            )
            .expect("page fault should be retryable");

        assert_eq!(
            action,
            CompletionAction::Retry(MemmoveRetry {
                next_src_offset: 16,
                next_dst_offset: 16,
                remaining_bytes: 112,
            })
        );
    }

    #[test]
    fn success_report_preserves_request_and_retry_metadata() {
        let src = [0x44; 16];
        let mut dst = [0; 16];
        let mut state = state_for(&src, &mut dst);
        state.apply_retry(MemmoveRetry {
            next_src_offset: 8,
            next_dst_offset: 8,
            remaining_bytes: 8,
        });

        let report = state
            .success_report(test_config().device_path(), DSA_COMP_SUCCESS)
            .expect("report should build");

        assert_eq!(report.requested_bytes, 16);
        assert_eq!(report.page_fault_retries, 1);
        assert_eq!(report.final_status, DSA_COMP_SUCCESS);
    }

    #[test]
    fn post_copy_verification_reports_metadata_without_buffer_contents_or_destination_length() {
        let config = test_config();
        let request = MemmoveRequest::new(4).expect("request");
        let report =
            MemmoveValidationReport::new(config.device_path(), request, 1, DSA_COMP_SUCCESS)
                .expect("report");
        let err =
            verify_initialized_destination(&config, request, &report, &[1, 2, 9, 4], &[1, 2, 3, 4])
                .expect_err("mismatch should fail");

        assert_eq!(err.kind(), "byte_mismatch");
        assert_eq!(err.phase(), Some(MemmovePhase::PostCopyVerify));
        assert_eq!(err.requested_bytes(), Some(4));
        assert_eq!(err.final_status(), Some(DSA_COMP_SUCCESS));
        let message = err.to_string();
        assert!(!message.contains("[1"));
        assert!(!message.contains("destination_len"));
        assert!(!message.contains("dst_len"));
    }

    #[test]
    fn malformed_completion_snapshot_surfaces_typed_error() {
        let src = [0x55; 32];
        let mut dst = [0; 32];
        let state = state_for(&src, &mut dst);

        let err = state
            .classify_snapshot(
                &test_config(),
                CompletionSnapshot::new(DSA_COMP_PAGE_FAULT_NOBOF, 0, 64, 0x1000),
            )
            .expect_err("bytes_completed past remaining is malformed");

        assert!(matches!(
            err,
            MemmoveError::MalformedCompletion {
                phase: MemmovePhase::PageFaultRetry,
                bytes_completed: 64,
                ..
            }
        ));
    }
}
