use super::{DsaHwDesc, DsaOpcode};

impl DsaHwDesc {
    pub fn fill_apply_delta(&mut self, delta: *const u8, dst: *mut u8, delta_record_size: u32) {
        self.prepare(DsaOpcode::ApplyDelta, super::default_completion_flags());
        self.set_src_addr(delta as u64);
        self.set_dst_addr(dst as u64);
        self.set_op_u32(0, delta_record_size);
    }

    pub fn apply_delta(&mut self, delta: &[u8], dst: &mut [u8]) {
        self.fill_apply_delta(delta.as_ptr(), dst.as_mut_ptr(), delta.len() as u32);
    }
}
