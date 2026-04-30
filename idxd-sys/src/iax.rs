use crate::idxd_uapi;
use std::ptr;

/// Bindgen-backed IAX/IAA hardware descriptor storage.
///
/// The ABI fields come from `linux/idxd.h` via bindgen
/// (`idxd_uapi::iax_hw_desc`). This wrapper only restores the 64-byte
/// alignment required by raw 64-byte descriptor submission; it does not define
/// an independent descriptor layout.
#[repr(C, align(64))]
#[derive(Clone, Copy)]
pub struct BindgenIaxHwDesc {
    raw: idxd_uapi::iax_hw_desc,
}

/// Public IAX/IAA descriptor helper type used by higher-level Rust bindings.
pub type IaxHwDesc = BindgenIaxHwDesc;

impl Default for BindgenIaxHwDesc {
    fn default() -> Self {
        // SAFETY: The generated IAX descriptor is plain C ABI storage.
        // Hardware descriptors are intentionally initialized from an all-zero
        // record before generated packed fields are written by fill helpers.
        Self {
            raw: unsafe { std::mem::zeroed() },
        }
    }
}

/// Bindgen-backed IAX/IAA completion record storage.
///
/// The ABI fields come from `linux/idxd.h` via bindgen
/// (`idxd_uapi::iax_completion_record`). This wrapper restores the 64-byte
/// alignment used by the IAX completion record contract.
#[repr(C, align(64))]
#[derive(Clone, Copy)]
pub struct BindgenIaxCompletionRecord {
    raw: idxd_uapi::iax_completion_record,
}

/// Public IAX/IAA completion helper type used by higher-level Rust bindings.
pub type IaxCompletionRecord = BindgenIaxCompletionRecord;

impl Default for BindgenIaxCompletionRecord {
    fn default() -> Self {
        // SAFETY: The generated completion record is plain C ABI storage and
        // IAX completion records are reset to an all-zero status before reuse.
        Self {
            raw: unsafe { std::mem::zeroed() },
        }
    }
}

pub const IAX_OPCODE_NOOP: u8 = idxd_uapi::iax_opcode::IAX_OPCODE_NOOP as u8;
pub const IAX_OPCODE_MEMMOVE: u8 = idxd_uapi::iax_opcode::IAX_OPCODE_MEMMOVE as u8;
pub const IAX_OPCODE_DECOMPRESS: u8 = idxd_uapi::iax_opcode::IAX_OPCODE_DECOMPRESS as u8;
pub const IAX_OPCODE_COMPRESS: u8 = idxd_uapi::iax_opcode::IAX_OPCODE_COMPRESS as u8;

/// IAX crc64 opcode from the IAX analytics operation encoding.
///
/// Some kernel headers expose only the base IAX opcode enum; keep this raw
/// constant local and guarded by layout/fill tests instead of inventing a
/// higher-level operation taxonomy in `idxd-sys`.
pub const IAX_OPCODE_CRC64: u8 = 0x44;

pub const IAX_STATUS_ANALYTICS_ERROR: u8 = 0x0a;
pub const IAX_CRC64_POLY_T10DIF: u64 = 0x8BB7_0000_0000_0000;
pub const IAX_CRC64_FLAGS_OFFSET: usize = 38;
pub const IAX_CRC64_POLY_OFFSET: usize = 56;
pub const IAX_CRC64_RESULT_OFFSET: usize = 32;

pub const IAX_COMP_NONE: u8 = idxd_uapi::iax_completion_status::IAX_COMP_NONE as u8;
pub const IAX_COMP_SUCCESS: u8 = idxd_uapi::iax_completion_status::IAX_COMP_SUCCESS as u8;
pub const IAX_COMP_PAGE_FAULT_IR: u8 =
    idxd_uapi::iax_completion_status::IAX_COMP_PAGE_FAULT_IR as u8;
pub const IAX_COMP_OUTBUF_OVERFLOW: u8 =
    idxd_uapi::iax_completion_status::IAX_COMP_OUTBUF_OVERFLOW as u8;
pub const IAX_COMP_STATUS_MASK: u8 = idxd_uapi::DSA_COMP_STATUS_MASK as u8;

impl BindgenIaxHwDesc {
    #[inline(always)]
    pub(crate) fn as_raw_ptr(&self) -> *const idxd_uapi::iax_hw_desc {
        ptr::addr_of!(self.raw)
    }

    #[inline(always)]
    fn as_raw_mut_ptr(&mut self) -> *mut idxd_uapi::iax_hw_desc {
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

    /// Read the generated descriptor first-source address field.
    #[inline(always)]
    pub fn src1_addr(&self) -> u64 {
        // SAFETY: Bindgen preserves `linux/idxd.h` packed layout with alignment
        // 1, so descriptor address fields may be unaligned inside the wrapper.
        unsafe { ptr::read_unaligned(ptr::addr_of!((*self.as_raw_ptr()).src1_addr)) }
    }

    /// Read the generated descriptor first-source size field.
    #[inline(always)]
    pub fn src1_size(&self) -> u32 {
        // SAFETY: Bindgen preserves `linux/idxd.h` packed layout with alignment
        // 1, so descriptor size fields may be unaligned inside the wrapper.
        unsafe { ptr::read_unaligned(ptr::addr_of!((*self.as_raw_ptr()).src1_size)) }
    }

    /// Read the crc64 flags field written at the IAX crc64 descriptor offset.
    #[inline(always)]
    pub fn crc64_flags(&self) -> u16 {
        // SAFETY: The offset is part of the raw crc64 descriptor contract and
        // is guarded by host-free layout tests. The field may be unaligned.
        unsafe {
            ptr::read_unaligned(
                (self.as_raw_ptr().cast::<u8>())
                    .add(IAX_CRC64_FLAGS_OFFSET)
                    .cast::<u16>(),
            )
        }
    }

    /// Read the crc64 polynomial field written at the IAX crc64 descriptor offset.
    #[inline(always)]
    pub fn crc64_poly(&self) -> u64 {
        // SAFETY: The offset is part of the raw crc64 descriptor contract and
        // is guarded by host-free layout tests. The field may be unaligned.
        unsafe {
            ptr::read_unaligned(
                (self.as_raw_ptr().cast::<u8>())
                    .add(IAX_CRC64_POLY_OFFSET)
                    .cast::<u64>(),
            )
        }
    }

    #[inline(always)]
    fn set_opcode_flags(&mut self, opcode: u8, extra_flags: u32) {
        let flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV | extra_flags;
        self.raw.set_pasid(0);
        self.raw.set_rsvd(0);
        self.raw.set_priv(0);
        self.raw.set_flags(flags & 0x00FF_FFFF);
        self.raw.set_opcode(opcode as u32);
    }

    /// Set completion record address.
    pub fn set_completion(&mut self, comp: &mut IaxCompletionRecord) {
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

    /// Fill for noop operation.
    pub fn fill_noop(&mut self) {
        *self = Self::default();
        self.set_opcode_flags(IAX_OPCODE_NOOP, 0);
    }

    /// Fill for IAX crc64 operation.
    pub fn fill_crc64(&mut self, src: *const u8, size: u32) {
        *self = Self::default();
        self.set_opcode_flags(IAX_OPCODE_CRC64, 0);
        // SAFETY: The generated descriptor is packed. Use unaligned stores into
        // generated fields and crc64 raw offsets while preserving the
        // caller-provided source address.
        unsafe {
            ptr::write_unaligned(
                ptr::addr_of_mut!((*self.as_raw_mut_ptr()).src1_addr),
                src as u64,
            );
            ptr::write_unaligned(ptr::addr_of_mut!((*self.as_raw_mut_ptr()).src1_size), size);
            ptr::write_unaligned(
                (self.as_raw_mut_ptr().cast::<u8>())
                    .add(IAX_CRC64_FLAGS_OFFSET)
                    .cast::<u16>(),
                0,
            );
            ptr::write_unaligned(
                (self.as_raw_mut_ptr().cast::<u8>())
                    .add(IAX_CRC64_POLY_OFFSET)
                    .cast::<u64>(),
                IAX_CRC64_POLY_T10DIF,
            );
        }
    }
}

impl BindgenIaxCompletionRecord {
    #[inline(always)]
    pub(crate) fn as_raw_ptr(&self) -> *const idxd_uapi::iax_completion_record {
        ptr::addr_of!(self.raw)
    }

    #[inline(always)]
    pub(crate) fn as_raw_mut_ptr(&mut self) -> *mut idxd_uapi::iax_completion_record {
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

    /// Read the IAX analytics error-code byte.
    #[inline(always)]
    pub fn error_code(&self) -> u8 {
        // SAFETY: `self.raw` is initialized completion-record storage. The
        // error-code byte is hardware-owned once status becomes non-zero, so a
        // volatile load preserves the raw diagnostic boundary.
        unsafe { ptr::read_volatile(ptr::addr_of!((*self.as_raw_ptr()).error_code)) }
    }

    /// Read the invalid-flags diagnostic field.
    #[inline(always)]
    pub fn invalid_flags(&self) -> u32 {
        // SAFETY: Bindgen preserves `linux/idxd.h` packed layout with alignment
        // 1, so multi-byte completion fields must be read with unaligned loads.
        unsafe { ptr::read_unaligned(ptr::addr_of!((*self.as_raw_ptr()).invalid_flags)) }
    }

    /// Read the faulting address from the generated completion record.
    #[inline(always)]
    pub fn fault_addr(&self) -> u64 {
        // SAFETY: Bindgen preserves `linux/idxd.h` packed layout with alignment
        // 1, so multi-byte completion fields must be read with unaligned loads.
        unsafe { ptr::read_unaligned(ptr::addr_of!((*self.as_raw_ptr()).fault_addr)) }
    }

    /// Read the crc64 result field from the IAX completion record.
    #[inline(always)]
    pub fn crc64(&self) -> u64 {
        // SAFETY: The offset is part of the raw crc64 completion contract and
        // is guarded by host-free layout tests. The field may be unaligned.
        unsafe {
            ptr::read_unaligned(
                (self.as_raw_ptr().cast::<u8>())
                    .add(IAX_CRC64_RESULT_OFFSET)
                    .cast::<u64>(),
            )
        }
    }
}

/// Poll an IAX completion record until status is non-zero.
/// Returns the masked status byte.
#[inline(always)]
pub fn poll_iax_completion(comp: &IaxCompletionRecord) -> u8 {
    const MAX_SPINS: u64 = 2_000_000_000;
    let mut spins: u64 = 0;
    loop {
        let status = comp.status();
        if status != IAX_COMP_NONE {
            return status & IAX_COMP_STATUS_MASK;
        }
        spins += 1;
        if spins >= MAX_SPINS {
            eprintln!("poll_iax_completion: timeout after {} spins", spins);
            return 0xFF;
        }
        core::hint::spin_loop();
    }
}

/// Reset an IAX completion record for reuse.
#[inline(always)]
pub fn reset_iax_completion(comp: &mut IaxCompletionRecord) {
    // SAFETY: `comp` is an initialized completion-record wrapper. Zeroing the
    // entire record restores the hardware contract's `IAX_COMP_NONE` state
    // before the caller reuses it for a new descriptor.
    unsafe {
        ptr::write_bytes(comp as *mut IaxCompletionRecord, 0, 1);
    }
}

/// Drain all in-flight IAX descriptors by polling every completion record to non-zero.
pub fn drain_iax_completions(comps: &[IaxCompletionRecord]) {
    for comp in comps {
        let status = comp.status();
        if status == IAX_COMP_NONE {
            poll_iax_completion(comp);
        }
    }
}

/// Touch the faulted page reported in an IAX completion record.
pub fn touch_iax_fault_page(comp: &IaxCompletionRecord) {
    let addr = comp.fault_addr();
    if addr != 0 {
        // SAFETY: The completion record reports the faulting virtual address.
        // The raw recovery contract is a volatile read+write touch of that page
        // so the OS installs a writable PTE before a caller retries DMA.
        unsafe {
            let p = addr as *mut u8;
            ptr::write_volatile(p, ptr::read_volatile(p));
        }
    }
}

pub fn crc16_t10dif(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if (crc & 0x8000) != 0 {
                crc = (crc << 1) ^ 0x8BB7;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

pub fn crc64_t10dif_field(data: &[u8]) -> u64 {
    (crc16_t10dif(data) as u64) << 48
}

const IDXD_OP_FLAG_CRAV: u32 = idxd_uapi::IDXD_OP_FLAG_CRAV;
const IDXD_OP_FLAG_RCR: u32 = idxd_uapi::IDXD_OP_FLAG_RCR;
