use super::{DsaFlags, DsaHwDesc, DsaOpcode};

impl DsaHwDesc {
    pub fn fill_noop(&mut self, flags: DsaFlags) {
        self.prepare(DsaOpcode::Noop, flags);
    }

    pub fn noop(&mut self, flags: DsaFlags) {
        self.fill_noop(flags);
    }
}
