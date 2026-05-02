use super::{DsaHwDesc, DsaOpcode};

impl DsaHwDesc {
    pub fn fill_memfill(&mut self, pattern: u64, dst: *mut u8, size: u32) {
        self.prepare(DsaOpcode::Memfill, super::default_completion_flags());
        self.set_src_addr(pattern);
        self.set_dst_addr(dst as u64);
        self.set_xfer_size(size);
    }

    pub fn memfill(&mut self, pattern: u64, dst: &mut [u8]) {
        self.fill_memfill(pattern, dst.as_mut_ptr(), dst.len() as u32);
    }
}
