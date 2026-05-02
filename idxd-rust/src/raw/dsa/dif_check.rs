use super::{DsaHwDesc, DsaOpcode};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DsaDifCheck {
    pub src_dif_flags: u8,
    pub flags: u8,
    pub ref_tag_seed: u32,
    pub app_tag_mask: u16,
    pub app_tag_seed: u16,
}

impl DsaHwDesc {
    pub fn fill_dif_check(&mut self, src: *const u8, size: u32, dif: DsaDifCheck) {
        self.prepare(DsaOpcode::DifCheck, super::default_completion_flags());
        self.set_src_addr(src as u64);
        self.set_xfer_size(size);
        self.set_dif_check(dif);
    }

    pub fn dif_check(&mut self, src: &[u8], dif: DsaDifCheck) {
        self.fill_dif_check(src.as_ptr(), src.len() as u32, dif);
    }

    pub(super) fn set_dif_check(&mut self, dif: DsaDifCheck) {
        self.set_op_u8(0, dif.src_dif_flags);
        self.set_op_u8(2, dif.flags);
        self.set_op_u32(8, dif.ref_tag_seed);
        self.set_op_u16(12, dif.app_tag_mask);
        self.set_op_u16(14, dif.app_tag_seed);
    }
}
