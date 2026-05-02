use std::{
    env,
    future::Future,
    path::Path,
    pin::Pin,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
    time::{Duration, Instant},
};

use idxd_rust::{
    DsaCompletionRecord, DsaCompletionStatus, DsaDifCheck, DsaDifInsert, DsaDifUpdate, DsaEngine,
    DsaFlag, DsaHwDesc,
};

const DEVICE_ENV: &str = "IDXD_RUST_DSA_WQ";
const POLL_TIMEOUT: Duration = Duration::from_secs(2);

fn dsa_device_path() -> Option<String> {
    env::var(DEVICE_ENV).ok().filter(|value| !value.is_empty())
}

fn success_like(status: u8) -> bool {
    matches!(
        DsaCompletionStatus::mask(status),
        value if value == DsaCompletionStatus::Success.as_u8()
            || value == DsaCompletionStatus::SuccessPredicate.as_u8()
    )
}

fn noop_waker() -> Waker {
    unsafe fn clone(_: *const ()) -> RawWaker {
        RawWaker::new(std::ptr::null(), &VTABLE)
    }
    unsafe fn wake(_: *const ()) {}
    unsafe fn wake_by_ref(_: *const ()) {}
    unsafe fn drop(_: *const ()) {}

    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

    // SAFETY: The vtable functions do not dereference the null data pointer.
    unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
}

fn spin_block_on<F>(future: &mut F) -> F::Output
where
    F: Future + Unpin,
{
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    let deadline = Instant::now() + POLL_TIMEOUT;

    loop {
        match Pin::new(&mut *future).poll(&mut cx) {
            Poll::Ready(output) => return output,
            Poll::Pending => {
                assert!(Instant::now() < deadline, "async DSA operation timed out");
                core::hint::spin_loop();
            }
        }
    }
}

fn complete<F>(mut future: F) -> DsaCompletionRecord
where
    F: Future<Output = DsaCompletionRecord> + Unpin,
{
    spin_block_on(&mut future)
}

fn assert_success(completion: DsaCompletionRecord) {
    let status = completion.status();
    assert!(
        success_like(status),
        "async DSA operation completed with non-success status={status:#04x} result={:#04x} bytes_completed={} fault_addr={:#x}",
        completion.result(),
        completion.bytes_completed(),
        completion.fault_addr(),
    );
}

#[test]
fn async_dsa_operations_submit_then_poll_completion_on_hardware() {
    let Some(device) = dsa_device_path() else {
        eprintln!(
            "skipping async hardware DSA test: set {DEVICE_ENV}=/dev/dsa/wqX.Y and run through launch"
        );
        return;
    };

    let engine = DsaEngine::open(Path::new(&device)).expect("open DSA work queue");
    let flags = DsaFlag::RequestCompletionRecord | DsaFlag::CompletionRecordAddressValid;

    assert_success(complete(engine.noop(flags)));
    assert_success(complete(engine.drain(flags)));

    let src = [0x9c_u8; 256];
    let mut dst = [0_u8; 256];
    {
        let mut operation = engine.memmove(&src, &mut dst);
        assert!(!operation.submitted());
        let completion = spin_block_on(&mut operation);
        assert!(operation.submitted());
        assert_success(completion);
    }
    assert_eq!(dst, src);

    let mut fill_dst = [0_u8; 64];
    assert_success(complete(
        engine.memfill(0x1122_3344_5566_7788, &mut fill_dst),
    ));
    assert_eq!(
        fill_dst,
        [0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11]
            .repeat(8)
            .as_slice()
    );

    let cmp_a = [0x33_u8; 128];
    let cmp_b = [0x33_u8; 128];
    assert_success(complete(engine.compare(&cmp_a, &cmp_b)));
    assert_success(complete(
        engine.compare_value(&cmp_a, 0x3333_3333_3333_3333),
    ));

    assert_success(complete(engine.crc_gen(&src, 0)));

    let mut copy_crc_dst = [0_u8; 256];
    assert_success(complete(engine.copy_crc(&src, &mut copy_crc_dst, 0)));
    assert_eq!(copy_crc_dst, src);

    let mut dual_dst = vec![0_u8; 8192];
    let (dst1, rest) = dual_dst.split_at_mut(4096);
    let dst2 = &mut rest[..4096];
    assert_success(complete(engine.dualcast(&src, dst1, dst2)));
    assert_eq!(&dst1[..src.len()], &src);
    assert_eq!(&dst2[..src.len()], &src);

    let flush_target = [1_u8; 64];
    assert_success(complete(engine.cache_flush(&flush_target)));

    let mut delta = [0_u8; 256];
    assert_success(complete(
        engine.create_delta(&cmp_a, &cmp_b, &mut delta, 0xff),
    ));

    let mut apply_dst = [0_u8; 256];
    assert_success(complete(engine.apply_delta(&delta, &mut apply_dst)));

    let dif_check = DsaDifCheck::default();
    assert_success(complete(engine.dif_check(&src, dif_check)));

    let dif_insert = DsaDifInsert::default();
    let mut dif_insert_dst = [0_u8; 512];
    assert_success(complete(engine.dif_insert(
        &src,
        &mut dif_insert_dst,
        dif_insert,
    )));

    let mut dif_strip_dst = [0_u8; 512];
    assert_success(complete(engine.dif_strip(
        &dif_insert_dst,
        &mut dif_strip_dst,
        dif_check,
    )));

    let dif_update = DsaDifUpdate::default();
    let mut dif_update_dst = [0_u8; 512];
    assert_success(complete(engine.dif_update(
        &src,
        &mut dif_update_dst,
        dif_update,
    )));

    let mut sub_desc = [DsaHwDesc::default()];
    let mut sub_completion = [DsaCompletionRecord::default()];
    let batch_src = [0x7b_u8; 64];
    let mut batch_dst = [0_u8; 64];
    sub_desc[0].memmove(&batch_src, &mut batch_dst);
    sub_desc[0].set_completion(&mut sub_completion[0]);
    assert_success(complete(engine.batch(&sub_desc, flags)));
    assert_eq!(batch_dst, batch_src);
    assert!(success_like(sub_completion[0].status()));
}
