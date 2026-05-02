use super::{DsaFlag, DsaHwDesc, DsaOpcode};

impl DsaHwDesc {
    pub fn fill_compare(&mut self, src1: *const u8, src2: *const u8, size: u32) {
        self.prepare(
            DsaOpcode::Compare,
            DsaFlag::RequestCompletionRecord | DsaFlag::CompletionRecordAddressValid,
        );
        self.set_src_addr(src1 as u64);
        self.set_dst_addr(src2 as u64);
        self.set_xfer_size(size);
    }

    pub fn compare(&mut self, src1: &[u8], src2: &[u8]) {
        self.fill_compare(src1.as_ptr(), src2.as_ptr(), src1.len() as u32);
    }
}
