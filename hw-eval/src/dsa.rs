//! DSA-specific descriptor/completion helpers backed by the canonical
//! `idxd-rust` raw wrapper module.
//!
//! `idxd-sys` owns generated UAPI and raw portal primitives. `idxd-rust::raw`
//! owns the thin Rust descriptor/completion wrappers. `hw-eval` keeps this shim
//! only to preserve its historical benchmark-facing names.

pub use idxd_rust::raw::dsa::{
    default_completion_flags, DsaCompletionRecord, DsaCompletionStatus, DsaFlag, DsaFlags,
    DsaHwDesc, DsaOpcode,
};

pub const DSA_COMP_STATUS_MASK: u8 = 0x7f;
pub const DSA_COMP_NONE: u8 = DsaCompletionStatus::None as u8;
pub const DSA_COMP_SUCCESS: u8 = DsaCompletionStatus::Success as u8;
pub const DSA_COMP_PAGE_FAULT_NOBOF: u8 = DsaCompletionStatus::PageFaultNoBof as u8;

pub const DSA_OPCODE_NOOP: u8 = DsaOpcode::Noop as u8;
pub const DSA_OPCODE_BATCH: u8 = DsaOpcode::Batch as u8;
pub const DSA_OPCODE_MEMMOVE: u8 = DsaOpcode::Memmove as u8;
pub const DSA_OPCODE_MEMFILL: u8 = DsaOpcode::Memfill as u8;
pub const DSA_OPCODE_COMPARE: u8 = DsaOpcode::Compare as u8;
pub const DSA_OPCODE_COMPVAL: u8 = DsaOpcode::CompareValue as u8;
pub const DSA_OPCODE_DUALCAST: u8 = DsaOpcode::Dualcast as u8;
pub const DSA_OPCODE_CRCGEN: u8 = DsaOpcode::CrcGen as u8;
pub const DSA_OPCODE_COPY_CRC: u8 = DsaOpcode::CopyCrc as u8;
pub const DSA_OPCODE_CFLUSH: u8 = DsaOpcode::CacheFlush as u8;

pub const IDXD_OP_FLAG_CRAV: u32 = DsaFlag::CompletionRecordAddressValid as u32;
pub const IDXD_OP_FLAG_RCR: u32 = DsaFlag::RequestCompletionRecord as u32;
pub const IDXD_OP_FLAG_CC: u32 = DsaFlag::CacheControl as u32;

#[inline(always)]
pub fn completion_flags_no_cache_control() -> DsaFlags {
    DsaFlag::RequestCompletionRecord | DsaFlag::CompletionRecordAddressValid
}

impl crate::submit::WqPortal {
    /// Submit a DSA descriptor through the local hw-eval portal.
    ///
    /// # Safety
    /// `desc` must point to a valid 64-byte-aligned DSA descriptor whose
    /// completion record and data buffers remain alive and hardware-accessible
    /// until the operation completes.
    #[inline(always)]
    pub unsafe fn submit(&self, desc: &DsaHwDesc) {
        // SAFETY: The caller of this unsafe shim guarantees descriptor,
        // completion-record, and data-buffer lifetime. The descriptor pointer
        // comes from the canonical `idxd-rust` raw wrapper.
        unsafe { self.submit_desc64(desc.as_desc64_ptr()) };
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
            return DsaCompletionStatus::mask(status);
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
    comp.clear();
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
    fn dsa_types_come_from_idxd_rust_raw_wrappers() {
        assert_eq!(size_of::<DsaHwDesc>(), 64);
        assert_eq!(align_of::<DsaHwDesc>(), 64);
        assert_eq!(size_of::<DsaCompletionRecord>(), 32);
        assert_eq!(align_of::<DsaCompletionRecord>(), 32);
    }

    #[test]
    fn descriptor_helpers_populate_raw_wrapper_storage() {
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
        assert_ne!(desc.completion_addr(), 0);
    }

    #[test]
    fn crc_helpers_do_not_request_cache_control() {
        let src = [0x5a_u8; 8];
        let mut dst = [0_u8; 8];
        let mut desc = DsaHwDesc::default();

        desc.fill_crc_gen(src.as_ptr(), src.len() as u32, 0, 0);
        assert_eq!(desc.opcode(), DSA_OPCODE_CRCGEN);
        assert_eq!(desc.flags(), IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV);

        desc.fill_copy_crc(src.as_ptr(), dst.as_mut_ptr(), src.len() as u32, 0, 0);
        assert_eq!(desc.opcode(), DSA_OPCODE_COPY_CRC);
        assert_eq!(desc.flags(), IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV);
    }

    #[test]
    fn reset_completion_restores_none_status() {
        let mut comp = DsaCompletionRecord::default();
        reset_completion(&mut comp);
        assert_eq!(comp.status(), DSA_COMP_NONE);
    }
}
