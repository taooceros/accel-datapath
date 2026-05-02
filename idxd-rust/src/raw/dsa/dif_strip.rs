use super::{DsaDifCheck, DsaHwDesc, DsaOpcode};

impl DsaHwDesc {
    pub fn fill_dif_strip(&mut self, src: *const u8, dst: *mut u8, size: u32, dif: DsaDifCheck) {
        self.prepare(DsaOpcode::DifStrip, super::default_completion_flags());
        self.set_src_addr(src as u64);
        self.set_dst_addr(dst as u64);
        self.set_xfer_size(size);
        self.set_dif_check(dif);
    }

    pub fn dif_strip(&mut self, src: &[u8], dst: &mut [u8], dif: DsaDifCheck) {
        self.fill_dif_strip(src.as_ptr(), dst.as_mut_ptr(), src.len() as u32, dif);
    }
}
