use std::ptr;

use idxd_sys::idxd_uapi;

/// DSA memmove descriptor wrapper.
///
/// The layout is still the bindgen `dsa_hw_desc`; this wrapper only restores the
/// 64-byte alignment required by descriptor submission and provides typed field
/// accessors/fill helpers.
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

/// DSA completion record wrapper.
///
/// The layout is still the bindgen `dsa_completion_record`; this wrapper only
/// restores the 32-byte completion-record alignment and provides volatile/packed
/// field accessors.
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

/// DSA opcode values used by this wrapper.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DsaOpcode {
    Memmove = idxd_uapi::dsa_opcode::DSA_OPCODE_MEMMOVE as u8,
}

impl DsaOpcode {
    #[inline(always)]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

/// DSA completion statuses used by this wrapper.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DsaCompletionStatus {
    None = idxd_uapi::dsa_completion_status::DSA_COMP_NONE as u8,
    Success = idxd_uapi::dsa_completion_status::DSA_COMP_SUCCESS as u8,
    PageFaultNoBof = idxd_uapi::dsa_completion_status::DSA_COMP_PAGE_FAULT_NOBOF as u8,
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

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DsaFlag {
    CompletionRecordAddressValid = idxd_uapi::IDXD_OP_FLAG_CRAV,
    RequestCompletionRecord = idxd_uapi::IDXD_OP_FLAG_RCR,
    CacheControl = idxd_uapi::IDXD_OP_FLAG_CC,
}

impl DsaFlag {
    #[inline(always)]
    const fn bits(self) -> u32 {
        self as u32
    }
}

impl DsaCompletionRecord {
    #[inline(always)]
    pub(crate) fn as_raw_mut_ptr(&mut self) -> *mut idxd_uapi::dsa_completion_record {
        ptr::addr_of_mut!(self.raw)
    }

    /// Reset the whole completion record to the no-completion state.
    #[inline(always)]
    pub fn clear(&mut self) {
        // SAFETY: `self` is initialized completion-record storage; zeroing it is
        // the hardware contract for reuse.
        unsafe { ptr::write_bytes(self as *mut Self, 0, 1) }
    }

    /// Read the volatile hardware-owned completion status byte.
    #[inline(always)]
    pub fn status(&self) -> u8 {
        // SAFETY: Hardware writes this byte; volatile load preserves observation.
        unsafe { ptr::read_volatile(ptr::addr_of!(self.raw.status)) }
    }

    /// Read the generated result byte.
    #[inline(always)]
    pub fn result(&self) -> u8 {
        // SAFETY: Bindgen preserves packed layout; use an unaligned load.
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.raw.__bindgen_anon_1.result)) }
    }

    /// Read bytes-completed metadata.
    #[inline(always)]
    pub fn bytes_completed(&self) -> u32 {
        // SAFETY: Bindgen preserves packed layout; use an unaligned load.
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.raw.bytes_completed)) }
    }

    /// Read the fault address metadata.
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

    /// Read the generated opcode bitfield.
    #[inline(always)]
    pub fn opcode(&self) -> u8 {
        self.raw.opcode() as u8
    }

    /// Read the generated flags bitfield.
    #[inline(always)]
    pub fn flags(&self) -> u32 {
        self.raw.flags()
    }

    /// Read the completion-record address field.
    #[inline(always)]
    pub fn completion_addr(&self) -> u64 {
        // SAFETY: Bindgen preserves packed layout; use an unaligned load.
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.raw.completion_addr)) }
    }

    /// Read the memmove source address field.
    #[inline(always)]
    pub fn src_addr(&self) -> u64 {
        // SAFETY: Bindgen preserves packed layout; use an unaligned load.
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.raw.__bindgen_anon_1.src_addr)) }
    }

    /// Read the memmove destination address field.
    #[inline(always)]
    pub fn dst_addr(&self) -> u64 {
        // SAFETY: Bindgen preserves packed layout; use an unaligned load.
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.raw.__bindgen_anon_2.dst_addr)) }
    }

    /// Read the transfer-size field.
    #[inline(always)]
    pub fn xfer_size(&self) -> u32 {
        // SAFETY: Bindgen preserves packed layout; use an unaligned load.
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.raw.__bindgen_anon_3.xfer_size)) }
    }

    /// Attach the completion record used by this descriptor.
    pub fn set_completion(&mut self, completion: &mut DsaCompletionRecord) {
        // SAFETY: The generated field is packed. The completion wrapper restores
        // required alignment; this stores its address without creating references
        // to packed storage.
        unsafe {
            ptr::write_unaligned(
                ptr::addr_of_mut!(self.raw.completion_addr),
                completion.as_raw_mut_ptr() as u64,
            );
        }
    }

    /// Fill this descriptor for one DSA memmove operation.
    pub fn fill_memmove(&mut self, src: *const u8, dst: *mut u8, size: u32) {
        *self = Self::default();
        let flags = DsaFlag::RequestCompletionRecord.bits()
            | DsaFlag::CompletionRecordAddressValid.bits()
            | DsaFlag::CacheControl.bits();
        self.raw.set_flags(flags & 0x00ff_ffff);
        self.raw.set_opcode(DsaOpcode::Memmove.as_u8() as u32);

        // SAFETY: The generated descriptor is packed. Use unaligned stores into
        // bindgen fields while preserving caller-provided addresses.
        unsafe {
            ptr::write_unaligned(
                ptr::addr_of_mut!(self.raw.__bindgen_anon_1.src_addr),
                src as u64,
            );
            ptr::write_unaligned(
                ptr::addr_of_mut!(self.raw.__bindgen_anon_2.dst_addr),
                dst as u64,
            );
            ptr::write_unaligned(ptr::addr_of_mut!(self.raw.__bindgen_anon_3.xfer_size), size);
        }
    }
}
