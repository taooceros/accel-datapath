use super::{DsaFlag, DsaHwDesc, DsaOpcode};

impl DsaHwDesc {
    pub fn fill_cache_flush(&mut self, addr: *const u8, size: u32) {
        self.prepare(
            DsaOpcode::CacheFlush,
            DsaFlag::RequestCompletionRecord | DsaFlag::CompletionRecordAddressValid,
        );
        self.set_dst_addr(addr as u64);
        self.set_xfer_size(size);
    }

    pub fn cache_flush(&mut self, addr: &[u8]) {
        self.fill_cache_flush(addr.as_ptr(), addr.len() as u32);
    }
}
