use crate::descriptor::DsaHwDesc;
use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::ptr;

pub struct WqPortal {
    portal: *mut u8,
    dedicated: bool,
}

/// Non-spinning ENQCMD submission result for shared work queues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnqcmdSubmission {
    Accepted,
    Rejected,
}

// SAFETY: The portal mapping is an MMIO doorbell page. Shared-WQ ENQCMD
// submission is architected for concurrent callers and reports backpressure as
// rejection; dedicated-WQ MOVDIR64B submission remains the caller's
// responsibility to serialize through `submit`/`submit_movdir64b` if a future
// path shares a dedicated portal. The direct async path must use the explicit
// ENQCMD helper below so executor threads do not spin inside `submit`.
unsafe impl Send for WqPortal {}
// SAFETY: See the `Send` invariant above. `Sync` only permits sharing the
// mapping handle; every unsafe submission method still requires the caller to
// keep descriptor and completion memory valid for the accepted operation and to
// choose a submission primitive compatible with the WQ mode.
unsafe impl Sync for WqPortal {}

impl WqPortal {
    /// Open a DSA work queue device (e.g., "/dev/dsa/wq0.0").
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        let fd = file.as_raw_fd();

        // mmap the WQ portal — one page (4096 bytes). The returned mapping is
        // owned by `WqPortal` and released by the `Drop` impl below.
        let portal = unsafe {
            libc::mmap(
                ptr::null_mut(),
                4096,
                libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_POPULATE,
                fd,
                0,
            )
        };

        if portal == libc::MAP_FAILED {
            return Err(std::io::Error::last_os_error());
        }

        let dedicated = detect_wq_mode(path);

        Ok(Self {
            portal: portal as *mut u8,
            dedicated,
        })
    }

    /// Returns true if this is a dedicated WQ (MOVDIR64B), false for shared (ENQCMD).
    pub fn is_dedicated(&self) -> bool {
        self.dedicated
    }

    /// Submit a descriptor to the work queue via MOVDIR64B (dedicated WQ).
    ///
    /// # Safety
    /// Descriptor must be valid and 64-byte aligned. Completion record must
    /// remain valid until the operation completes.
    #[inline(always)]
    pub unsafe fn submit_movdir64b(&self, desc: &DsaHwDesc) {
        // SAFETY: The caller guarantees that `desc` points to a valid,
        // 64-byte-aligned DSA descriptor and that its completion record stays
        // alive until hardware completion. `self.portal` is a live WQ portal
        // mapping owned by this `WqPortal`.
        unsafe {
            core::arch::asm!(
                "movdir64b ({src}), {dst}",
                dst = in(reg) self.portal,
                src = in(reg) desc.as_raw_ptr(),
                options(nostack, preserves_flags, att_syntax),
            );
        }
    }

    /// Submit a descriptor via ENQCMD (shared WQ). Returns true if accepted.
    ///
    /// # Safety
    /// Same requirements as submit_movdir64b.
    #[inline(always)]
    pub unsafe fn submit_enqcmd(&self, desc: &DsaHwDesc) -> bool {
        let mut retry: u8;
        // SAFETY: The caller guarantees that `desc` points to a valid,
        // 64-byte-aligned DSA descriptor and that its completion record stays
        // alive until hardware completion. `self.portal` is a live WQ portal
        // mapping owned by this `WqPortal`. ENQCMD reports shared-WQ
        // backpressure through ZF, which is copied into `retry` once.
        unsafe {
            core::arch::asm!(
                "enqcmd {dst}, [{src}]", // Intel syntax: dst, [src]
                "setnz {result}",        // ZF=0 (success) -> result=1
                dst = in(reg) self.portal,
                src = in(reg) desc.as_raw_ptr(),
                result = out(reg_byte) retry,
                // Removed preserves_flags because we modify ZF.
                options(nostack),
            );
        }
        retry != 0
    }

    /// Submit a descriptor via ENQCMD once and expose hardware backpressure.
    ///
    /// This helper never falls back to MOVDIR64B and never retries or spins;
    /// direct async callers should pair `Rejected` with bounded yielding or
    /// backoff in async context.
    ///
    /// # Safety
    /// Descriptor must be valid and 64-byte aligned. The descriptor and its
    /// completion record must remain valid until hardware completion if this
    /// method returns [`EnqcmdSubmission::Accepted`]. The caller must only use
    /// this helper with a shared work queue that accepts ENQCMD submission.
    #[inline(always)]
    pub unsafe fn submit_enqcmd_once(&self, desc: &DsaHwDesc) -> EnqcmdSubmission {
        // SAFETY: Forwarding this unsafe API's descriptor/completion lifetime
        // requirements to the raw ENQCMD primitive. This wrapper adds only the
        // typed accepted/rejected result and does not call `submit`.
        if unsafe { self.submit_enqcmd(desc) } {
            EnqcmdSubmission::Accepted
        } else {
            EnqcmdSubmission::Rejected
        }
    }

    /// Submit a descriptor using the appropriate method for this WQ type.
    ///
    /// # Safety
    /// Descriptor and completion record must be valid.
    #[inline(always)]
    pub unsafe fn submit(&self, desc: &DsaHwDesc) {
        if self.dedicated {
            // SAFETY: Forwarding this unsafe API's descriptor/completion
            // validity requirements to the dedicated-WQ submission primitive.
            unsafe { self.submit_movdir64b(desc) };
        } else {
            // Retry until accepted (shared WQ may reject under contention).
            loop {
                // SAFETY: Forwarding this unsafe API's descriptor/completion
                // validity requirements to the shared-WQ submission primitive.
                if unsafe { self.submit_enqcmd(desc) } {
                    break;
                }
                core::hint::spin_loop();
            }
        }
    }
}

impl Drop for WqPortal {
    fn drop(&mut self) {
        // SAFETY: `self.portal` is the page-sized mapping returned by `mmap` in
        // `WqPortal::open`. The mapping is released exactly once when the owning
        // portal handle is dropped.
        unsafe {
            libc::munmap(self.portal as *mut libc::c_void, 4096);
        }
    }
}

/// Detect WQ mode from sysfs. Returns true for dedicated, false for shared.
fn detect_wq_mode(dev_path: &Path) -> bool {
    let filename = match dev_path.file_name().and_then(|f| f.to_str()) {
        Some(f) => f,
        None => {
            eprintln!(
                "WARNING: cannot parse device name from {:?}, assuming dedicated WQ",
                dev_path
            );
            return true;
        }
    };
    let sysfs = format!("/sys/bus/dsa/devices/{}/mode", filename);
    match std::fs::read_to_string(&sysfs) {
        Ok(mode) => {
            let mode = mode.trim();
            if mode == "dedicated" {
                true
            } else if mode == "shared" {
                false
            } else {
                eprintln!("WARNING: unknown WQ mode '{}', assuming dedicated", mode);
                true
            }
        }
        Err(_) => {
            eprintln!("WARNING: cannot read {}, assuming dedicated WQ", sysfs);
            true
        }
    }
}
