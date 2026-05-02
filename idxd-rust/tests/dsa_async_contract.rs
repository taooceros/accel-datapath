use std::{future::Future, io::ErrorKind, path::Path};

use idxd_rust::{
    DsaCompletionRecord, DsaDifCheck, DsaDifInsert, DsaDifUpdate, DsaEngine, DsaFlag, DsaHwDesc,
    DsaOperation,
};

fn assert_completion_future<T: Future<Output = DsaCompletionRecord>>() {}

#[test]
fn dsa_operation_is_a_completion_future() {
    assert_completion_future::<DsaOperation<'static, 'static>>();
}

#[test]
fn idxd_async_module_path_exports_dsa_types() {
    assert_completion_future::<idxd_rust::idxd_async::DsaOperation<'static, 'static>>();
}

#[test]
fn engine_open_reports_missing_queue_before_submission() {
    let err = match DsaEngine::open(Path::new("/tmp/idxd-rust-missing-dsa-wq")) {
        Ok(_) => panic!("unexpectedly opened missing DSA work queue"),
        Err(err) => err,
    };
    assert_eq!(err.kind(), ErrorKind::NotFound);
}

#[allow(dead_code)]
fn async_engine_wraps_every_dsa_operation(
    engine: &DsaEngine,
    src: &[u8],
    src2: &[u8],
    delta: &mut [u8],
    dst: &mut [u8],
    dst2: &mut [u8],
) {
    let flags = DsaFlag::RequestCompletionRecord | DsaFlag::CompletionRecordAddressValid;

    {
        let desc = DsaHwDesc::default();
        let _operation = engine.submit_descriptor(desc);
    }
    {
        let _operation = engine.noop(flags);
    }
    {
        let descs = [DsaHwDesc::default()];
        let _operation = engine.batch(&descs, flags);
    }
    {
        let _operation = engine.drain(flags);
    }
    {
        let _operation = engine.memmove(src, dst);
    }
    {
        let _operation = engine.memfill(0, dst);
    }
    {
        let _operation = engine.compare(src, src2);
    }
    {
        let _operation = engine.compare_value(src, 0);
    }
    {
        let _operation = engine.create_delta(src, src2, delta, 0xff);
    }
    {
        let _operation = engine.apply_delta(delta, dst);
    }
    {
        let _operation = engine.dualcast(src, dst, dst2);
    }
    {
        let _operation = engine.crc_gen(src, 0);
    }
    {
        let _operation = engine.copy_crc(src, dst, 0);
    }
    {
        let _operation = engine.dif_check(src, DsaDifCheck::default());
    }
    {
        let _operation = engine.dif_insert(src, dst, DsaDifInsert::default());
    }
    {
        let _operation = engine.dif_strip(src, dst, DsaDifCheck::default());
    }
    {
        let _operation = engine.dif_update(src, dst, DsaDifUpdate::default());
    }
    {
        let _operation = engine.cache_flush(src);
    }
}
