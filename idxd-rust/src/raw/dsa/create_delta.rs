use super::{DsaHwDesc, DsaOpcode};

impl DsaHwDesc {
    pub fn fill_create_delta(
        &mut self,
        src1: *const u8,
        src2: *const u8,
        size: u32,
        delta: *mut u8,
        max_delta_size: u32,
        expected_result_mask: u8,
    ) {
        self.prepare(DsaOpcode::CreateDelta, super::default_completion_flags());
        self.set_src_addr(src1 as u64);
        self.set_dst_addr(src2 as u64);
        self.set_xfer_size(size);
        self.set_op_u64(0, delta as u64);
        self.set_op_u32(8, max_delta_size);
        self.set_op_u8(16, expected_result_mask);
    }

    pub fn create_delta(
        &mut self,
        src1: &[u8],
        src2: &[u8],
        delta: &mut [u8],
        expected_result_mask: u8,
    ) {
        self.fill_create_delta(
            src1.as_ptr(),
            src2.as_ptr(),
            src1.len() as u32,
            delta.as_mut_ptr(),
            delta.len() as u32,
            expected_result_mask,
        );
    }
}
