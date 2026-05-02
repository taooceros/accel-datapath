use super::{DsaHwDesc, DsaOpcode};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DsaDifInsert {
    pub dest_dif_flags: u8,
    pub flags: u8,
    pub ref_tag_seed: u32,
    pub app_tag_mask: u16,
    pub app_tag_seed: u16,
}

impl DsaHwDesc {
    pub fn fill_dif_insert(&mut self, src: *const u8, dst: *mut u8, size: u32, dif: DsaDifInsert) {
        self.prepare(DsaOpcode::DifInsert, super::default_completion_flags());
        self.set_src_addr(src as u64);
        self.set_dst_addr(dst as u64);
        self.set_xfer_size(size);
        self.set_op_u8(1, dif.dest_dif_flags);
        self.set_op_u8(2, dif.flags);
        self.set_op_u32(16, dif.ref_tag_seed);
        self.set_op_u16(20, dif.app_tag_mask);
        self.set_op_u16(22, dif.app_tag_seed);
    }

    pub fn dif_insert(&mut self, src: &[u8], dst: &mut [u8], dif: DsaDifInsert) {
        self.fill_dif_insert(src.as_ptr(), dst.as_mut_ptr(), src.len() as u32, dif);
    }
}
