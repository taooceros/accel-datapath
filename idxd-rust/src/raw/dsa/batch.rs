use super::{DsaFlags, DsaHwDesc, DsaOpcode};

impl DsaHwDesc {
    pub fn fill_batch(&mut self, desc_list: *const DsaHwDesc, desc_count: u32, flags: DsaFlags) {
        self.prepare(DsaOpcode::Batch, flags);
        self.set_src_addr(desc_list as u64);
        self.set_xfer_size(desc_count);
    }

    pub fn batch(&mut self, descs: &[DsaHwDesc], flags: DsaFlags) {
        self.fill_batch(descs.as_ptr(), descs.len() as u32, flags);
    }
}
