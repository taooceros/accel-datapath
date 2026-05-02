use super::{DsaFlag, DsaHwDesc, DsaOpcode};

impl DsaHwDesc {
    pub fn fill_compare_value(&mut self, src: *const u8, pattern: u64, size: u32) {
        self.prepare(
            DsaOpcode::CompareValue,
            DsaFlag::RequestCompletionRecord | DsaFlag::CompletionRecordAddressValid,
        );
        self.set_src_addr(src as u64);
        self.set_dst_addr(pattern);
        self.set_xfer_size(size);
    }

    pub fn compare_value(&mut self, src: &[u8], pattern: u64) {
        self.fill_compare_value(src.as_ptr(), pattern, src.len() as u32);
    }
}
