use super::{DsaFlag, DsaHwDesc, DsaOpcode};

impl DsaHwDesc {
    pub fn fill_copy_crc(
        &mut self,
        src: *const u8,
        dst: *mut u8,
        size: u32,
        crc_seed: u32,
        seed_addr: u64,
    ) {
        self.prepare(
            DsaOpcode::CopyCrc,
            DsaFlag::RequestCompletionRecord | DsaFlag::CompletionRecordAddressValid,
        );
        self.set_src_addr(src as u64);
        self.set_dst_addr(dst as u64);
        self.set_xfer_size(size);
        self.set_op_u32(0, crc_seed);
        self.set_op_u64(8, seed_addr);
    }

    pub fn copy_crc(&mut self, src: &[u8], dst: &mut [u8], seed: u32) {
        self.fill_copy_crc(src.as_ptr(), dst.as_mut_ptr(), src.len() as u32, seed, 0);
    }
}
