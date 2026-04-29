use std::io;
use std::path::{Path, PathBuf};

use idxd_sys::{
    DSA_COMP_PAGE_FAULT_NOBOF, DSA_COMP_STATUS_MASK, DSA_COMP_SUCCESS, DsaCompletionRecord,
};
use snafu::Snafu;

/// Repo-wide default DSA work-queue path.
pub const DEFAULT_DEVICE_PATH: &str = "/dev/dsa/wq0.0";
/// `idxd-sys` returns `0xFF` when polling times out.
pub const COMPLETION_TIMEOUT_STATUS: u8 = 0xFF;
/// Keep the first bridge conservative: touch-and-retry once.
pub const DEFAULT_MAX_PAGE_FAULT_RETRIES: u32 = 1;
/// DSA descriptors encode transfer size as `u32`.
pub const MAX_MEMMOVE_BYTES: usize = u32::MAX as usize;

/// Stable configuration for one reusable DSA memmove session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemmoveValidationConfig {
    device_path: PathBuf,
    max_page_fault_retries: u32,
}

impl Default for MemmoveValidationConfig {
    fn default() -> Self {
        Self {
            device_path: PathBuf::from(DEFAULT_DEVICE_PATH),
            max_page_fault_retries: DEFAULT_MAX_PAGE_FAULT_RETRIES,
        }
    }
}

#[bon::bon]
impl MemmoveValidationConfig {
    #[builder(finish_fn = build)]
    pub fn builder(
        #[builder(default = PathBuf::from(DEFAULT_DEVICE_PATH), into)] device_path: PathBuf,
        #[builder(default = DEFAULT_MAX_PAGE_FAULT_RETRIES)] max_page_fault_retries: u32,
    ) -> Result<Self, MemmoveError> {
        Self::with_retries(device_path, max_page_fault_retries)
    }

    pub fn new<P: AsRef<Path>>(device_path: P) -> Result<Self, MemmoveError> {
        Self::with_retries(device_path, DEFAULT_MAX_PAGE_FAULT_RETRIES)
    }

    pub fn with_retries<P: AsRef<Path>>(
        device_path: P,
        max_page_fault_retries: u32,
    ) -> Result<Self, MemmoveError> {
        Ok(Self {
            device_path: normalize_device_path(device_path.as_ref())?,
            max_page_fault_retries,
        })
    }

    pub fn device_path(&self) -> &Path {
        &self.device_path
    }

    pub fn max_page_fault_retries(&self) -> u32 {
        self.max_page_fault_retries
    }
}

/// Request validation that can be exercised without hardware.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemmoveRequest {
    len: u32,
}

impl MemmoveRequest {
    /// Validate a requested memmove length against the DSA descriptor contract.
    pub fn new(requested_len: usize) -> Result<Self, MemmoveError> {
        if requested_len == 0 || requested_len > MAX_MEMMOVE_BYTES {
            return Err(MemmoveError::InvalidLength {
                requested_len,
                max_len: MAX_MEMMOVE_BYTES,
            });
        }

        Ok(Self {
            len: requested_len as u32,
        })
    }

    /// Validate caller-owned source and destination buffers before queue-open.
    pub fn for_buffers(dst_len: usize, src_len: usize) -> Result<Self, MemmoveError> {
        let request = Self::new(src_len)?;
        if dst_len < src_len {
            return Err(MemmoveError::DestinationTooSmall { src_len, dst_len });
        }

        Ok(request)
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// Observable completion state used by hardware-free contract tests and runtime
/// error mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletionSnapshot {
    pub status: u8,
    pub result: u8,
    pub bytes_completed: u32,
    pub fault_addr: u64,
}

impl CompletionSnapshot {
    pub fn new(status: u8, result: u8, bytes_completed: u32, fault_addr: u64) -> Self {
        Self {
            status,
            result,
            bytes_completed,
            fault_addr,
        }
    }

    pub(crate) fn from_record(record: &DsaCompletionRecord, polled_status: u8) -> Self {
        Self {
            status: polled_status,
            result: record.result(),
            bytes_completed: record.bytes_completed(),
            fault_addr: record.fault_addr(),
        }
    }
}

/// How the memmove state machine should proceed after reading a completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionAction {
    Success,
    Retry(MemmoveRetry),
}

/// Offset adjustment for a continuation descriptor after a recoverable page
/// fault.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemmoveRetry {
    pub next_src_offset: usize,
    pub next_dst_offset: usize,
    pub remaining_bytes: usize,
}

/// Stable success metadata retained for later CLI serialization and debugging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemmoveValidationReport {
    pub device_path: PathBuf,
    pub requested_bytes: usize,
    pub page_fault_retries: u32,
    pub final_status: u8,
}

impl MemmoveValidationReport {
    pub fn new<P: AsRef<Path>>(
        device_path: P,
        request: MemmoveRequest,
        page_fault_retries: u32,
        final_status: u8,
    ) -> Result<Self, MemmoveError> {
        Ok(Self {
            device_path: normalize_device_path(device_path.as_ref())?,
            requested_bytes: request.len(),
            page_fault_retries,
            final_status,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemmovePhase {
    QueueOpen,
    CompletionPoll,
    PageFaultRetry,
    PostCopyVerify,
}

impl std::fmt::Display for MemmovePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QueueOpen => f.write_str("queue_open"),
            Self::CompletionPoll => f.write_str("completion_poll"),
            Self::PageFaultRetry => f.write_str("page_fault_retry"),
            Self::PostCopyVerify => f.write_str("post_copy_verify"),
        }
    }
}

/// Typed failure surface for queue-open, completion, retry, and post-copy
/// validation faults.
#[derive(Debug, Snafu)]
pub enum MemmoveError {
    #[snafu(display("invalid DSA work-queue path: {}", device_path.display()))]
    InvalidDevicePath { device_path: PathBuf },

    #[snafu(display("invalid memmove length {requested_len}; expected 1..={max_len}"))]
    InvalidLength {
        requested_len: usize,
        max_len: usize,
    },

    #[snafu(display("destination buffer too small: src_len={src_len}, dst_len={dst_len}"))]
    DestinationTooSmall { src_len: usize, dst_len: usize },

    #[snafu(display(
        "failed to open DSA work queue {} during {phase}: {source}",
        device_path.display()
    ))]
    QueueOpen {
        device_path: PathBuf,
        phase: MemmovePhase,
        source: io::Error,
    },

    #[snafu(display(
        "completion polling timed out for {} during {phase} after {page_fault_retries} retries",
        device_path.display()
    ))]
    CompletionTimeout {
        device_path: PathBuf,
        phase: MemmovePhase,
        page_fault_retries: u32,
    },

    #[snafu(display(
        "malformed completion for {} during {phase} after {page_fault_retries} retries: status=0x{status:02x}, result={result}, bytes_completed={bytes_completed}, fault_addr=0x{fault_addr:x} ({detail})",
        device_path.display()
    ))]
    MalformedCompletion {
        device_path: PathBuf,
        phase: MemmovePhase,
        status: u8,
        result: u8,
        bytes_completed: u32,
        fault_addr: u64,
        page_fault_retries: u32,
        detail: &'static str,
    },

    #[snafu(display(
        "page-fault retry exhausted for {} during {phase}: retries={retries}, bytes_completed={bytes_completed}, fault_addr=0x{fault_addr:x}",
        device_path.display()
    ))]
    PageFaultRetryExhausted {
        device_path: PathBuf,
        phase: MemmovePhase,
        retries: u32,
        bytes_completed: u32,
        fault_addr: u64,
    },

    #[snafu(display(
        "memmove completion failed for {} during {phase} after {page_fault_retries} retries: status=0x{status:02x}, bytes_completed={bytes_completed}, fault_addr=0x{fault_addr:x}",
        device_path.display()
    ))]
    CompletionStatus {
        device_path: PathBuf,
        phase: MemmovePhase,
        status: u8,
        bytes_completed: u32,
        fault_addr: u64,
        page_fault_retries: u32,
    },

    #[snafu(display(
        "post-copy verification failed for {} during {phase}: mismatch_offset={mismatch_offset}, requested_bytes={requested_bytes}, final_status=0x{final_status:02x}, retries={page_fault_retries}",
        device_path.display()
    ))]
    ByteMismatch {
        device_path: PathBuf,
        phase: MemmovePhase,
        requested_bytes: usize,
        mismatch_offset: usize,
        final_status: u8,
        page_fault_retries: u32,
    },
}

impl MemmoveError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::InvalidDevicePath { .. } => "invalid_device_path",
            Self::InvalidLength { .. } => "invalid_length",
            Self::DestinationTooSmall { .. } => "destination_too_small",
            Self::QueueOpen { .. } => "queue_open",
            Self::CompletionTimeout { .. } => "completion_timeout",
            Self::MalformedCompletion { .. } => "malformed_completion",
            Self::PageFaultRetryExhausted { .. } => "page_fault_retry_exhausted",
            Self::CompletionStatus { .. } => "completion_status",
            Self::ByteMismatch { .. } => "byte_mismatch",
        }
    }

    pub fn device_path(&self) -> Option<&Path> {
        match self {
            Self::InvalidDevicePath { device_path }
            | Self::QueueOpen { device_path, .. }
            | Self::CompletionTimeout { device_path, .. }
            | Self::MalformedCompletion { device_path, .. }
            | Self::PageFaultRetryExhausted { device_path, .. }
            | Self::CompletionStatus { device_path, .. }
            | Self::ByteMismatch { device_path, .. } => Some(device_path.as_path()),
            Self::InvalidLength { .. } | Self::DestinationTooSmall { .. } => None,
        }
    }

    pub fn phase(&self) -> Option<MemmovePhase> {
        match self {
            Self::QueueOpen { phase, .. }
            | Self::CompletionTimeout { phase, .. }
            | Self::MalformedCompletion { phase, .. }
            | Self::PageFaultRetryExhausted { phase, .. }
            | Self::CompletionStatus { phase, .. }
            | Self::ByteMismatch { phase, .. } => Some(*phase),
            Self::InvalidDevicePath { .. }
            | Self::InvalidLength { .. }
            | Self::DestinationTooSmall { .. } => None,
        }
    }

    pub fn page_fault_retries(&self) -> Option<u32> {
        match self {
            Self::CompletionTimeout {
                page_fault_retries, ..
            }
            | Self::MalformedCompletion {
                page_fault_retries, ..
            }
            | Self::CompletionStatus {
                page_fault_retries, ..
            }
            | Self::ByteMismatch {
                page_fault_retries, ..
            } => Some(*page_fault_retries),
            Self::PageFaultRetryExhausted { retries, .. } => Some(*retries),
            Self::InvalidDevicePath { .. }
            | Self::InvalidLength { .. }
            | Self::DestinationTooSmall { .. }
            | Self::QueueOpen { .. } => None,
        }
    }

    pub fn final_status(&self) -> Option<u8> {
        match self {
            Self::MalformedCompletion { status, .. }
            | Self::CompletionStatus { status, .. }
            | Self::ByteMismatch {
                final_status: status,
                ..
            } => Some(*status),
            Self::InvalidDevicePath { .. }
            | Self::InvalidLength { .. }
            | Self::DestinationTooSmall { .. }
            | Self::QueueOpen { .. }
            | Self::CompletionTimeout { .. }
            | Self::PageFaultRetryExhausted { .. } => None,
        }
    }

    pub fn requested_bytes(&self) -> Option<usize> {
        match self {
            Self::InvalidLength { requested_len, .. } => Some(*requested_len),
            Self::DestinationTooSmall { src_len, .. } => Some(*src_len),
            Self::ByteMismatch {
                requested_bytes, ..
            } => Some(*requested_bytes),
            Self::InvalidDevicePath { .. }
            | Self::QueueOpen { .. }
            | Self::CompletionTimeout { .. }
            | Self::MalformedCompletion { .. }
            | Self::PageFaultRetryExhausted { .. }
            | Self::CompletionStatus { .. } => None,
        }
    }
}

/// Interpret a memmove completion record in a hardware-free test or after live
/// polling.
pub fn classify_memmove_completion(
    config: &MemmoveValidationConfig,
    remaining_len: usize,
    snapshot: CompletionSnapshot,
    page_fault_retries: u32,
) -> Result<CompletionAction, MemmoveError> {
    let device_path = config.device_path().to_path_buf();

    if snapshot.status == COMPLETION_TIMEOUT_STATUS {
        return Err(MemmoveError::CompletionTimeout {
            device_path,
            phase: MemmovePhase::CompletionPoll,
            page_fault_retries,
        });
    }

    if snapshot.status == DSA_COMP_SUCCESS {
        return Ok(CompletionAction::Success);
    }

    if snapshot.status == DSA_COMP_PAGE_FAULT_NOBOF {
        if snapshot.fault_addr == 0 {
            return Err(MemmoveError::MalformedCompletion {
                device_path,
                phase: MemmovePhase::PageFaultRetry,
                status: snapshot.status,
                result: snapshot.result,
                bytes_completed: snapshot.bytes_completed,
                fault_addr: snapshot.fault_addr,
                page_fault_retries,
                detail: "page fault reported without a fault address",
            });
        }

        if snapshot.result > 1 {
            return Err(MemmoveError::MalformedCompletion {
                device_path,
                phase: MemmovePhase::PageFaultRetry,
                status: snapshot.status,
                result: snapshot.result,
                bytes_completed: snapshot.bytes_completed,
                fault_addr: snapshot.fault_addr,
                page_fault_retries,
                detail: "memmove page-fault direction must be 0 or 1",
            });
        }

        if snapshot.bytes_completed as usize > remaining_len {
            return Err(MemmoveError::MalformedCompletion {
                device_path,
                phase: MemmovePhase::PageFaultRetry,
                status: snapshot.status,
                result: snapshot.result,
                bytes_completed: snapshot.bytes_completed,
                fault_addr: snapshot.fault_addr,
                page_fault_retries,
                detail: "bytes_completed exceeds the remaining transfer size",
            });
        }

        if page_fault_retries >= config.max_page_fault_retries() {
            return Err(MemmoveError::PageFaultRetryExhausted {
                device_path,
                phase: MemmovePhase::PageFaultRetry,
                retries: page_fault_retries,
                bytes_completed: snapshot.bytes_completed,
                fault_addr: snapshot.fault_addr,
            });
        }

        let advanced = snapshot.bytes_completed as usize;
        let remaining_bytes = remaining_len - advanced;
        let (next_src_offset, next_dst_offset) = if snapshot.result == 0 {
            (advanced, advanced)
        } else {
            (0, 0)
        };

        return Ok(CompletionAction::Retry(MemmoveRetry {
            next_src_offset,
            next_dst_offset,
            remaining_bytes,
        }));
    }

    Err(MemmoveError::CompletionStatus {
        device_path,
        phase: MemmovePhase::CompletionPoll,
        status: snapshot.status & DSA_COMP_STATUS_MASK,
        bytes_completed: snapshot.bytes_completed,
        fault_addr: snapshot.fault_addr,
        page_fault_retries,
    })
}

pub(crate) fn normalize_device_path(device_path: &Path) -> Result<PathBuf, MemmoveError> {
    if device_path.as_os_str().is_empty() {
        return Err(MemmoveError::InvalidDevicePath {
            device_path: device_path.to_path_buf(),
        });
    }

    Ok(device_path.to_path_buf())
}
