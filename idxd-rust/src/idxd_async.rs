use std::{
    future::Future,
    marker::PhantomData,
    path::Path,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{
    DsaCompletionRecord, DsaCompletionStatus, DsaDifCheck, DsaDifInsert, DsaDifUpdate, DsaFlags,
    DsaHwDesc, WqPortal, detect_wq_mode,
};

/// Minimal polling engine for DSA descriptor futures.
///
/// This is intentionally naive: a future submits its descriptor on first poll,
/// then later polls only read the completion status byte. There is no runtime,
/// interrupt, retry, cancellation, or validation policy here.
pub struct DsaEngine {
    portal: WqPortal,
    dedicated: bool,
}

impl DsaEngine {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let dedicated = detect_wq_mode(path);
        let portal = WqPortal::open(path)?;
        Ok(Self { portal, dedicated })
    }

    pub fn from_portal(portal: WqPortal, dedicated: bool) -> Self {
        Self { portal, dedicated }
    }

    pub fn submit_descriptor(&self, desc: DsaHwDesc) -> DsaOperation<'_, 'static> {
        DsaOperation::new(self, desc)
    }

    pub fn noop(&self, flags: DsaFlags) -> DsaOperation<'_, 'static> {
        let mut desc = DsaHwDesc::default();
        desc.noop(flags);
        DsaOperation::new(self, desc)
    }

    pub fn batch<'a>(&'a self, descs: &'a [DsaHwDesc], flags: DsaFlags) -> DsaOperation<'a, 'a> {
        let mut desc = DsaHwDesc::default();
        desc.batch(descs, flags);
        DsaOperation::new(self, desc)
    }

    pub fn drain(&self, flags: DsaFlags) -> DsaOperation<'_, 'static> {
        let mut desc = DsaHwDesc::default();
        desc.drain(flags);
        DsaOperation::new(self, desc)
    }

    pub fn memmove<'a>(&'a self, src: &'a [u8], dst: &'a mut [u8]) -> DsaOperation<'a, 'a> {
        let mut desc = DsaHwDesc::default();
        desc.memmove(src, dst);
        DsaOperation::new(self, desc)
    }

    pub fn memfill<'a>(&'a self, pattern: u64, dst: &'a mut [u8]) -> DsaOperation<'a, 'a> {
        let mut desc = DsaHwDesc::default();
        desc.memfill(pattern, dst);
        DsaOperation::new(self, desc)
    }

    pub fn compare<'a>(&'a self, src1: &'a [u8], src2: &'a [u8]) -> DsaOperation<'a, 'a> {
        let mut desc = DsaHwDesc::default();
        desc.compare(src1, src2);
        DsaOperation::new(self, desc)
    }

    pub fn compare_value<'a>(&'a self, src: &'a [u8], pattern: u64) -> DsaOperation<'a, 'a> {
        let mut desc = DsaHwDesc::default();
        desc.compare_value(src, pattern);
        DsaOperation::new(self, desc)
    }

    pub fn create_delta<'a>(
        &'a self,
        src1: &'a [u8],
        src2: &'a [u8],
        delta: &'a mut [u8],
        expected_result_mask: u8,
    ) -> DsaOperation<'a, 'a> {
        let mut desc = DsaHwDesc::default();
        desc.create_delta(src1, src2, delta, expected_result_mask);
        DsaOperation::new(self, desc)
    }

    pub fn apply_delta<'a>(&'a self, delta: &'a [u8], dst: &'a mut [u8]) -> DsaOperation<'a, 'a> {
        let mut desc = DsaHwDesc::default();
        desc.apply_delta(delta, dst);
        DsaOperation::new(self, desc)
    }

    pub fn dualcast<'a>(
        &'a self,
        src: &'a [u8],
        dst1: &'a mut [u8],
        dst2: &'a mut [u8],
    ) -> DsaOperation<'a, 'a> {
        let mut desc = DsaHwDesc::default();
        desc.dualcast(src, dst1, dst2);
        DsaOperation::new(self, desc)
    }

    pub fn crc_gen<'a>(&'a self, src: &'a [u8], seed: u32) -> DsaOperation<'a, 'a> {
        let mut desc = DsaHwDesc::default();
        desc.crc_gen(src, seed);
        DsaOperation::new(self, desc)
    }

    pub fn copy_crc<'a>(
        &'a self,
        src: &'a [u8],
        dst: &'a mut [u8],
        seed: u32,
    ) -> DsaOperation<'a, 'a> {
        let mut desc = DsaHwDesc::default();
        desc.copy_crc(src, dst, seed);
        DsaOperation::new(self, desc)
    }

    pub fn dif_check<'a>(&'a self, src: &'a [u8], dif: DsaDifCheck) -> DsaOperation<'a, 'a> {
        let mut desc = DsaHwDesc::default();
        desc.dif_check(src, dif);
        DsaOperation::new(self, desc)
    }

    pub fn dif_insert<'a>(
        &'a self,
        src: &'a [u8],
        dst: &'a mut [u8],
        dif: DsaDifInsert,
    ) -> DsaOperation<'a, 'a> {
        let mut desc = DsaHwDesc::default();
        desc.dif_insert(src, dst, dif);
        DsaOperation::new(self, desc)
    }

    pub fn dif_strip<'a>(
        &'a self,
        src: &'a [u8],
        dst: &'a mut [u8],
        dif: DsaDifCheck,
    ) -> DsaOperation<'a, 'a> {
        let mut desc = DsaHwDesc::default();
        desc.dif_strip(src, dst, dif);
        DsaOperation::new(self, desc)
    }

    pub fn dif_update<'a>(
        &'a self,
        src: &'a [u8],
        dst: &'a mut [u8],
        dif: DsaDifUpdate,
    ) -> DsaOperation<'a, 'a> {
        let mut desc = DsaHwDesc::default();
        desc.dif_update(src, dst, dif);
        DsaOperation::new(self, desc)
    }

    pub fn cache_flush<'a>(&'a self, addr: &'a [u8]) -> DsaOperation<'a, 'a> {
        let mut desc = DsaHwDesc::default();
        desc.cache_flush(addr);
        DsaOperation::new(self, desc)
    }

    fn submit(&self, desc: &DsaHwDesc) {
        // SAFETY: The operation future owns the descriptor and completion record,
        // and its Rusty constructors tie borrowed buffers to the future lifetime.
        unsafe { self.portal.submit_dsa(desc, self.dedicated) };
    }
}

/// A single DSA operation future.
///
/// First poll submits to hardware. Later polls check the completion record. The
/// future resolves with the completion record once hardware writes any non-zero
/// completion status.
pub struct DsaOperation<'engine, 'buffers> {
    engine: &'engine DsaEngine,
    desc: DsaHwDesc,
    completion: DsaCompletionRecord,
    submitted: bool,
    _buffers: PhantomData<&'buffers mut ()>,
}

impl<'engine, 'buffers> DsaOperation<'engine, 'buffers> {
    fn new(engine: &'engine DsaEngine, desc: DsaHwDesc) -> Self {
        Self {
            engine,
            desc,
            completion: DsaCompletionRecord::default(),
            submitted: false,
            _buffers: PhantomData,
        }
    }

    pub fn descriptor(&self) -> &DsaHwDesc {
        &self.desc
    }

    pub fn completion(&self) -> &DsaCompletionRecord {
        &self.completion
    }

    pub fn submitted(&self) -> bool {
        self.submitted
    }
}

impl Future for DsaOperation<'_, '_> {
    type Output = DsaCompletionRecord;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().get_mut();

        if !this.submitted {
            this.completion.clear();
            this.desc.set_completion(&mut this.completion);
            this.engine.submit(&this.desc);
            this.submitted = true;
        }

        if this.completion.status() != DsaCompletionStatus::None.as_u8() {
            Poll::Ready(this.completion)
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}
