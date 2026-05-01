use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::ptr;

/// Raw mapped IDXD work-queue portal.
///
/// The type owns the MMIO page mapping only. It does not know DSA/IAX
/// descriptors, queue mode policy, retry behavior, or typed submission results;
/// those Rust conveniences belong in `idxd-rust`.
pub struct WqPortal {
    portal: *mut u8,
}

// SAFETY: The portal mapping is an MMIO doorbell page. Sharing the mapping
// handle across threads is sound; callers of the unsafe submission primitives
// remain responsible for descriptor/completion/buffer lifetime and WQ-mode
// correctness.
unsafe impl Send for WqPortal {}
// SAFETY: See the `Send` invariant above. `Sync` only permits sharing the raw
// mapping handle; each unsafe submission still carries the descriptor lifetime
// and queue-mode contract.
unsafe impl Sync for WqPortal {}

impl WqPortal {
    /// Map an IDXD work queue device (for example `/dev/dsa/wq0.0`).
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        let fd = file.as_raw_fd();

        // SAFETY: `fd` is an open work-queue device. The returned mapping is
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

        Ok(Self {
            portal: portal.cast::<u8>(),
        })
    }

    /// Submit a raw 64-byte descriptor to a dedicated WQ via MOVDIR64B.
    ///
    /// # Safety
    /// `desc` must be non-null, valid to read 64 bytes from, and 64-byte
    /// aligned. The descriptor's completion record and all referenced buffers
    /// must remain valid until the operation completes. The caller must only use
    /// this helper with a dedicated work queue that accepts MOVDIR64B.
    #[inline(always)]
    pub unsafe fn submit_movdir64b_desc64(&self, desc: *const u8) {
        // SAFETY: The caller guarantees that `desc` points to a valid,
        // 64-byte-aligned descriptor and that its completion record stays alive
        // until hardware completion. `self.portal` is a live WQ portal mapping.
        unsafe {
            core::arch::asm!(
                "movdir64b ({src}), {dst}",
                dst = in(reg) self.portal,
                src = in(reg) desc,
                options(nostack, preserves_flags, att_syntax),
            );
        }
    }

    /// Submit a raw 64-byte descriptor via ENQCMD. Returns true if accepted.
    ///
    /// # Safety
    /// `desc` must be non-null, valid to read 64 bytes from, and 64-byte
    /// aligned. The descriptor's completion record and all referenced buffers
    /// must remain valid until hardware completion if this returns true. The
    /// caller must only use this helper with a shared work queue that accepts
    /// ENQCMD submission.
    #[inline(always)]
    pub unsafe fn submit_enqcmd_desc64(&self, desc: *const u8) -> bool {
        let mut accepted: u8;
        // SAFETY: The caller guarantees that `desc` points to a valid,
        // 64-byte-aligned descriptor and that its completion record stays alive
        // until hardware completion. ENQCMD reports shared-WQ backpressure via ZF.
        unsafe {
            core::arch::asm!(
                "enqcmd {dst}, [{src}]",
                "setnz {accepted}",
                dst = in(reg) self.portal,
                src = in(reg) desc,
                accepted = out(reg_byte) accepted,
                options(nostack),
            );
        }
        accepted != 0
    }
}

impl Drop for WqPortal {
    fn drop(&mut self) {
        // SAFETY: `self.portal` is the page-sized mapping returned by `mmap` in
        // `WqPortal::open`. The mapping is released exactly once here.
        unsafe {
            libc::munmap(self.portal.cast::<libc::c_void>(), 4096);
        }
    }
}
