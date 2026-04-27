//! IAX-specific descriptor/completion helpers backed by bindgen-generated
//! `linux/idxd.h` definitions.

use crate::submit::{poll_status, zero_record};
use idxd_sys::idxd;

#[repr(C, align(64))]
#[derive(Clone, Copy)]
pub struct IaxHwDesc(pub idxd::iax_hw_desc);

impl Default for IaxHwDesc {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C, align(64))]
#[derive(Clone, Copy)]
pub struct IaxCompletionRecord(pub idxd::iax_completion_record);

impl Default for IaxCompletionRecord {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

pub const IAX_OPCODE_NOOP: u8 = idxd::iax_opcode::IAX_OPCODE_NOOP as u8;
pub const IAX_OPCODE_MEMMOVE: u8 = idxd::iax_opcode::IAX_OPCODE_MEMMOVE as u8;
pub const IAX_OPCODE_DECOMPRESS: u8 = idxd::iax_opcode::IAX_OPCODE_DECOMPRESS as u8;
pub const IAX_OPCODE_COMPRESS: u8 = idxd::iax_opcode::IAX_OPCODE_COMPRESS as u8;
pub const IAX_OPCODE_CRC64: u8 = 0x44;

pub const IAX_STATUS_ANALYTICS_ERROR: u8 = 0x0a;
pub const IAX_CRC64_POLY_T10DIF: u64 = 0x8BB7_0000_0000_0000;

pub const IAX_COMP_NONE: u8 = idxd::iax_completion_status::IAX_COMP_NONE as u8;
pub const IAX_COMP_SUCCESS: u8 = idxd::iax_completion_status::IAX_COMP_SUCCESS as u8;
pub const IAX_COMP_PAGE_FAULT_IR: u8 = idxd::iax_completion_status::IAX_COMP_PAGE_FAULT_IR as u8;
pub const IAX_COMP_OUTBUF_OVERFLOW: u8 =
    idxd::iax_completion_status::IAX_COMP_OUTBUF_OVERFLOW as u8;

impl IaxHwDesc {
    #[inline(always)]
    fn set_opcode_flags(&mut self, opcode: u8, extra_flags: u32) {
        let flags = (idxd::IDXD_OP_FLAG_RCR | idxd::IDXD_OP_FLAG_CRAV) as u32 | extra_flags;
        self.0.set_pasid(0);
        self.0.set_rsvd(0);
        self.0.set_priv(0);
        self.0.set_flags(flags);
        self.0.set_opcode(opcode as u32);
    }

    #[inline(always)]
    pub fn set_completion(&mut self, comp: &mut IaxCompletionRecord) {
        self.0.completion_addr = std::ptr::addr_of_mut!(comp.0) as u64;
    }

    #[inline(always)]
    pub fn fill_noop(&mut self) {
        *self = Self::default();
        self.set_opcode_flags(IAX_OPCODE_NOOP, 0);
    }

    #[inline(always)]
    pub fn fill_memmove(&mut self, src: *const u8, dst: *mut u8, size: u32) {
        *self = Self::default();
        self.set_opcode_flags(IAX_OPCODE_MEMMOVE, idxd::IDXD_OP_FLAG_CC as u32);
        self.0.src1_addr = src as u64;
        self.0.dst_addr = dst as u64;
        self.0.src1_size = size;
    }

    #[inline(always)]
    pub fn fill_crc64(&mut self, src: *const u8, size: u32) {
        *self = Self::default();
        self.set_opcode_flags(IAX_OPCODE_CRC64, 0);
        self.0.src1_addr = src as u64;
        self.0.src1_size = size;

        unsafe {
            let base = std::ptr::addr_of_mut!(self.0) as *mut u8;
            std::ptr::write_unaligned(base.add(38) as *mut u16, 0);
            std::ptr::write_unaligned(base.add(56) as *mut u64, IAX_CRC64_POLY_T10DIF);
        }
    }
}

impl crate::submit::WqPortal {
    #[inline(always)]
    pub unsafe fn submit_iax(&self, desc: &IaxHwDesc) {
        self.submit_desc64(std::ptr::addr_of!(desc.0) as *const u8);
    }
}

#[inline(always)]
pub fn poll_completion(comp: &IaxCompletionRecord) -> u8 {
    poll_status(std::ptr::addr_of!(comp.0.status), IAX_COMP_NONE)
}

#[inline(always)]
pub fn reset_completion(comp: &mut IaxCompletionRecord) {
    zero_record(&mut comp.0);
}

#[inline(always)]
pub fn completion_error_code(comp: &IaxCompletionRecord) -> u8 {
    unsafe { std::ptr::read_volatile(std::ptr::addr_of!(comp.0.error_code)) }
}

#[inline(always)]
pub fn completion_invalid_flags(comp: &IaxCompletionRecord) -> u32 {
    unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(comp.0.invalid_flags)) }
}

#[inline(always)]
pub fn completion_crc64(comp: &IaxCompletionRecord) -> u64 {
    unsafe {
        let base = std::ptr::addr_of!(comp.0) as *const u8;
        std::ptr::read_unaligned(base.add(32) as *const u64)
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

pub fn drain_completions(comps: &[IaxCompletionRecord]) {
    for comp in comps {
        let status = unsafe { std::ptr::read_volatile(std::ptr::addr_of!(comp.0.status)) };
        if status == IAX_COMP_NONE {
            poll_completion(comp);
        }
    }
}

pub fn touch_fault_page(comp: &IaxCompletionRecord) {
    let addr = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(comp.0.fault_addr)) };
    if addr != 0 {
        unsafe {
            let p = addr as *mut u8;
            std::ptr::write_volatile(p, std::ptr::read_volatile(p));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc16_t10dif_matches_known_vector() {
        assert_eq!(crc16_t10dif(b"123456789"), 0xD0DB);
    }

    #[test]
    fn crc64_t10dif_field_packs_crc_in_msb_bits() {
        assert_eq!(crc64_t10dif_field(b"123456789"), 0xD0DB_0000_0000_0000);
    }

    #[test]
    fn completion_crc64_reads_64_bit_field_at_offset_32() {
        let mut comp = IaxCompletionRecord::default();
        unsafe {
            let base = std::ptr::addr_of_mut!(comp.0) as *mut u8;
            std::ptr::write_unaligned(base.add(32) as *mut u64, 0xD0DB_0000_0000_0000);
        }
        assert_eq!(completion_crc64(&comp), 0xD0DB_0000_0000_0000);
    }

    #[test]
    fn fill_crc64_populates_expected_descriptor_fields() {
        let data = [0xABu8; 16];
        let mut desc = IaxHwDesc::default();
        desc.fill_crc64(data.as_ptr(), data.len() as u32);

        assert_eq!(desc.0.opcode(), IAX_OPCODE_CRC64 as u32);
        let src1_addr = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(desc.0.src1_addr)) };
        let src1_size = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(desc.0.src1_size)) };
        assert_eq!(src1_addr, data.as_ptr() as u64);
        assert_eq!(src1_size, data.len() as u32);
        assert_eq!(
            desc.0.flags(),
            (idxd::IDXD_OP_FLAG_RCR | idxd::IDXD_OP_FLAG_CRAV) as u32
        );

        unsafe {
            let base = std::ptr::addr_of!(desc.0) as *const u8;
            assert_eq!(std::ptr::read_unaligned(base.add(38) as *const u16), 0);
            assert_eq!(
                std::ptr::read_unaligned(base.add(56) as *const u64),
                IAX_CRC64_POLY_T10DIF
            );
        }
    }
}
