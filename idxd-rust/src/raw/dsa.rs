use enumflags2::{BitFlags, bitflags};
use std::ptr;

use idxd_sys::idxd_uapi;

/// 64-byte aligned wrapper over the generated DSA hardware descriptor.
#[repr(C, align(64))]
#[derive(Clone, Copy)]
pub struct DsaHwDesc {
    raw: idxd_uapi::dsa_hw_desc,
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

/// 32-byte aligned wrapper over the generated DSA completion record.
#[repr(C, align(32))]
#[derive(Clone, Copy)]
pub struct DsaCompletionRecord {
    raw: idxd_uapi::dsa_completion_record,
}

impl Default for DsaCompletionRecord {
    fn default() -> Self {
        // SAFETY: The generated completion record is plain C ABI storage and is
        // reset to an all-zero status before reuse.
        Self {
            raw: unsafe { std::mem::zeroed() },
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DsaOpcode {
    Noop = idxd_uapi::dsa_opcode::DSA_OPCODE_NOOP as u8,
    Batch = idxd_uapi::dsa_opcode::DSA_OPCODE_BATCH as u8,
    Drain = idxd_uapi::dsa_opcode::DSA_OPCODE_DRAIN as u8,
    Memmove = idxd_uapi::dsa_opcode::DSA_OPCODE_MEMMOVE as u8,
    Memfill = idxd_uapi::dsa_opcode::DSA_OPCODE_MEMFILL as u8,
    Compare = idxd_uapi::dsa_opcode::DSA_OPCODE_COMPARE as u8,
    CompareValue = idxd_uapi::dsa_opcode::DSA_OPCODE_COMPVAL as u8,
    CreateDelta = idxd_uapi::dsa_opcode::DSA_OPCODE_CR_DELTA as u8,
    ApplyDelta = idxd_uapi::dsa_opcode::DSA_OPCODE_AP_DELTA as u8,
    Dualcast = idxd_uapi::dsa_opcode::DSA_OPCODE_DUALCAST as u8,
    CrcGen = idxd_uapi::dsa_opcode::DSA_OPCODE_CRCGEN as u8,
    CopyCrc = idxd_uapi::dsa_opcode::DSA_OPCODE_COPY_CRC as u8,
    DifCheck = idxd_uapi::dsa_opcode::DSA_OPCODE_DIF_CHECK as u8,
    DifInsert = idxd_uapi::dsa_opcode::DSA_OPCODE_DIF_INS as u8,
    DifStrip = idxd_uapi::dsa_opcode::DSA_OPCODE_DIF_STRP as u8,
    DifUpdate = idxd_uapi::dsa_opcode::DSA_OPCODE_DIF_UPDT as u8,
    CacheFlush = idxd_uapi::dsa_opcode::DSA_OPCODE_CFLUSH as u8,
}

impl DsaOpcode {
    #[inline(always)]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DsaCompletionStatus {
    None = idxd_uapi::dsa_completion_status::DSA_COMP_NONE as u8,
    Success = idxd_uapi::dsa_completion_status::DSA_COMP_SUCCESS as u8,
    SuccessPredicate = idxd_uapi::dsa_completion_status::DSA_COMP_SUCCESS_PRED as u8,
    PageFaultNoBof = idxd_uapi::dsa_completion_status::DSA_COMP_PAGE_FAULT_NOBOF as u8,
    PageFaultIr = idxd_uapi::dsa_completion_status::DSA_COMP_PAGE_FAULT_IR as u8,
    BatchFail = idxd_uapi::dsa_completion_status::DSA_COMP_BATCH_FAIL as u8,
    BatchPageFault = idxd_uapi::dsa_completion_status::DSA_COMP_BATCH_PAGE_FAULT as u8,
    DrOffsetNoInc = idxd_uapi::dsa_completion_status::DSA_COMP_DR_OFFSET_NOINC as u8,
    DrOffsetERange = idxd_uapi::dsa_completion_status::DSA_COMP_DR_OFFSET_ERANGE as u8,
    DifError = idxd_uapi::dsa_completion_status::DSA_COMP_DIF_ERR as u8,
    BadOpcode = idxd_uapi::dsa_completion_status::DSA_COMP_BAD_OPCODE as u8,
    InvalidFlags = idxd_uapi::dsa_completion_status::DSA_COMP_INVALID_FLAGS as u8,
    NoZeroReserve = idxd_uapi::dsa_completion_status::DSA_COMP_NOZERO_RESERVE as u8,
    XferERange = idxd_uapi::dsa_completion_status::DSA_COMP_XFER_ERANGE as u8,
    DescCountERange = idxd_uapi::dsa_completion_status::DSA_COMP_DESC_CNT_ERANGE as u8,
    DrERange = idxd_uapi::dsa_completion_status::DSA_COMP_DR_ERANGE as u8,
    OverlapBuffers = idxd_uapi::dsa_completion_status::DSA_COMP_OVERLAP_BUFFERS as u8,
    DcastError = idxd_uapi::dsa_completion_status::DSA_COMP_DCAST_ERR as u8,
    DescListAlign = idxd_uapi::dsa_completion_status::DSA_COMP_DESCLIST_ALIGN as u8,
    IntHandleInvalid = idxd_uapi::dsa_completion_status::DSA_COMP_INT_HANDLE_INVAL as u8,
    CompletionRecordTranslate = idxd_uapi::dsa_completion_status::DSA_COMP_CRA_XLAT as u8,
    CompletionRecordAlign = idxd_uapi::dsa_completion_status::DSA_COMP_CRA_ALIGN as u8,
    AddressAlign = idxd_uapi::dsa_completion_status::DSA_COMP_ADDR_ALIGN as u8,
    PrivilegeBad = idxd_uapi::dsa_completion_status::DSA_COMP_PRIV_BAD as u8,
    TrafficClassConflict = idxd_uapi::dsa_completion_status::DSA_COMP_TRAFFIC_CLASS_CONF as u8,
    PageFaultReadback = idxd_uapi::dsa_completion_status::DSA_COMP_PFAULT_RDBA as u8,
    HardwareError1 = idxd_uapi::dsa_completion_status::DSA_COMP_HW_ERR1 as u8,
    HardwareErrorDrb = idxd_uapi::dsa_completion_status::DSA_COMP_HW_ERR_DRB as u8,
    TranslationFail = idxd_uapi::dsa_completion_status::DSA_COMP_TRANSLATION_FAIL as u8,
}

impl DsaCompletionStatus {
    #[inline(always)]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    #[inline(always)]
    pub const fn mask(status: u8) -> u8 {
        status & idxd_uapi::DSA_COMP_STATUS_MASK as u8
    }
}

#[bitflags]
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DsaFlag {
    Fence = idxd_uapi::IDXD_OP_FLAG_FENCE,
    BlockOnFault = idxd_uapi::IDXD_OP_FLAG_BOF,
    CompletionRecordAddressValid = idxd_uapi::IDXD_OP_FLAG_CRAV,
    RequestCompletionRecord = idxd_uapi::IDXD_OP_FLAG_RCR,
    RequestCompletionInterrupt = idxd_uapi::IDXD_OP_FLAG_RCI,
    CompletionRecordStatus = idxd_uapi::IDXD_OP_FLAG_CRSTS,
    CompletionRecord = idxd_uapi::IDXD_OP_FLAG_CR,
    CacheControl = idxd_uapi::IDXD_OP_FLAG_CC,
    Address1TrafficClass = idxd_uapi::IDXD_OP_FLAG_ADDR1_TCS,
    Address2TrafficClass = idxd_uapi::IDXD_OP_FLAG_ADDR2_TCS,
    Address3TrafficClass = idxd_uapi::IDXD_OP_FLAG_ADDR3_TCS,
    CompletionRecordTrafficClass = idxd_uapi::IDXD_OP_FLAG_CR_TCS,
    StoreOnly = idxd_uapi::IDXD_OP_FLAG_STORD,
    DrainReadback = idxd_uapi::IDXD_OP_FLAG_DRDBK,
    DrainStatus = idxd_uapi::IDXD_OP_FLAG_DSTS,
}

pub type DsaFlags = BitFlags<DsaFlag>;

#[inline(always)]
pub fn default_completion_flags() -> DsaFlags {
    DsaFlag::RequestCompletionRecord | DsaFlag::CompletionRecordAddressValid | DsaFlag::CacheControl
}

mod apply_delta;
mod batch;
mod cache_flush;
mod compare;
mod compare_value;
mod copy_crc;
mod crc_gen;
mod create_delta;
mod dif_check;
mod dif_insert;
mod dif_strip;
mod dif_update;
mod drain;
mod dualcast;
mod memfill;
mod memmove;
mod noop;

pub use dif_check::DsaDifCheck;
pub use dif_insert::DsaDifInsert;
pub use dif_update::DsaDifUpdate;

impl DsaCompletionRecord {
    #[inline(always)]
    pub(crate) fn as_raw_mut_ptr(&mut self) -> *mut idxd_uapi::dsa_completion_record {
        ptr::addr_of_mut!(self.raw)
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        // SAFETY: `self` is initialized completion-record storage; zeroing it is
        // the hardware contract for reuse.
        unsafe { ptr::write_bytes(self as *mut Self, 0, 1) }
    }

    #[inline(always)]
    pub fn status(&self) -> u8 {
        // SAFETY: Hardware writes this byte; volatile load preserves observation.
        unsafe { ptr::read_volatile(ptr::addr_of!(self.raw.status)) }
    }

    #[inline(always)]
    pub fn result(&self) -> u8 {
        // SAFETY: Bindgen preserves packed layout; use an unaligned load.
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.raw.__bindgen_anon_1.result)) }
    }

    #[inline(always)]
    pub fn bytes_completed(&self) -> u32 {
        // SAFETY: Bindgen preserves packed layout; use an unaligned load.
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.raw.bytes_completed)) }
    }

    #[inline(always)]
    pub fn fault_addr(&self) -> u64 {
        // SAFETY: Bindgen preserves packed layout; use an unaligned load.
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.raw.fault_addr)) }
    }
}

impl DsaHwDesc {
    #[inline(always)]
    pub(crate) fn as_desc64_ptr(&self) -> *const u8 {
        ptr::addr_of!(self.raw).cast::<u8>()
    }

    #[inline(always)]
    pub fn prepare(&mut self, opcode: DsaOpcode, flags: DsaFlags) {
        *self = Self::default();
        self.raw.set_flags(flags.bits() & 0x00ff_ffff);
        self.raw.set_opcode(opcode.as_u8() as u32);
    }

    pub fn set_completion(&mut self, completion: &mut DsaCompletionRecord) {
        self.set_completion_addr(completion.as_raw_mut_ptr() as u64);
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
        self.get_u64(8)
    }

    #[inline(always)]
    pub fn src_addr(&self) -> u64 {
        self.get_u64(16)
    }

    #[inline(always)]
    pub fn dst_addr(&self) -> u64 {
        self.get_u64(24)
    }

    #[inline(always)]
    pub fn xfer_size(&self) -> u32 {
        self.get_u32(32)
    }

    #[inline(always)]
    pub fn desc_count(&self) -> u32 {
        self.get_u32(32)
    }

    #[inline(always)]
    pub fn op_specific(&self) -> [u8; 24] {
        // SAFETY: The op-specific byte array starts at descriptor offset 40.
        unsafe { ptr::read_unaligned(self.byte_ptr(40).cast::<[u8; 24]>()) }
    }

    #[inline(always)]
    fn set_completion_addr(&mut self, value: u64) {
        self.put_u64(8, value);
    }

    #[inline(always)]
    fn set_src_addr(&mut self, value: u64) {
        self.put_u64(16, value);
    }

    #[inline(always)]
    fn set_dst_addr(&mut self, value: u64) {
        self.put_u64(24, value);
    }

    #[inline(always)]
    fn set_xfer_size(&mut self, value: u32) {
        self.put_u32(32, value);
    }

    #[inline(always)]
    fn set_op_u8(&mut self, offset: usize, value: u8) {
        self.put_u8(40 + offset, value);
    }

    #[inline(always)]
    fn set_op_u16(&mut self, offset: usize, value: u16) {
        self.put_u16(40 + offset, value);
    }

    #[inline(always)]
    fn set_op_u32(&mut self, offset: usize, value: u32) {
        self.put_u32(40 + offset, value);
    }

    #[inline(always)]
    fn set_op_u64(&mut self, offset: usize, value: u64) {
        self.put_u64(40 + offset, value);
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
    fn put_u8(&mut self, offset: usize, value: u8) {
        // SAFETY: Descriptor fields may be packed; access through raw stores.
        unsafe { ptr::write_unaligned(self.byte_mut_ptr(offset), value) }
    }

    #[inline(always)]
    fn put_u16(&mut self, offset: usize, value: u16) {
        // SAFETY: Descriptor fields may be packed; access through unaligned stores.
        unsafe { ptr::write_unaligned(self.byte_mut_ptr(offset).cast::<u16>(), value) }
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
