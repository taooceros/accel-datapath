use super::{DsaHwDesc, DsaOpcode};

impl DsaHwDesc {
    pub fn fill_memmove(&mut self, src: *const u8, dst: *mut u8, size: u32) {
        self.prepare(DsaOpcode::Memmove, super::default_completion_flags());
        self.set_src_addr(src as u64);
        self.set_dst_addr(dst as u64);
        self.set_xfer_size(size);
    }

    pub fn memmove(&mut self, src: &[u8], dst: &mut [u8]) {
        self.fill_memmove(src.as_ptr(), dst.as_mut_ptr(), src.len() as u32);
    }
}
