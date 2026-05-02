use super::{DsaHwDesc, DsaOpcode};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DsaDifUpdate {
    pub src_flags: u8,
    pub dest_flags: u8,
    pub flags: u8,
    pub src_ref_tag_seed: u32,
    pub src_app_tag_mask: u16,
    pub src_app_tag_seed: u16,
    pub dest_ref_tag_seed: u32,
    pub dest_app_tag_mask: u16,
    pub dest_app_tag_seed: u16,
}

impl DsaHwDesc {
    pub fn fill_dif_update(&mut self, src: *const u8, dst: *mut u8, size: u32, dif: DsaDifUpdate) {
        self.prepare(DsaOpcode::DifUpdate, super::default_completion_flags());
        self.set_src_addr(src as u64);
        self.set_dst_addr(dst as u64);
        self.set_xfer_size(size);
        self.set_op_u8(0, dif.src_flags);
        self.set_op_u8(1, dif.dest_flags);
        self.set_op_u8(2, dif.flags);
        self.set_op_u32(8, dif.src_ref_tag_seed);
        self.set_op_u16(12, dif.src_app_tag_mask);
        self.set_op_u16(14, dif.src_app_tag_seed);
        self.set_op_u32(16, dif.dest_ref_tag_seed);
        self.set_op_u16(20, dif.dest_app_tag_mask);
        self.set_op_u16(22, dif.dest_app_tag_seed);
    }

    pub fn dif_update(&mut self, src: &[u8], dst: &mut [u8], dif: DsaDifUpdate) {
        self.fill_dif_update(src.as_ptr(), dst.as_mut_ptr(), src.len() as u32, dif);
    }
}
