//! DSA-specific descriptor/completion definitions and helpers.

use crate::submit::{poll_status, zero_record, IDXD_OP_FLAG_CC, IDXD_OP_FLAG_CRAV, IDXD_OP_FLAG_RCR};

#[repr(C, align(64))]
#[derive(Clone, Copy)]
pub struct DsaHwDesc {
    pub pasid_priv: u32,
    pub flags_opcode: u32,
    pub completion_addr: u64,
    pub src_addr: u64,
    pub dst_addr: u64,
    pub xfer_size: u32,
    pub int_handle: u16,
    pub rsvd1: u16,
    pub op_specific: [u8; 24],
}

impl Default for DsaHwDesc {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C, align(32))]
#[derive(Clone, Copy)]
pub struct DsaCompletionRecord {
    pub status: u8,
    pub result: u8,
    pub rsvd: u16,
    pub bytes_completed: u32,
    pub fault_addr: u64,
    pub op_specific: [u8; 16],
}

impl Default for DsaCompletionRecord {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

impl DsaCompletionRecord {
    pub fn crc_value(&self) -> u64 {
        u64::from_le_bytes(self.op_specific[..8].try_into().unwrap())
    }
}

pub const DSA_OPCODE_NOOP: u8 = 0x00;
pub const DSA_OPCODE_BATCH: u8 = 0x01;
pub const DSA_OPCODE_MEMMOVE: u8 = 0x03;
pub const DSA_OPCODE_MEMFILL: u8 = 0x04;
pub const DSA_OPCODE_COMPARE: u8 = 0x05;
pub const DSA_OPCODE_CRCGEN: u8 = 0x10;
pub const DSA_OPCODE_COPY_CRC: u8 = 0x11;

pub const DSA_COMP_NONE: u8 = 0;
pub const DSA_COMP_SUCCESS: u8 = 1;
pub const DSA_COMP_PAGE_FAULT_NOBOF: u8 = 3;

impl DsaHwDesc {
    #[inline(always)]
    fn set_opcode_flags(&mut self, opcode: u8, extra_flags: u32) {
        let flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV | extra_flags;
        self.flags_opcode = (flags & 0x00FF_FFFF) | ((opcode as u32) << 24);
    }

    #[inline(always)]
    pub fn set_completion(&mut self, comp: &mut DsaCompletionRecord) {
        self.completion_addr = comp as *mut DsaCompletionRecord as u64;
    }

    #[inline(always)]
    pub fn fill_memmove(&mut self, src: *const u8, dst: *mut u8, size: u32) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_MEMMOVE, IDXD_OP_FLAG_CC);
        self.src_addr = src as u64;
        self.dst_addr = dst as u64;
        self.xfer_size = size;
    }

    #[inline(always)]
    pub fn fill_crc_gen(&mut self, src: *const u8, size: u32, seed: u32) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_CRCGEN, 0);
        self.src_addr = src as u64;
        self.xfer_size = size;
        self.op_specific[0..4].copy_from_slice(&seed.to_le_bytes());
    }

    #[inline(always)]
    pub fn fill_copy_crc(&mut self, src: *const u8, dst: *mut u8, size: u32, seed: u32) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_COPY_CRC, 0);
        self.src_addr = src as u64;
        self.dst_addr = dst as u64;
        self.xfer_size = size;
        self.op_specific[0..4].copy_from_slice(&seed.to_le_bytes());
    }

    #[inline(always)]
    pub fn fill_memfill(&mut self, dst: *mut u8, size: u32, pattern: u64) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_MEMFILL, IDXD_OP_FLAG_CC);
        self.src_addr = pattern;
        self.dst_addr = dst as u64;
        self.xfer_size = size;
    }

    #[inline(always)]
    pub fn fill_compare(&mut self, src1: *const u8, src2: *const u8, size: u32) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_COMPARE, 0);
        self.src_addr = src1 as u64;
        self.dst_addr = src2 as u64;
        self.xfer_size = size;
    }

    #[inline(always)]
    pub fn fill_batch(&mut self, desc_list: *const DsaHwDesc, count: u32) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_BATCH, 0);
        self.src_addr = desc_list as u64;
        self.xfer_size = count;
    }

    #[inline(always)]
    pub fn fill_noop(&mut self) {
        *self = Self::default();
        self.set_opcode_flags(DSA_OPCODE_NOOP, 0);
    }
}

impl crate::submit::WqPortal {
    #[inline(always)]
    pub unsafe fn submit(&self, desc: &DsaHwDesc) {
        self.submit_desc64(desc as *const DsaHwDesc as *const u8);
    }
}

#[inline(always)]
pub fn poll_completion(comp: &DsaCompletionRecord) -> u8 {
    poll_status(&comp.status as *const u8, DSA_COMP_NONE)
}

#[inline(always)]
pub fn reset_completion(comp: &mut DsaCompletionRecord) {
    zero_record(comp);
}

pub fn drain_completions(comps: &[DsaCompletionRecord]) {
    for comp in comps {
        let status = unsafe { std::ptr::read_volatile(&comp.status) };
        if status == DSA_COMP_NONE {
            poll_completion(comp);
        }
    }
}

pub fn touch_fault_page(comp: &DsaCompletionRecord) {
    let addr = unsafe { std::ptr::read_volatile(&comp.fault_addr) };
    if addr != 0 {
        unsafe {
            let p = addr as *mut u8;
            std::ptr::write_volatile(p, std::ptr::read_volatile(p));
        }
    }
}
