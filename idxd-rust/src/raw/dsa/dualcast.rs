use super::{DsaHwDesc, DsaOpcode};

impl DsaHwDesc {
    pub fn fill_dualcast(&mut self, src: *const u8, dst1: *mut u8, dst2: *mut u8, size: u32) {
        self.prepare(DsaOpcode::Dualcast, super::default_completion_flags());
        self.set_src_addr(src as u64);
        self.set_dst_addr(dst1 as u64);
        self.set_xfer_size(size);
        self.set_op_u64(0, dst2 as u64);
    }

    pub fn dualcast(&mut self, src: &[u8], dst1: &mut [u8], dst2: &mut [u8]) {
        self.fill_dualcast(
            src.as_ptr(),
            dst1.as_mut_ptr(),
            dst2.as_mut_ptr(),
            src.len() as u32,
        );
    }
}
