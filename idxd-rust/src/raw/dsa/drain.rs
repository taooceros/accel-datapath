use super::{DsaFlags, DsaHwDesc, DsaOpcode};

impl DsaHwDesc {
    pub fn fill_drain(&mut self, flags: DsaFlags) {
        self.prepare(DsaOpcode::Drain, flags);
    }

    pub fn drain(&mut self, flags: DsaFlags) {
        self.fill_drain(flags);
    }
}
