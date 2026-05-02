use super::{DsaFlag, DsaHwDesc, DsaOpcode};

impl DsaHwDesc {
    pub fn fill_crc_gen(&mut self, src: *const u8, size: u32, crc_seed: u32, seed_addr: u64) {
        self.prepare(
            DsaOpcode::CrcGen,
            DsaFlag::RequestCompletionRecord | DsaFlag::CompletionRecordAddressValid,
        );
        self.set_src_addr(src as u64);
        self.set_xfer_size(size);
        self.set_op_u32(0, crc_seed);
        self.set_op_u64(8, seed_addr);
    }

    pub fn crc_gen(&mut self, src: &[u8], seed: u32) {
        self.fill_crc_gen(src.as_ptr(), src.len() as u32, seed, 0);
    }
}
