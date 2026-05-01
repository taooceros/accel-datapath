use std::ptr;

use idxd_sys::idxd_uapi;

const IAX_CRC64_OPCODE: u8 = 0x44;
const IAX_CRC64_POLY_T10DIF: u64 = 0x8bb7_0000_0000_0000;
const IAX_CRC64_FLAGS_OFFSET: usize = 38;
const IAX_CRC64_POLY_OFFSET: usize = 56;
const IAX_CRC64_RESULT_OFFSET: usize = 32;

/// IAX crc64 descriptor wrapper.
///
/// The layout is still the bindgen `iax_hw_desc`; this wrapper only restores the
/// 64-byte alignment required by descriptor submission and provides the crc64
/// field accessors/fill helper.
#[repr(C, align(64))]
#[derive(Clone, Copy)]
pub struct IaxHwDesc {
    raw: idxd_uapi::iax_hw_desc,
}

impl Default for IaxHwDesc {
    fn default() -> Self {
        // SAFETY: The generated descriptor is plain C ABI storage. Hardware
        // descriptors are initialized from all-zero storage before fields are set.
        Self {
            raw: unsafe { std::mem::zeroed() },
        }
    }
}

/// IAX completion record wrapper.
#[repr(C, align(64))]
#[derive(Clone, Copy)]
pub struct IaxCompletionRecord {
    raw: idxd_uapi::iax_completion_record,
}

impl Default for IaxCompletionRecord {
    fn default() -> Self {
        // SAFETY: The generated completion record is plain C ABI storage and is
        // reset to an all-zero status before reuse.
        Self {
            raw: unsafe { std::mem::zeroed() },
        }
    }
}

/// IAX opcode values used by this wrapper.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IaxOpcode {
    Crc64 = IAX_CRC64_OPCODE,
}

impl IaxOpcode {
    #[inline(always)]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

/// IAX completion statuses used by this wrapper.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IaxCompletionStatus {
    None = idxd_uapi::iax_completion_status::IAX_COMP_NONE as u8,
    Success = idxd_uapi::iax_completion_status::IAX_COMP_SUCCESS as u8,
    PageFaultIr = idxd_uapi::iax_completion_status::IAX_COMP_PAGE_FAULT_IR as u8,
    AnalyticsError = 0x0a,
}

impl IaxCompletionStatus {
    #[inline(always)]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    #[inline(always)]
    pub const fn mask(status: u8) -> u8 {
        status & idxd_uapi::DSA_COMP_STATUS_MASK as u8
    }
}

impl IaxCompletionRecord {
    #[inline(always)]
    pub(crate) fn as_raw_mut_ptr(&mut self) -> *mut idxd_uapi::iax_completion_record {
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

    /// Read the analytics error-code byte.
    #[inline(always)]
    pub fn error_code(&self) -> u8 {
        // SAFETY: Hardware writes this byte with completion metadata.
        unsafe { ptr::read_volatile(ptr::addr_of!(self.raw.error_code)) }
    }

    /// Read the invalid-flags diagnostic field.
    #[inline(always)]
    pub fn invalid_flags(&self) -> u32 {
        // SAFETY: Bindgen preserves packed layout; use an unaligned load.
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.raw.invalid_flags)) }
    }

    /// Read the fault address metadata.
    #[inline(always)]
    pub fn fault_addr(&self) -> u64 {
        // SAFETY: Bindgen preserves packed layout; use an unaligned load.
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.raw.fault_addr)) }
    }

    /// Read the crc64 result field.
    #[inline(always)]
    pub fn crc64(&self) -> u64 {
        // SAFETY: The offset is part of the crc64 completion contract and may be
        // unaligned in the generated storage.
        unsafe {
            ptr::read_unaligned(
                ptr::addr_of!(self.raw)
                    .cast::<u8>()
                    .add(IAX_CRC64_RESULT_OFFSET)
                    .cast::<u64>(),
            )
        }
    }
}

impl IaxHwDesc {
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

    /// Read the crc64 source address field.
    #[inline(always)]
    pub fn src1_addr(&self) -> u64 {
        // SAFETY: Bindgen preserves packed layout; use an unaligned load.
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.raw.src1_addr)) }
    }

    /// Read the crc64 source size field.
    #[inline(always)]
    pub fn src1_size(&self) -> u32 {
        // SAFETY: Bindgen preserves packed layout; use an unaligned load.
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.raw.src1_size)) }
    }

    /// Attach the completion record used by this descriptor.
    pub fn set_completion(&mut self, completion: &mut IaxCompletionRecord) {
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

    /// Fill this descriptor for one IAX crc64 operation.
    pub fn fill_crc64(&mut self, src: *const u8, size: u32) {
        *self = Self::default();
        let flags = idxd_uapi::IDXD_OP_FLAG_RCR | idxd_uapi::IDXD_OP_FLAG_CRAV;
        self.raw.set_pasid(0);
        self.raw.set_rsvd(0);
        self.raw.set_priv(0);
        self.raw.set_flags(flags & 0x00ff_ffff);
        self.raw.set_opcode(IaxOpcode::Crc64.as_u8() as u32);

        // SAFETY: The generated descriptor is packed. Use unaligned stores into
        // bindgen fields and documented crc64 raw offsets.
        unsafe {
            ptr::write_unaligned(ptr::addr_of_mut!(self.raw.src1_addr), src as u64);
            ptr::write_unaligned(ptr::addr_of_mut!(self.raw.src1_size), size);
            ptr::write_unaligned(
                ptr::addr_of_mut!(self.raw)
                    .cast::<u8>()
                    .add(IAX_CRC64_FLAGS_OFFSET)
                    .cast::<u16>(),
                0,
            );
            ptr::write_unaligned(
                ptr::addr_of_mut!(self.raw)
                    .cast::<u8>()
                    .add(IAX_CRC64_POLY_OFFSET)
                    .cast::<u64>(),
                IAX_CRC64_POLY_T10DIF,
            );
        }
    }
}
