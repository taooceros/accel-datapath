use crate::idxd_uapi;
use std::ptr;

/// Bindgen-backed DSA hardware descriptor storage.
///
/// The ABI fields come from `linux/idxd.h` via bindgen (`idxd_uapi::dsa_hw_desc`).
/// This wrapper only restores the 64-byte alignment required by MOVDIR64B
/// descriptor submission; it does not define an independent descriptor layout.
#[repr(C, align(64))]
#[derive(Clone, Copy)]
pub struct BindgenDsaHwDesc {
    raw: idxd_uapi::dsa_hw_desc,
}

/// Public descriptor helper type used by higher-level Rust bindings.
pub type DsaHwDesc = BindgenDsaHwDesc;

impl Default for BindgenDsaHwDesc {
    fn default() -> Self {
        // SAFETY: The generated descriptor is plain C ABI storage. Hardware
        // descriptors are intentionally initialized from an all-zero record
        // before generated packed fields are written by the fill helpers below.
        Self { raw: unsafe { std::mem::zeroed() } }
    }
}

/// Bindgen-backed DSA completion record storage.
///
/// The ABI fields come from `linux/idxd.h` via bindgen
/// (`idxd_uapi::dsa_completion_record`). This wrapper only restores the
/// 32-byte alignment expected by the DSA completion record contract.
#[repr(C, align(32))]
#[derive(Clone, Copy)]
pub struct BindgenDsaCompletionRecord {
    raw: idxd_uapi::dsa_completion_record,
}

/// Public completion helper type used by higher-level Rust bindings.
pub type DsaCompletionRecord = BindgenDsaCompletionRecord;

impl Default for BindgenDsaCompletionRecord {
    fn default() -> Self {
        // SAFETY: The generated completion record is plain C ABI storage and
        // DSA completion records are reset to an all-zero status before reuse.
        Self { raw: unsafe { std::mem::zeroed() } }
    }
}

impl BindgenDsaCompletionRecord {
    #[inline(always)]
    pub(crate) fn as_raw_ptr(&self) -> *const idxd_uapi::dsa_completion_record {
        ptr::addr_of!(self.raw)
    }

    #[inline(always)]
    pub(crate) fn as_raw_mut_ptr(&mut self) -> *mut idxd_uapi::dsa_completion_record {
        ptr::addr_of_mut!(self.raw)
    }

    /// Read the volatile hardware completion status byte.
    #[inline(always)]
    pub fn status(&self) -> u8 {
        // SAFETY: `self.raw` is initialized completion-record storage. The
        // status byte is written by hardware, so callers must observe it with a
        // volatile load rather than an ordinary cached Rust read.
        unsafe { ptr::read_volatile(ptr::addr_of!((*self.as_raw_ptr()).status)) }
    }

    /// Read the result byte from the generated completion union.
    #[inline(always)]
    pub fn result(&self) -> u8 {
        // SAFETY: Bindgen preserves `linux/idxd.h` packed layout with alignment
        // 1, so generated completion fields may be unaligned inside the wrapper.
        unsafe { ptr::read_unaligned(ptr::addr_of!((*self.as_raw_ptr()).__bindgen_anon_1.result)) }
    }

    /// Read bytes completed from the generated completion record.
    #[inline(always)]
    pub fn bytes_completed(&self) -> u32 {
        // SAFETY: Bindgen preserves `linux/idxd.h` packed layout with alignment
        // 1, so multi-byte completion fields must be read with unaligned loads.
        unsafe { ptr::read_unaligned(ptr::addr_of!((*self.as_raw_ptr()).bytes_completed)) }
    }

    /// Read the faulting address from the generated completion record.
    #[inline(always)]
    pub fn fault_addr(&self) -> u64 {
        // SAFETY: Bindgen preserves `linux/idxd.h` packed layout with alignment
        // 1, so multi-byte completion fields must be read with unaligned loads.
        unsafe { ptr::read_unaligned(ptr::addr_of!((*self.as_raw_ptr()).fault_addr)) }
    }

    /// Read CRC value from completion record (for crc_gen / copy_crc ops).
    pub fn crc_value(&self) -> u64 {
        // SAFETY: Bindgen preserves `linux/idxd.h` packed layout with alignment
        // 1, so generated union fields may be unaligned inside the wrapper.
        unsafe { ptr::read_unaligned(ptr::addr_of!((*self.as_raw_ptr()).__bindgen_anon_2.crc_val)) }
    }
}

pub const DSA_OPCODE_NOOP: u8 = idxd_uapi::dsa_opcode::DSA_OPCODE_NOOP as u8;
pub const DSA_OPCODE_BATCH: u8 = idxd_uapi::dsa_opcode::DSA_OPCODE_BATCH as u8;
pub const DSA_OPCODE_MEMMOVE: u8 = idxd_uapi::dsa_opcode::DSA_OPCODE_MEMMOVE as u8;
pub const DSA_OPCODE_MEMFILL: u8 = idxd_uapi::dsa_opcode::DSA_OPCODE_MEMFILL as u8;
pub const DSA_OPCODE_COMPARE: u8 = idxd_uapi::dsa_opcode::DSA_OPCODE_COMPARE as u8;
pub const DSA_OPCODE_COMPVAL: u8 = idxd_uapi::dsa_opcode::DSA_OPCODE_COMPVAL as u8;
pub const DSA_OPCODE_DUALCAST: u8 = idxd_uapi::dsa_opcode::DSA_OPCODE_DUALCAST as u8;
pub const DSA_OPCODE_CRCGEN: u8 = idxd_uapi::dsa_opcode::DSA_OPCODE_CRCGEN as u8;
pub const DSA_OPCODE_COPY_CRC: u8 = idxd_uapi::dsa_opcode::DSA_OPCODE_COPY_CRC as u8;
pub const DSA_OPCODE_CFLUSH: u8 = idxd_uapi::dsa_opcode::DSA_OPCODE_CFLUSH as u8;

pub const IDXD_OP_FLAG_CRAV: u32 = idxd_uapi::IDXD_OP_FLAG_CRAV;
pub const IDXD_OP_FLAG_RCR: u32 = idxd_uapi::IDXD_OP_FLAG_RCR;
pub const IDXD_OP_FLAG_CC: u32 = idxd_uapi::IDXD_OP_FLAG_CC;

pub const DSA_COMP_NONE: u8 = idxd_uapi::dsa_completion_status::DSA_COMP_NONE as u8;
pub const DSA_COMP_SUCCESS: u8 = idxd_uapi::dsa_completion_status::DSA_COMP_SUCCESS as u8;
pub const DSA_COMP_PAGE_FAULT_NOBOF: u8 =
    idxd_uapi::dsa_completion_status::DSA_COMP_PAGE_FAULT_NOBOF as u8;
pub const DSA_COMP_STATUS_MASK: u8 = idxd_uapi::DSA_COMP_STATUS_MASK as u8;

impl BindgenDsaHwDesc {
    #[inline(always)]
    pub(crate) fn as_raw_ptr(&self) -> *const idxd_uapi::dsa_hw_desc {
        ptr::addr_of!(self.raw)
    }

    #[inline(always)]
    fn as_raw_mut_ptr(&mut self) -> *mut idxd_uapi::dsa_hw_desc {
        ptr::addr_of_mut!(self.raw)
    }

    /// Read the generated descriptor opcode bitfield.
    #[inline(always)]
    pub fn opcode(&self) -> u8 {
        self.raw.opcode() as u8
    }

    /// Read the generated descriptor flags bitfield.
    #[inline(always)]
    pub fn flags(&self) -> u32 {
        self.raw.flags()
    }

    /// Read the generated descriptor completion record address field.
    #[inline(always)]
    pub fn completion_addr(&self) -> u64 {
        // SAFETY: Bindgen preserves `linux/idxd.h` packed layout with alignment
        // 1, so descriptor address fields may be unaligned inside the wrapper.
        unsafe { ptr::read_unaligned(ptr::addr_of!((*self.as_raw_ptr()).completion_addr)) }
    }

    /// Read the generated descriptor source address field used by memmove.
    #[inline(always)]
    pub fn src_addr(&self) -> u64 {
        // SAFETY: Bindgen preserves `linux/idxd.h` packed layout with alignment
        // 1, so generated descriptor union fields need unaligned loads.
        unsafe {
            ptr::read_unaligned(ptr::addr_of!(
                (*self.as_raw_ptr()).__bindgen_anon_1.src_addr
            ))
        }
    }

    /// Read the generated descriptor destination address field used by memmove.
    #[inline(always)]
    pub fn dst_addr(&self) -> u64 {
        // SAFETY: Bindgen preserves `linux/idxd.h` packed layout with alignment
        // 1, so generated descriptor union fields need unaligned loads.
        unsafe {
            ptr::read_unaligned(ptr::addr_of!(
                (*self.as_raw_ptr()).__bindgen_anon_2.dst_addr
            ))
        }
    }

    /// Read the generated descriptor transfer size field used by memmove.
    #[inline(always)]
    pub fn xfer_size(&self) -> u32 {
        // SAFETY: Bindgen preserves `linux/idxd.h` packed layout with alignment
        // 1, so generated descriptor union fields need unaligned loads.
        unsafe {
            ptr::read_unaligned(ptr::addr_of!(
                (*self.as_raw_ptr()).__bindgen_anon_3.xfer_size
            ))
        }
    }

    /// Set opcode and standard flags (RCR + CRAV).
    fn set_opcode_flags(&mut self, opcode: u8, extra_flags: u32) {
        let flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV | extra_flags;
        self.raw.set_flags(flags & 0x00FF_FFFF);
        self.raw.set_opcode(opcode as u32);
    }

    /// Set completion record address.
    pub fn set_completion(&mut self, comp: &mut DsaCompletionRecord) {
        // SAFETY: `completion_addr` is a packed generated field. The completion
        // wrapper restores hardware-required alignment, and this stores its raw
        // address without creating a Rust reference to the packed field.
        unsafe {
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).completion_addr),
                comp.as_raw_mut_ptr() as u64,
            );
        }
    }

    /// Fill for memmove (data_move) operation.
    pub fn fill_memmove(&mut self, src: *const u8, dst: *mut u8, size: u32) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_MEMMOVE, IDXD_OP_FLAG_CC);
        // SAFETY: The generated descriptor is packed. Use unaligned stores into
        // the bindgen fields while preserving the caller-provided raw addresses.
        unsafe {
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_1.src_addr),
                src as u64,
            );
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_2.dst_addr),
                dst as u64,
            );
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_3.xfer_size),
                size,
            );
        }
    }

    /// Fill for CRC generation operation.
    pub fn fill_crc_gen(&mut self, src: *const u8, size: u32, seed: u32) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_CRCGEN, 0);
        // SAFETY: The generated descriptor is packed. Use unaligned stores into
        // address/size fields and byte-copy the seed into op-specific storage.
        unsafe {
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_1.src_addr),
                src as u64,
            );
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_3.xfer_size),
                size,
            );
            ptr::copy_nonoverlapping(
                seed.to_le_bytes().as_ptr(),
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_4.op_specific)
                    .cast::<u8>(),
                std::mem::size_of::<u32>(),
            );
        }
    }

    /// Fill for copy + CRC operation (fused copy and CRC-32C).
    pub fn fill_copy_crc(&mut self, src: *const u8, dst: *mut u8, size: u32, seed: u32) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_COPY_CRC, 0);
        // SAFETY: The generated descriptor is packed. Use unaligned stores into
        // address/size fields and byte-copy the seed into op-specific storage.
        unsafe {
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_1.src_addr),
                src as u64,
            );
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_2.dst_addr),
                dst as u64,
            );
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_3.xfer_size),
                size,
            );
            ptr::copy_nonoverlapping(
                seed.to_le_bytes().as_ptr(),
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_4.op_specific)
                    .cast::<u8>(),
                std::mem::size_of::<u32>(),
            );
        }
    }

    /// Fill for memory fill operation.
    pub fn fill_memfill(&mut self, dst: *mut u8, size: u32, pattern: u64) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_MEMFILL, IDXD_OP_FLAG_CC);
        // SAFETY: The generated descriptor is packed. Use unaligned stores into
        // the bindgen fields while preserving the caller-provided raw address.
        unsafe {
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_1.pattern),
                pattern,
            );
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_2.dst_addr),
                dst as u64,
            );
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_3.xfer_size),
                size,
            );
        }
    }

    /// Fill for compare operation.
    pub fn fill_compare(&mut self, src1: *const u8, src2: *const u8, size: u32) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_COMPARE, 0);
        // SAFETY: The generated descriptor is packed. Use unaligned stores into
        // source-address and transfer-size union fields.
        unsafe {
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_1.src_addr),
                src1 as u64,
            );
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_2.src2_addr),
                src2 as u64,
            );
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_3.xfer_size),
                size,
            );
        }
    }

    /// Fill for batch operation.
    pub fn fill_batch(&mut self, desc_list: *const DsaHwDesc, count: u32) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_BATCH, 0);
        // SAFETY: The generated descriptor is packed. Use unaligned stores into
        // descriptor-list and count union fields.
        unsafe {
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_1.desc_list_addr),
                desc_list as u64,
            );
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).__bindgen_anon_3.desc_count),
                count,
            );
        }
    }

    /// Fill for noop operation (useful for measuring submission overhead).
    pub fn fill_noop(&mut self) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_NOOP, 0);
    }
}
