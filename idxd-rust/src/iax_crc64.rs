use std::path::{Path, PathBuf};

use idxd_sys::{
    IAX_COMP_PAGE_FAULT_IR, IAX_COMP_SUCCESS, IaxCompletionRecord, IaxHwDesc, WqPortal,
    poll_iax_completion, reset_iax_completion, touch_iax_fault_page,
};
use snafu::Snafu;

use crate::lifecycle::{BlockingOperation, BlockingOperationDecision, run_blocking_operation};

/// `idxd-sys` returns `0xFF` when polling an IAX completion times out.
pub const IAX_CRC64_COMPLETION_TIMEOUT_STATUS: u8 = 0xFF;
/// Keep the first representative IAX operation conservative: touch-and-retry once.
pub const DEFAULT_IAX_CRC64_MAX_PAGE_FAULT_RETRIES: u32 = 1;
/// IAX crc64 descriptors encode source size as `u32`.
pub const MAX_IAX_CRC64_BYTES: usize = u32::MAX as usize;

/// Result alias for the representative IAX/IAA crc64 operation.
pub type IaxCrc64Result = Result<IaxCrc64Report, IaxCrc64Error>;

/// Stable success metadata for the representative IAX/IAA crc64 operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IaxCrc64Report {
    /// Work-queue device path used by the session.
    pub device_path: PathBuf,
    /// Source bytes requested by the caller.
    pub requested_bytes: usize,
    /// Number of page-fault retries consumed before success.
    pub page_fault_retries: u32,
    /// Terminal completion status observed by the operation.
    pub final_status: u8,
    /// CRC64 result field reported by the IAX completion record.
    pub crc64: u64,
}

impl IaxCrc64Report {
    fn new<P: AsRef<Path>>(
        device_path: P,
        request: IaxCrc64Request,
        page_fault_retries: u32,
        final_status: u8,
        crc64: u64,
    ) -> Self {
        Self {
            device_path: device_path.as_ref().to_path_buf(),
            requested_bytes: request.len(),
            page_fault_retries,
            final_status,
            crc64,
        }
    }
}

/// Phase associated with an IAX crc64 completion diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IaxCrc64Phase {
    /// Polling the completion record for terminal status.
    CompletionPoll,
    /// Handling a page-fault completion and deciding whether to retry.
    PageFaultRetry,
}

impl std::fmt::Display for IaxCrc64Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CompletionPoll => f.write_str("completion_poll"),
            Self::PageFaultRetry => f.write_str("page_fault_retry"),
        }
    }
}

/// Narrow visible error surface for the representative IAX/IAA crc64 operation.
#[derive(Debug, Snafu)]
pub enum IaxCrc64Error {
    /// The caller requested a length that cannot be encoded in one crc64 descriptor.
    #[snafu(display("invalid IAX crc64 length {requested_len}; expected 1..={max_len}"))]
    InvalidLength {
        /// Caller-requested source length.
        requested_len: usize,
        /// Maximum source length representable by the descriptor field.
        max_len: usize,
    },

    /// The completion poll reached the timeout marker before terminal completion.
    #[snafu(display(
        "IAX crc64 completion polling timed out for {} during {phase}: requested_bytes={requested_bytes}, status=0x{status:02x}, error_code=0x{error_code:02x}, invalid_flags=0x{invalid_flags:08x}, fault_addr=0x{fault_addr:x}, retries={page_fault_retries}",
        device_path.display()
    ))]
    CompletionTimeout {
        /// Work-queue device path used by the session.
        device_path: PathBuf,
        /// Operation phase that produced the timeout marker.
        phase: IaxCrc64Phase,
        /// Caller-requested source length.
        requested_bytes: usize,
        /// Timeout status marker.
        status: u8,
        /// Completion-record error code observed with the timeout marker.
        error_code: u8,
        /// Completion-record invalid-flags field observed with the timeout marker.
        invalid_flags: u32,
        /// Completion-record fault address observed with the timeout marker.
        fault_addr: u64,
        /// Retries consumed before the timeout marker.
        page_fault_retries: u32,
    },

    /// The completion record was internally inconsistent for this narrow operation.
    #[snafu(display(
        "malformed IAX crc64 completion for {} during {phase} after {page_fault_retries} retries: requested_bytes={requested_bytes}, status=0x{status:02x}, error_code=0x{error_code:02x}, invalid_flags=0x{invalid_flags:08x}, fault_addr=0x{fault_addr:x} ({detail})",
        device_path.display()
    ))]
    MalformedCompletion {
        /// Work-queue device path used by the session.
        device_path: PathBuf,
        /// Operation phase that found the malformed completion.
        phase: IaxCrc64Phase,
        /// Caller-requested source length.
        requested_bytes: usize,
        /// Completion status byte.
        status: u8,
        /// Completion-record error code.
        error_code: u8,
        /// Completion-record invalid-flags field.
        invalid_flags: u32,
        /// Completion-record fault address.
        fault_addr: u64,
        /// Retries consumed before the malformed completion.
        page_fault_retries: u32,
        /// Static explanation for the malformed condition.
        detail: &'static str,
    },

    /// A recoverable IAX input page fault repeated beyond the retry budget.
    #[snafu(display(
        "IAX crc64 page-fault retry exhausted for {} during {phase}: requested_bytes={requested_bytes}, status=0x{status:02x}, error_code=0x{error_code:02x}, invalid_flags=0x{invalid_flags:08x}, fault_addr=0x{fault_addr:x}, retries={page_fault_retries}",
        device_path.display()
    ))]
    PageFaultRetryExhausted {
        /// Work-queue device path used by the session.
        device_path: PathBuf,
        /// Operation phase that exhausted retries.
        phase: IaxCrc64Phase,
        /// Caller-requested source length.
        requested_bytes: usize,
        /// Completion status byte.
        status: u8,
        /// Completion-record error code.
        error_code: u8,
        /// Completion-record invalid-flags field.
        invalid_flags: u32,
        /// Completion-record fault address.
        fault_addr: u64,
        /// Retries consumed before failure.
        page_fault_retries: u32,
    },

    /// The completion status was terminal but not a successful crc64 result.
    #[snafu(display(
        "IAX crc64 completion failed for {} during {phase}: requested_bytes={requested_bytes}, status=0x{status:02x}, error_code=0x{error_code:02x}, invalid_flags=0x{invalid_flags:08x}, fault_addr=0x{fault_addr:x}, retries={page_fault_retries}",
        device_path.display()
    ))]
    CompletionStatus {
        /// Work-queue device path used by the session.
        device_path: PathBuf,
        /// Operation phase that observed the terminal status.
        phase: IaxCrc64Phase,
        /// Caller-requested source length.
        requested_bytes: usize,
        /// Completion status byte.
        status: u8,
        /// Completion-record error code.
        error_code: u8,
        /// Completion-record invalid-flags field.
        invalid_flags: u32,
        /// Completion-record fault address.
        fault_addr: u64,
        /// Retries consumed before failure.
        page_fault_retries: u32,
    },
}

impl IaxCrc64Error {
    /// Stable machine-readable error kind for tests and operator diagnostics.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::InvalidLength { .. } => "invalid_length",
            Self::CompletionTimeout { .. } => "completion_timeout",
            Self::MalformedCompletion { .. } => "malformed_completion",
            Self::PageFaultRetryExhausted { .. } => "page_fault_retry_exhausted",
            Self::CompletionStatus { .. } => "completion_status",
        }
    }

    /// Work-queue device path associated with this failure, when hardware was reached.
    pub fn device_path(&self) -> Option<&Path> {
        match self {
            Self::InvalidLength { .. } => None,
            Self::CompletionTimeout { device_path, .. }
            | Self::MalformedCompletion { device_path, .. }
            | Self::PageFaultRetryExhausted { device_path, .. }
            | Self::CompletionStatus { device_path, .. } => Some(device_path.as_path()),
        }
    }

    /// Caller-requested source byte count associated with this failure.
    pub fn requested_bytes(&self) -> usize {
        match self {
            Self::InvalidLength { requested_len, .. } => *requested_len,
            Self::CompletionTimeout {
                requested_bytes, ..
            }
            | Self::MalformedCompletion {
                requested_bytes, ..
            }
            | Self::PageFaultRetryExhausted {
                requested_bytes, ..
            }
            | Self::CompletionStatus {
                requested_bytes, ..
            } => *requested_bytes,
        }
    }

    /// Operation phase associated with this failure, when hardware was reached.
    pub fn phase(&self) -> Option<IaxCrc64Phase> {
        match self {
            Self::InvalidLength { .. } => None,
            Self::CompletionTimeout { phase, .. }
            | Self::MalformedCompletion { phase, .. }
            | Self::PageFaultRetryExhausted { phase, .. }
            | Self::CompletionStatus { phase, .. } => Some(*phase),
        }
    }

    /// Completion status associated with this failure, when one was observed.
    pub fn final_status(&self) -> Option<u8> {
        match self {
            Self::InvalidLength { .. } => None,
            Self::CompletionTimeout { status, .. }
            | Self::MalformedCompletion { status, .. }
            | Self::PageFaultRetryExhausted { status, .. }
            | Self::CompletionStatus { status, .. } => Some(*status),
        }
    }

    /// Completion error-code byte associated with this failure, when one was observed.
    pub fn error_code(&self) -> Option<u8> {
        match self {
            Self::InvalidLength { .. } => None,
            Self::CompletionTimeout { error_code, .. }
            | Self::MalformedCompletion { error_code, .. }
            | Self::PageFaultRetryExhausted { error_code, .. }
            | Self::CompletionStatus { error_code, .. } => Some(*error_code),
        }
    }

    /// Completion invalid-flags field associated with this failure, when one was observed.
    pub fn invalid_flags(&self) -> Option<u32> {
        match self {
            Self::InvalidLength { .. } => None,
            Self::CompletionTimeout { invalid_flags, .. }
            | Self::MalformedCompletion { invalid_flags, .. }
            | Self::PageFaultRetryExhausted { invalid_flags, .. }
            | Self::CompletionStatus { invalid_flags, .. } => Some(*invalid_flags),
        }
    }

    /// Completion fault address associated with this failure, when one was observed.
    pub fn fault_addr(&self) -> Option<u64> {
        match self {
            Self::InvalidLength { .. } => None,
            Self::CompletionTimeout { fault_addr, .. }
            | Self::MalformedCompletion { fault_addr, .. }
            | Self::PageFaultRetryExhausted { fault_addr, .. }
            | Self::CompletionStatus { fault_addr, .. } => Some(*fault_addr),
        }
    }

    /// Page-fault retries consumed before this failure, when hardware was reached.
    pub fn page_fault_retries(&self) -> Option<u32> {
        match self {
            Self::InvalidLength { .. } => None,
            Self::CompletionTimeout {
                page_fault_retries, ..
            }
            | Self::MalformedCompletion {
                page_fault_retries, ..
            }
            | Self::PageFaultRetryExhausted {
                page_fault_retries, ..
            }
            | Self::CompletionStatus {
                page_fault_retries, ..
            } => Some(*page_fault_retries),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IaxCrc64Request {
    len: u32,
}

impl IaxCrc64Request {
    fn new(requested_len: usize) -> Result<Self, IaxCrc64Error> {
        if requested_len == 0 || requested_len > MAX_IAX_CRC64_BYTES {
            return Err(IaxCrc64Error::InvalidLength {
                requested_len,
                max_len: MAX_IAX_CRC64_BYTES,
            });
        }

        Ok(Self {
            len: requested_len as u32,
        })
    }

    fn len(&self) -> usize {
        self.len as usize
    }

    fn len_u32(&self) -> u32 {
        self.len
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IaxCrc64CompletionSnapshot {
    status: u8,
    error_code: u8,
    invalid_flags: u32,
    fault_addr: u64,
    crc64: u64,
}

impl IaxCrc64CompletionSnapshot {
    #[cfg(test)]
    fn new(status: u8, error_code: u8, invalid_flags: u32, fault_addr: u64, crc64: u64) -> Self {
        Self {
            status,
            error_code,
            invalid_flags,
            fault_addr,
            crc64,
        }
    }

    fn from_record(record: &IaxCompletionRecord, polled_status: u8) -> Self {
        Self {
            status: polled_status,
            error_code: record.error_code(),
            invalid_flags: record.invalid_flags(),
            fault_addr: record.fault_addr(),
            crc64: record.crc64(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IaxCrc64Action {
    Success,
    Retry,
}

/// Operation-local IAX crc64 state for the shared blocking lifecycle.
pub(crate) struct IaxCrc64State<'a> {
    desc: IaxHwDesc,
    comp: IaxCompletionRecord,
    device_path: PathBuf,
    src: &'a [u8],
    request: IaxCrc64Request,
    retries: u32,
    max_page_fault_retries: u32,
}

impl<'a> IaxCrc64State<'a> {
    pub(crate) fn new<P: AsRef<Path>>(
        device_path: P,
        src: &'a [u8],
    ) -> Result<Self, IaxCrc64Error> {
        let request = IaxCrc64Request::new(src.len())?;
        Ok(Self {
            desc: IaxHwDesc::default(),
            comp: IaxCompletionRecord::default(),
            device_path: device_path.as_ref().to_path_buf(),
            src,
            request,
            retries: 0,
            max_page_fault_retries: DEFAULT_IAX_CRC64_MAX_PAGE_FAULT_RETRIES,
        })
    }

    #[allow(dead_code)]
    pub(crate) fn descriptor(&self) -> &IaxHwDesc {
        &self.desc
    }

    #[allow(dead_code)]
    pub(crate) fn completion(&self) -> &IaxCompletionRecord {
        &self.comp
    }

    #[allow(dead_code)]
    pub(crate) fn retries(&self) -> u32 {
        self.retries
    }

    fn classify_snapshot(
        &self,
        snapshot: IaxCrc64CompletionSnapshot,
    ) -> Result<IaxCrc64Action, IaxCrc64Error> {
        if snapshot.status == IAX_CRC64_COMPLETION_TIMEOUT_STATUS {
            return Err(self.completion_timeout(snapshot));
        }

        if snapshot.invalid_flags != 0 {
            return Err(self.malformed_completion(
                IaxCrc64Phase::CompletionPoll,
                snapshot,
                "completion carried invalid flags",
            ));
        }

        if snapshot.status == IAX_COMP_SUCCESS {
            if snapshot.error_code != 0 {
                return Err(self.malformed_completion(
                    IaxCrc64Phase::CompletionPoll,
                    snapshot,
                    "successful completion carried an error code",
                ));
            }
            return Ok(IaxCrc64Action::Success);
        }

        if snapshot.status == IAX_COMP_PAGE_FAULT_IR {
            if snapshot.fault_addr == 0 {
                return Err(self.malformed_completion(
                    IaxCrc64Phase::PageFaultRetry,
                    snapshot,
                    "page fault reported without a fault address",
                ));
            }

            if self.retries >= self.max_page_fault_retries {
                return Err(IaxCrc64Error::PageFaultRetryExhausted {
                    device_path: self.device_path.clone(),
                    phase: IaxCrc64Phase::PageFaultRetry,
                    requested_bytes: self.request.len(),
                    status: snapshot.status,
                    error_code: snapshot.error_code,
                    invalid_flags: snapshot.invalid_flags,
                    fault_addr: snapshot.fault_addr,
                    page_fault_retries: self.retries,
                });
            }

            return Ok(IaxCrc64Action::Retry);
        }

        Err(IaxCrc64Error::CompletionStatus {
            device_path: self.device_path.clone(),
            phase: IaxCrc64Phase::CompletionPoll,
            requested_bytes: self.request.len(),
            status: snapshot.status,
            error_code: snapshot.error_code,
            invalid_flags: snapshot.invalid_flags,
            fault_addr: snapshot.fault_addr,
            page_fault_retries: self.retries,
        })
    }

    fn completion_timeout(&self, snapshot: IaxCrc64CompletionSnapshot) -> IaxCrc64Error {
        IaxCrc64Error::CompletionTimeout {
            device_path: self.device_path.clone(),
            phase: IaxCrc64Phase::CompletionPoll,
            requested_bytes: self.request.len(),
            status: snapshot.status,
            error_code: snapshot.error_code,
            invalid_flags: snapshot.invalid_flags,
            fault_addr: snapshot.fault_addr,
            page_fault_retries: self.retries,
        }
    }

    fn malformed_completion(
        &self,
        phase: IaxCrc64Phase,
        snapshot: IaxCrc64CompletionSnapshot,
        detail: &'static str,
    ) -> IaxCrc64Error {
        IaxCrc64Error::MalformedCompletion {
            device_path: self.device_path.clone(),
            phase,
            requested_bytes: self.request.len(),
            status: snapshot.status,
            error_code: snapshot.error_code,
            invalid_flags: snapshot.invalid_flags,
            fault_addr: snapshot.fault_addr,
            page_fault_retries: self.retries,
            detail,
        }
    }

    fn apply_retry(&mut self) {
        self.retries += 1;
    }

    fn success_report(&self, snapshot: IaxCrc64CompletionSnapshot) -> IaxCrc64Report {
        IaxCrc64Report::new(
            &self.device_path,
            self.request,
            self.retries,
            snapshot.status,
            snapshot.crc64,
        )
    }
}

impl BlockingOperation for IaxCrc64State<'_> {
    type Completion = IaxCrc64CompletionSnapshot;
    type Output = IaxCrc64Report;
    type Error = IaxCrc64Error;

    fn reset_and_fill_descriptor(&mut self) {
        reset_iax_completion(&mut self.comp);
        self.desc
            .fill_crc64(self.src.as_ptr(), self.request.len_u32());
        self.desc.set_completion(&mut self.comp);
    }

    unsafe fn submit(&self, portal: &WqPortal) {
        // SAFETY: `IaxCrc64State` owns the descriptor and completion record,
        // and stores a borrowed source slice that remains alive for the full
        // synchronous lifecycle. The descriptor references no destination
        // payload buffer.
        unsafe {
            portal.submit_iax(&self.desc);
        }
    }

    fn observe_completion(&self) -> IaxCrc64CompletionSnapshot {
        let polled_status = poll_iax_completion(&self.comp);
        IaxCrc64CompletionSnapshot::from_record(&self.comp, polled_status)
    }

    fn classify_completion(
        &mut self,
        snapshot: IaxCrc64CompletionSnapshot,
    ) -> Result<BlockingOperationDecision<Self::Output>, Self::Error> {
        match self.classify_snapshot(snapshot)? {
            IaxCrc64Action::Success => Ok(BlockingOperationDecision::Complete(
                self.success_report(snapshot),
            )),
            IaxCrc64Action::Retry => {
                touch_iax_fault_page(&self.comp);
                self.apply_retry();
                Ok(BlockingOperationDecision::Retry)
            }
        }
    }
}

pub(crate) fn run_iax_crc64<P: AsRef<Path>>(
    portal: &WqPortal,
    device_path: P,
    src: &[u8],
) -> IaxCrc64Result {
    let mut operation = IaxCrc64State::new(device_path, src)?;
    run_blocking_operation(portal, &mut operation)
}

#[cfg(test)]
mod tests {
    use idxd_sys::IAX_STATUS_ANALYTICS_ERROR;

    use super::*;

    fn assert_no_payload_markers(text: &str) {
        for forbidden in [
            "[1, 2, 3, 4]",
            "source_bytes",
            "destination_bytes",
            "payload",
            "secret-token",
        ] {
            assert!(
                !text.contains(forbidden),
                "IAX crc64 diagnostic leaked forbidden payload marker {forbidden:?}: {text}"
            );
        }
    }

    fn state_for(src: &[u8]) -> IaxCrc64State<'_> {
        IaxCrc64State::new("/dev/iax/wq1.0", src).expect("valid crc64 state")
    }

    fn snapshot(status: u8) -> IaxCrc64CompletionSnapshot {
        IaxCrc64CompletionSnapshot::new(status, 0, 0, 0, 0x0123_4567_89ab_cdef)
    }

    #[test]
    fn request_validation_rejects_empty_and_oversized_inputs_before_submit() {
        let empty = IaxCrc64Request::new(0).expect_err("empty crc64 input should fail");
        assert_eq!(empty.kind(), "invalid_length");
        assert_eq!(empty.requested_bytes(), 0);
        assert!(empty.device_path().is_none());

        let oversized = IaxCrc64Request::new(MAX_IAX_CRC64_BYTES + 1)
            .expect_err("oversized crc64 input should fail");
        assert_eq!(oversized.kind(), "invalid_length");
        assert_eq!(oversized.requested_bytes(), MAX_IAX_CRC64_BYTES + 1);
        assert_no_payload_markers(&oversized.to_string());
    }

    #[test]
    fn state_fills_crc64_descriptor_and_completion_record_for_source_slice() {
        let src = [0x5a_u8; 33];
        let mut state = state_for(&src);

        state.reset_and_fill_descriptor();

        assert_eq!(state.descriptor().src1_addr(), src.as_ptr() as u64);
        assert_eq!(state.descriptor().src1_size(), src.len() as u32);
        assert_eq!(
            state.descriptor().completion_addr(),
            state.completion() as *const IaxCompletionRecord as u64
        );
    }

    #[test]
    fn success_snapshot_returns_crc64_report() {
        let src = [0x11_u8; 16];
        let state = state_for(&src);
        let success = snapshot(IAX_COMP_SUCCESS);

        assert_eq!(
            state.classify_snapshot(success).expect("success action"),
            IaxCrc64Action::Success
        );
        let report = state.success_report(success);

        assert_eq!(report.device_path, PathBuf::from("/dev/iax/wq1.0"));
        assert_eq!(report.requested_bytes, src.len());
        assert_eq!(report.page_fault_retries, 0);
        assert_eq!(report.final_status, IAX_COMP_SUCCESS);
        assert_eq!(report.crc64, 0x0123_4567_89ab_cdef);
    }

    #[test]
    fn page_fault_retry_decision_and_retry_exhaustion_preserve_fault_metadata() {
        let src = [0x22_u8; 64];
        let mut state = state_for(&src);
        let page_fault =
            IaxCrc64CompletionSnapshot::new(IAX_COMP_PAGE_FAULT_IR, 0x7e, 0, 0xfeed_cafe, 0);

        assert_eq!(
            state
                .classify_snapshot(page_fault)
                .expect("first page fault should be retryable"),
            IaxCrc64Action::Retry
        );
        state.apply_retry();

        let err = state
            .classify_snapshot(page_fault)
            .expect_err("second page fault should exhaust the retry budget");
        assert_eq!(err.kind(), "page_fault_retry_exhausted");
        assert_eq!(err.phase(), Some(IaxCrc64Phase::PageFaultRetry));
        assert_eq!(err.requested_bytes(), src.len());
        assert_eq!(err.final_status(), Some(IAX_COMP_PAGE_FAULT_IR));
        assert_eq!(err.error_code(), Some(0x7e));
        assert_eq!(err.invalid_flags(), Some(0));
        assert_eq!(err.fault_addr(), Some(0xfeed_cafe));
        assert_eq!(err.page_fault_retries(), Some(1));
    }

    #[test]
    fn timeout_and_unexpected_status_preserve_visible_completion_diagnostics() {
        let src = [0x33_u8; 8];
        let state = state_for(&src);
        let timeout = IaxCrc64CompletionSnapshot::new(
            IAX_CRC64_COMPLETION_TIMEOUT_STATUS,
            0x44,
            0x55aa,
            0x1234,
            0,
        );

        let timeout_err = state
            .classify_snapshot(timeout)
            .expect_err("timeout marker should fail");
        assert_eq!(timeout_err.kind(), "completion_timeout");
        assert_eq!(timeout_err.phase(), Some(IaxCrc64Phase::CompletionPoll));
        assert_eq!(timeout_err.requested_bytes(), src.len());
        assert_eq!(
            timeout_err.final_status(),
            Some(IAX_CRC64_COMPLETION_TIMEOUT_STATUS)
        );
        assert_eq!(timeout_err.error_code(), Some(0x44));
        assert_eq!(timeout_err.invalid_flags(), Some(0x55aa));
        assert_eq!(timeout_err.fault_addr(), Some(0x1234));

        let unexpected =
            IaxCrc64CompletionSnapshot::new(IAX_STATUS_ANALYTICS_ERROR, 0x91, 0, 0xabcd, 0);
        let status_err = state
            .classify_snapshot(unexpected)
            .expect_err("unexpected status should fail");
        assert_eq!(status_err.kind(), "completion_status");
        assert_eq!(status_err.final_status(), Some(IAX_STATUS_ANALYTICS_ERROR));
        assert_eq!(status_err.error_code(), Some(0x91));
        assert_eq!(status_err.fault_addr(), Some(0xabcd));
    }

    #[test]
    fn invalid_flags_are_malformed_and_never_coerced_to_success() {
        let src = [0x44_u8; 4];
        let state = state_for(&src);
        let invalid_success = IaxCrc64CompletionSnapshot::new(IAX_COMP_SUCCESS, 0, 0x80, 0, 0);

        let err = state
            .classify_snapshot(invalid_success)
            .expect_err("invalid flags should fail even with success status");
        assert_eq!(err.kind(), "malformed_completion");
        assert_eq!(err.final_status(), Some(IAX_COMP_SUCCESS));
        assert_eq!(err.invalid_flags(), Some(0x80));
    }

    #[test]
    fn display_and_debug_do_not_include_payload_markers() {
        let src = b"secret-token payload source_bytes destination_bytes";
        let state = state_for(src);
        let err = state
            .classify_snapshot(IaxCrc64CompletionSnapshot::new(
                IAX_STATUS_ANALYTICS_ERROR,
                0x42,
                0x99,
                0x1000,
                0,
            ))
            .expect_err("status should fail");

        assert_no_payload_markers(&err.to_string());
        assert_no_payload_markers(&format!("{err:?}"));
    }
}
