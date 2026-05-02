//! DSA-specific descriptor/completion helpers backed by bindgen-generated
//! `linux/idxd.h` definitions from the root-level `idxd-sys` crate.
//!
//! `idxd-sys` now owns only the raw generated UAPI and raw portal primitives.
//! `hw-eval` keeps these tiny benchmark-local wrappers so its methodology code can
//! fill descriptors without depending on higher-level `idxd-rust` APIs.

use std::ptr;

use idxd_sys::idxd;

pub const DSA_COMP_STATUS_MASK: u8 = idxd::DSA_COMP_STATUS_MASK as u8;
pub const DSA_COMP_NONE: u8 = idxd::dsa_completion_status::DSA_COMP_NONE as u8;
pub const DSA_COMP_SUCCESS: u8 = idxd::dsa_completion_status::DSA_COMP_SUCCESS as u8;
pub const DSA_COMP_PAGE_FAULT_NOBOF: u8 =
    idxd::dsa_completion_status::DSA_COMP_PAGE_FAULT_NOBOF as u8;

pub const DSA_OPCODE_NOOP: u8 = idxd::dsa_opcode::DSA_OPCODE_NOOP as u8;
pub const DSA_OPCODE_BATCH: u8 = idxd::dsa_opcode::DSA_OPCODE_BATCH as u8;
pub const DSA_OPCODE_MEMMOVE: u8 = idxd::dsa_opcode::DSA_OPCODE_MEMMOVE as u8;
pub const DSA_OPCODE_MEMFILL: u8 = idxd::dsa_opcode::DSA_OPCODE_MEMFILL as u8;
pub const DSA_OPCODE_COMPARE: u8 = idxd::dsa_opcode::DSA_OPCODE_COMPARE as u8;
pub const DSA_OPCODE_COMPVAL: u8 = idxd::dsa_opcode::DSA_OPCODE_COMPVAL as u8;
pub const DSA_OPCODE_DUALCAST: u8 = idxd::dsa_opcode::DSA_OPCODE_DUALCAST as u8;
pub const DSA_OPCODE_CRCGEN: u8 = idxd::dsa_opcode::DSA_OPCODE_CRCGEN as u8;
pub const DSA_OPCODE_COPY_CRC: u8 = idxd::dsa_opcode::DSA_OPCODE_COPY_CRC as u8;
pub const DSA_OPCODE_CFLUSH: u8 = idxd::dsa_opcode::DSA_OPCODE_CFLUSH as u8;

pub const IDXD_OP_FLAG_CRAV: u32 = idxd::IDXD_OP_FLAG_CRAV;
pub const IDXD_OP_FLAG_RCR: u32 = idxd::IDXD_OP_FLAG_RCR;
pub const IDXD_OP_FLAG_CC: u32 = idxd::IDXD_OP_FLAG_CC;

const COMPLETION_ADDR_OFFSET: usize = 8;
const SRC_ADDR_OFFSET: usize = 16;
const DST_ADDR_OFFSET: usize = 24;
const XFER_SIZE_OFFSET: usize = 32;
const OP_SPECIFIC_OFFSET: usize = 40;

#[inline(always)]
const fn completion_flags() -> u32 {
    IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV | IDXD_OP_FLAG_CC
}

#[inline(always)]
const fn completion_flags_no_cache_control() -> u32 {
    IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV
}

/// 64-byte aligned DSA descriptor wrapper for benchmark-local descriptor fills.
#[repr(C, align(64))]
#[derive(Clone, Copy)]
pub struct DsaHwDesc {
    raw: idxd::dsa_hw_desc,
}

impl Default for DsaHwDesc {
    fn default() -> Self {
        // SAFETY: The generated descriptor is plain C ABI storage. Hardware
        // descriptors are initialized from all-zero storage before fields are set.
        Self {
            raw: unsafe { std::mem::zeroed() },
        }
    }
}

/// 32-byte aligned DSA completion record wrapper.
#[repr(C, align(32))]
#[derive(Clone, Copy)]
pub struct DsaCompletionRecord {
    raw: idxd::dsa_completion_record,
}

impl Default for DsaCompletionRecord {
    fn default() -> Self {
        // SAFETY: The generated completion record is plain C ABI storage and is
        // reset to all-zero status before reuse.
        Self {
            raw: unsafe { std::mem::zeroed() },
        }
    }
}

impl DsaCompletionRecord {
    #[inline(always)]
    pub fn status(&self) -> u8 {
        // SAFETY: Hardware writes this byte; volatile load preserves observation.
        unsafe { ptr::read_volatile(ptr::addr_of!(self.raw.status)) }
    }

    #[inline(always)]
    pub fn fault_addr(&self) -> u64 {
        // SAFETY: Bindgen preserves layout; use an unaligned load because UAPI
        // records may contain packed or union-backed fields.
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.raw.fault_addr)) }
    }
}

impl DsaHwDesc {
    #[inline(always)]
    fn as_desc64_ptr(&self) -> *const u8 {
        ptr::addr_of!(self.raw).cast::<u8>()
    }

    #[inline(always)]
    fn prepare(&mut self, opcode: u8, flags: u32) {
        *self = Self::default();
        self.raw.set_flags(flags & 0x00ff_ffff);
        self.raw.set_opcode(opcode as u32);
    }

    #[inline(always)]
    pub fn set_completion(&mut self, completion: &mut DsaCompletionRecord) {
        self.put_u64(
            COMPLETION_ADDR_OFFSET,
            ptr::addr_of_mut!(completion.raw) as u64,
        );
    }

    #[inline(always)]
    pub fn fill_noop(&mut self) {
        self.prepare(DSA_OPCODE_NOOP, completion_flags());
    }

    #[inline(always)]
    pub fn fill_memmove(&mut self, src: *const u8, dst: *mut u8, size: u32) {
        self.prepare(DSA_OPCODE_MEMMOVE, completion_flags());
        self.set_src_addr(src as u64);
        self.set_dst_addr(dst as u64);
        self.set_xfer_size(size);
    }

    #[inline(always)]
    pub fn fill_crc_gen(&mut self, src: *const u8, size: u32, crc_seed: u32) {
        self.prepare(DSA_OPCODE_CRCGEN, completion_flags_no_cache_control());
        self.set_src_addr(src as u64);
        self.set_xfer_size(size);
        self.set_op_u32(0, crc_seed);
        self.set_op_u64(8, 0);
    }

    #[inline(always)]
    pub fn fill_copy_crc(&mut self, src: *const u8, dst: *mut u8, size: u32, crc_seed: u32) {
        self.prepare(DSA_OPCODE_COPY_CRC, completion_flags_no_cache_control());
        self.set_src_addr(src as u64);
        self.set_dst_addr(dst as u64);
        self.set_xfer_size(size);
        self.set_op_u32(0, crc_seed);
        self.set_op_u64(8, 0);
    }

    #[inline(always)]
    pub fn fill_batch(&mut self, desc_list: *const DsaHwDesc, desc_count: u32) {
        self.prepare(DSA_OPCODE_BATCH, completion_flags());
        self.set_src_addr(desc_list.cast::<u8>() as u64);
        self.set_xfer_size(desc_count);
    }

    #[inline(always)]
    pub fn opcode(&self) -> u8 {
        self.raw.opcode() as u8
    }

    #[inline(always)]
    pub fn flags(&self) -> u32 {
        self.raw.flags()
    }

    #[inline(always)]
    pub fn completion_addr(&self) -> u64 {
        self.get_u64(COMPLETION_ADDR_OFFSET)
    }

    #[inline(always)]
    pub fn src_addr(&self) -> u64 {
        self.get_u64(SRC_ADDR_OFFSET)
    }

    #[inline(always)]
    pub fn dst_addr(&self) -> u64 {
        self.get_u64(DST_ADDR_OFFSET)
    }

    #[inline(always)]
    pub fn xfer_size(&self) -> u32 {
        self.get_u32(XFER_SIZE_OFFSET)
    }

    #[inline(always)]
    fn set_src_addr(&mut self, value: u64) {
        self.put_u64(SRC_ADDR_OFFSET, value);
    }

    #[inline(always)]
    fn set_dst_addr(&mut self, value: u64) {
        self.put_u64(DST_ADDR_OFFSET, value);
    }

    #[inline(always)]
    fn set_xfer_size(&mut self, value: u32) {
        self.put_u32(XFER_SIZE_OFFSET, value);
    }

    #[inline(always)]
    fn set_op_u32(&mut self, offset: usize, value: u32) {
        self.put_u32(OP_SPECIFIC_OFFSET + offset, value);
    }

    #[inline(always)]
    fn set_op_u64(&mut self, offset: usize, value: u64) {
        self.put_u64(OP_SPECIFIC_OFFSET + offset, value);
    }

    #[inline(always)]
    fn byte_ptr(&self, offset: usize) -> *const u8 {
        ptr::addr_of!(self.raw).cast::<u8>().wrapping_add(offset)
    }

    #[inline(always)]
    fn byte_mut_ptr(&mut self, offset: usize) -> *mut u8 {
        ptr::addr_of_mut!(self.raw)
            .cast::<u8>()
            .wrapping_add(offset)
    }

    #[inline(always)]
    fn get_u64(&self, offset: usize) -> u64 {
        // SAFETY: Descriptor fields may be packed; access through unaligned loads.
        unsafe { ptr::read_unaligned(self.byte_ptr(offset).cast::<u64>()) }
    }

    #[inline(always)]
    fn get_u32(&self, offset: usize) -> u32 {
        // SAFETY: Descriptor fields may be packed; access through unaligned loads.
        unsafe { ptr::read_unaligned(self.byte_ptr(offset).cast::<u32>()) }
    }

    #[inline(always)]
    fn put_u32(&mut self, offset: usize, value: u32) {
        // SAFETY: Descriptor fields may be packed; access through unaligned stores.
        unsafe { ptr::write_unaligned(self.byte_mut_ptr(offset).cast::<u32>(), value) }
    }

    #[inline(always)]
    fn put_u64(&mut self, offset: usize, value: u64) {
        // SAFETY: Descriptor fields may be packed; access through unaligned stores.
        unsafe { ptr::write_unaligned(self.byte_mut_ptr(offset).cast::<u64>(), value) }
    }
}

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
        // aligned wrapper around the bindgen-generated DSA descriptor.
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
        // SAFETY: `DsaCompletionRecord` is initialized completion storage; zero is
        // the hardware NONE state used before descriptor submission.
        ptr::write_bytes(comp as *mut DsaCompletionRecord, 0, 1);
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
            ptr::write_volatile(p, ptr::read_volatile(p));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, size_of};

    #[test]
    fn dsa_wrappers_preserve_generated_layout_size_and_hardware_alignment() {
        assert_eq!(size_of::<DsaHwDesc>(), size_of::<idxd::dsa_hw_desc>());
        assert_eq!(align_of::<DsaHwDesc>(), 64);
        assert_eq!(
            size_of::<DsaCompletionRecord>(),
            size_of::<idxd::dsa_completion_record>()
        );
        assert_eq!(align_of::<DsaCompletionRecord>(), 32);
    }

    #[test]
    fn descriptor_helpers_populate_bindgen_backed_storage() {
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
        assert_eq!(desc.completion_addr(), ptr::addr_of_mut!(comp.raw) as u64);
    }

    #[test]
    fn crc_helpers_do_not_request_cache_control() {
        let src = [0x5a_u8; 8];
        let mut dst = [0_u8; 8];
        let mut desc = DsaHwDesc::default();

        desc.fill_crc_gen(src.as_ptr(), src.len() as u32, 0);
        assert_eq!(desc.opcode(), DSA_OPCODE_CRCGEN);
        assert_eq!(desc.flags(), IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV);

        desc.fill_copy_crc(src.as_ptr(), dst.as_mut_ptr(), src.len() as u32, 0);
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
