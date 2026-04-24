//! Thin Intel DSA memmove bridge for `tonic-profile`.
//!
//! This crate deliberately stays narrow: it opens one IDXD work queue, submits
//! one memmove descriptor at a time through `idxd-bindings`, retries recoverable
//! page faults, and maps queue-open/completion failures into typed Rust errors.

use std::io;
use std::path::{Path, PathBuf};

use idxd_bindings::{
    poll_completion, reset_completion, touch_fault_page, DsaCompletionRecord, DsaHwDesc, WqPortal,
    DSA_COMP_PAGE_FAULT_NOBOF, DSA_COMP_STATUS_MASK, DSA_COMP_SUCCESS,
};
use thiserror::Error;

/// Repo-wide default DSA work-queue path.
pub const DEFAULT_DEVICE_PATH: &str = "/dev/dsa/wq0.0";
/// `idxd-bindings` returns `0xFF` when polling times out.
pub const COMPLETION_TIMEOUT_STATUS: u8 = 0xFF;
/// Keep the first bridge conservative: touch-and-retry once.
pub const DEFAULT_MAX_PAGE_FAULT_RETRIES: u32 = 1;
/// DSA descriptors encode transfer size as `u32`.
pub const MAX_MEMMOVE_BYTES: usize = u32::MAX as usize;

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

    fn from_record(record: &DsaCompletionRecord, polled_status: u8) -> Self {
        Self {
            status: polled_status,
            result: record.result,
            bytes_completed: record.bytes_completed,
            fault_addr: record.fault_addr,
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

/// Success metadata retained for later report wiring and debugging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemmoveOutcome {
    pub device_path: PathBuf,
    pub requested_bytes: usize,
    pub page_fault_retries: u32,
    pub final_status: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemmovePhase {
    QueueOpen,
    CompletionPoll,
    PageFaultRetry,
}

impl std::fmt::Display for MemmovePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QueueOpen => f.write_str("queue_open"),
            Self::CompletionPoll => f.write_str("completion_poll"),
            Self::PageFaultRetry => f.write_str("page_fault_retry"),
        }
    }
}

/// Typed failure surface for queue-open, completion, and retry faults.
#[derive(Debug, Error)]
pub enum MemmoveError {
    #[error("invalid DSA work-queue path: {device_path}")]
    InvalidDevicePath { device_path: PathBuf },

    #[error("invalid memmove length {requested_len}; expected 1..={max_len}")]
    InvalidLength {
        requested_len: usize,
        max_len: usize,
    },

    #[error("destination buffer too small: src_len={src_len}, dst_len={dst_len}")]
    DestinationTooSmall { src_len: usize, dst_len: usize },

    #[error("failed to open DSA work queue {device_path}: {source}")]
    QueueOpen {
        device_path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("completion polling timed out for {device_path} during {phase}")]
    CompletionTimeout {
        device_path: PathBuf,
        phase: MemmovePhase,
    },

    #[error(
        "malformed completion for {device_path} during {phase}: status=0x{status:02x}, result={result}, bytes_completed={bytes_completed}, fault_addr=0x{fault_addr:x} ({detail})"
    )]
    MalformedCompletion {
        device_path: PathBuf,
        phase: MemmovePhase,
        status: u8,
        result: u8,
        bytes_completed: u32,
        fault_addr: u64,
        detail: &'static str,
    },

    #[error(
        "page-fault retry exhausted for {device_path}: retries={retries}, bytes_completed={bytes_completed}, fault_addr=0x{fault_addr:x}"
    )]
    PageFaultRetryExhausted {
        device_path: PathBuf,
        retries: u32,
        bytes_completed: u32,
        fault_addr: u64,
    },

    #[error(
        "memmove completion failed for {device_path}: status=0x{status:02x}, bytes_completed={bytes_completed}, fault_addr=0x{fault_addr:x}"
    )]
    CompletionStatus {
        device_path: PathBuf,
        status: u8,
        bytes_completed: u32,
        fault_addr: u64,
    },
}

/// Thin reusable session over one mapped DSA work queue.
pub struct DsaSession {
    device_path: PathBuf,
    portal: WqPortal,
    max_page_fault_retries: u32,
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
        let device_path = normalize_device_path(device_path.as_ref())?;
        let portal = WqPortal::open(&device_path).map_err(|source| MemmoveError::QueueOpen {
            device_path: device_path.clone(),
            source,
        })?;

        Ok(Self {
            device_path,
            portal,
            max_page_fault_retries,
        })
    }

    pub fn device_path(&self) -> &Path {
        &self.device_path
    }

    pub fn max_page_fault_retries(&self) -> u32 {
        self.max_page_fault_retries
    }

    /// Submit one memmove over the mapped work queue.
    pub fn memmove(&self, dst: &mut [u8], src: &[u8]) -> Result<MemmoveOutcome, MemmoveError> {
        let request = MemmoveRequest::new(src.len())?;
        if dst.len() < src.len() {
            return Err(MemmoveError::DestinationTooSmall {
                src_len: src.len(),
                dst_len: dst.len(),
            });
        }

        self.memmove_inner(dst.as_mut_ptr(), src.as_ptr(), request)
    }

    fn memmove_inner(
        &self,
        dst: *mut u8,
        src: *const u8,
        request: MemmoveRequest,
    ) -> Result<MemmoveOutcome, MemmoveError> {
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
                self.device_path.clone(),
                state.remaining(),
                snapshot,
                retries,
                self.max_page_fault_retries,
            )? {
                CompletionAction::Success => {
                    return Ok(MemmoveOutcome {
                        device_path: self.device_path.clone(),
                        requested_bytes: request.len(),
                        page_fault_retries: retries,
                        final_status: snapshot.status,
                    });
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

/// Interpret a memmove completion record in a hardware-free test or after live
/// polling.
pub fn classify_memmove_completion(
    device_path: PathBuf,
    remaining_len: usize,
    snapshot: CompletionSnapshot,
    page_fault_retries: u32,
    max_page_fault_retries: u32,
) -> Result<CompletionAction, MemmoveError> {
    if snapshot.status == COMPLETION_TIMEOUT_STATUS {
        return Err(MemmoveError::CompletionTimeout {
            device_path,
            phase: MemmovePhase::CompletionPoll,
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
                detail: "bytes_completed exceeds the remaining transfer size",
            });
        }

        if page_fault_retries >= max_page_fault_retries {
            return Err(MemmoveError::PageFaultRetryExhausted {
                device_path,
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
        status: snapshot.status & DSA_COMP_STATUS_MASK,
        bytes_completed: snapshot.bytes_completed,
        fault_addr: snapshot.fault_addr,
    })
}

fn normalize_device_path(device_path: &Path) -> Result<PathBuf, MemmoveError> {
    if device_path.as_os_str().is_empty() {
        return Err(MemmoveError::InvalidDevicePath {
            device_path: device_path.to_path_buf(),
        });
    }

    Ok(device_path.to_path_buf())
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
            remaining: request.len,
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
