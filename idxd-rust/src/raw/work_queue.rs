use std::path::Path;

use crate::raw::dsa_memmove::DsaHwDesc;
use crate::raw::iax_crc64::IaxHwDesc;

/// Rust-owned wrapper around the raw `idxd-sys` MMIO work-queue portal.
pub struct WqPortal {
    raw: idxd_sys::WqPortal,
}

impl WqPortal {
    /// Map an IDXD work-queue device through the raw sys crate.
    pub fn open(path: &Path) -> std::io::Result<Self> {
        idxd_sys::WqPortal::open(path).map(|raw| Self { raw })
    }

    /// Submit one DSA memmove descriptor using the detected WQ mode.
    ///
    /// # Safety
    /// The descriptor, completion record, and referenced buffers must remain
    /// valid until hardware completion.
    #[inline(always)]
    pub(crate) unsafe fn submit_dsa_memmove(&self, desc: &DsaHwDesc, dedicated: bool) {
        // SAFETY: The caller provides the descriptor lifetime contract; this
        // method only selects the raw doorbell primitive.
        unsafe { self.submit_desc64(desc.as_desc64_ptr(), dedicated) }
    }

    /// Submit one IAX crc64 descriptor using the detected WQ mode.
    ///
    /// # Safety
    /// The descriptor, completion record, and referenced buffers must remain
    /// valid until hardware completion.
    #[inline(always)]
    pub(crate) unsafe fn submit_iax_crc64(&self, desc: &IaxHwDesc, dedicated: bool) {
        // SAFETY: The caller provides the descriptor lifetime contract; this
        // method only selects the raw doorbell primitive.
        unsafe { self.submit_desc64(desc.as_desc64_ptr(), dedicated) }
    }

    #[inline(always)]
    unsafe fn submit_desc64(&self, desc: *const u8, dedicated: bool) {
        if dedicated {
            // SAFETY: Forwarding this unsafe API's descriptor/completion lifetime
            // contract to the raw dedicated-WQ primitive.
            unsafe { self.raw.submit_movdir64b_desc64(desc) };
        } else {
            loop {
                // SAFETY: Forwarding this unsafe API's descriptor/completion
                // lifetime contract to the raw shared-WQ primitive.
                if unsafe { self.raw.submit_enqcmd_desc64(desc) } {
                    break;
                }
                core::hint::spin_loop();
            }
        }
    }
}

/// Detect WQ mode from sysfs. Returns true for dedicated, false for shared.
pub fn detect_wq_mode(dev_path: &Path) -> bool {
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
        Ok(mode) => match mode.trim() {
            "dedicated" => true,
            "shared" => false,
            other => {
                eprintln!("WARNING: unknown WQ mode '{}', assuming dedicated", other);
                true
            }
        },
        Err(_) => {
            eprintln!("WARNING: cannot read {}, assuming dedicated WQ", sysfs);
            true
        }
    }
}
