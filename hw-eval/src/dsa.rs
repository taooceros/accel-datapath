//! DSA-specific descriptor/completion helpers backed by bindgen-generated
//! `linux/idxd.h` definitions from the root-level `idxd-sys` crate.
//!
//! This module intentionally does not define descriptor or completion layouts.
//! It preserves the historical `hw_eval::dsa::*` API while delegating DSA ABI
//! ownership, descriptor field writes, and completion accessors to `idxd-sys`.

pub use idxd_sys::{
    DsaCompletionRecord, DsaHwDesc, DSA_COMP_NONE, DSA_COMP_PAGE_FAULT_NOBOF, DSA_COMP_STATUS_MASK,
    DSA_COMP_SUCCESS, DSA_OPCODE_BATCH, DSA_OPCODE_CFLUSH, DSA_OPCODE_COMPARE, DSA_OPCODE_COMPVAL,
    DSA_OPCODE_COPY_CRC, DSA_OPCODE_CRCGEN, DSA_OPCODE_DUALCAST, DSA_OPCODE_MEMFILL,
    DSA_OPCODE_MEMMOVE, DSA_OPCODE_NOOP, IDXD_OP_FLAG_CC, IDXD_OP_FLAG_CRAV, IDXD_OP_FLAG_RCR,
};

impl crate::submit::WqPortal {
    /// Submit a bindgen-backed DSA descriptor through the local hw-eval portal.
    ///
    /// # Safety
    /// `desc` must point to a valid 64-byte-aligned DSA descriptor whose
    /// completion record and data buffers remain alive and hardware-accessible
    /// until the operation completes. Hardware owns the descriptor contents for
    /// the duration of submission, and the mapped WQ portal must match the DSA
    /// descriptor ABI represented by `idxd-sys`.
    #[inline(always)]
    pub unsafe fn submit(&self, desc: &DsaHwDesc) {
        // SAFETY: The caller of this unsafe shim guarantees descriptor
        // alignment/lifetime and completion-record validity. `DsaHwDesc` is the
        // aligned `idxd-sys` wrapper around the bindgen-generated 64-byte DSA
        // descriptor, so its address is the hardware descriptor address passed
        // to the local MOVDIR64B/ENQCMD portal primitive.
        unsafe { self.submit_desc64(desc as *const DsaHwDesc as *const u8) };
    }
}

/// Poll a completion record until hardware writes a non-`NONE` status.
#[inline(always)]
pub fn poll_completion(comp: &DsaCompletionRecord) -> u8 {
    const MAX_SPINS: u64 = 2_000_000_000;
    let mut spins: u64 = 0;

    loop {
        let status = comp.status();
        if status != DSA_COMP_NONE {
            return status & DSA_COMP_STATUS_MASK;
        }

        spins += 1;
        if spins >= MAX_SPINS {
            eprintln!("poll completion: timeout after {} spins", spins);
            return 0xFF;
        }
        core::hint::spin_loop();
    }
}

/// Reset a DSA completion record for reuse.
#[inline(always)]
pub fn reset_completion(comp: &mut DsaCompletionRecord) {
    unsafe {
        // SAFETY: `DsaCompletionRecord` is plain bindgen-backed completion
        // storage wrapped by `idxd-sys`; zero is the hardware NONE state used
        // before descriptor submission.
        std::ptr::write_bytes(comp as *mut DsaCompletionRecord, 0, 1);
    }
}

/// Drain in-flight descriptors by polling every incomplete completion record.
pub fn drain_completions(comps: &[DsaCompletionRecord]) {
    for comp in comps {
        if comp.status() == DSA_COMP_NONE {
            poll_completion(comp);
        }
    }
}

/// Touch the faulting page reported by hardware so callers can retry.
pub fn touch_fault_page(comp: &DsaCompletionRecord) {
    let addr = comp.fault_addr();
    if addr != 0 {
        unsafe {
            let p = addr as *mut u8;
            // SAFETY: This intentionally performs the same best-effort write
            // touch as the historical helper for a hardware-reported fault
            // address. Callers only invoke it after a DSA page-fault completion.
            std::ptr::write_volatile(p, std::ptr::read_volatile(p));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, size_of};

    #[test]
    fn dsa_types_are_reexported_from_idxd_sys() {
        assert_eq!(size_of::<DsaHwDesc>(), size_of::<idxd_sys::DsaHwDesc>());
        assert_eq!(align_of::<DsaHwDesc>(), align_of::<idxd_sys::DsaHwDesc>());
        assert_eq!(
            size_of::<DsaCompletionRecord>(),
            size_of::<idxd_sys::DsaCompletionRecord>()
        );
        assert_eq!(
            align_of::<DsaCompletionRecord>(),
            align_of::<idxd_sys::DsaCompletionRecord>()
        );
    }

    #[test]
    fn descriptor_helpers_delegate_to_idxd_sys_accessors() {
        let src = [0x5a_u8; 8];
        let mut dst = [0_u8; 8];
        let mut desc = DsaHwDesc::default();
        let mut comp = DsaCompletionRecord::default();

        desc.fill_memmove(src.as_ptr(), dst.as_mut_ptr(), src.len() as u32);
        desc.set_completion(&mut comp);

        assert_eq!(desc.opcode(), DSA_OPCODE_MEMMOVE);
        assert_eq!(
            desc.flags(),
            IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV | IDXD_OP_FLAG_CC
        );
        assert_eq!(desc.src_addr(), src.as_ptr() as u64);
        assert_eq!(desc.dst_addr(), dst.as_mut_ptr() as u64);
        assert_eq!(desc.xfer_size(), src.len() as u32);
        assert_eq!(
            desc.completion_addr(),
            (&mut comp as *mut DsaCompletionRecord) as u64
        );
    }

    #[test]
    fn reset_completion_restores_none_status() {
        let mut comp = DsaCompletionRecord::default();
        reset_completion(&mut comp);
        assert_eq!(comp.status(), DSA_COMP_NONE);
    }
}
