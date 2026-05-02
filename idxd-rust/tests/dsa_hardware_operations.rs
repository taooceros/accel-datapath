use std::{
    env,
    path::Path,
    time::{Duration, Instant},
};

use idxd_rust::{
    DsaCompletionRecord, DsaCompletionStatus, DsaDifCheck, DsaDifInsert, DsaDifUpdate, DsaFlag,
    DsaHwDesc, WqPortal, detect_wq_mode,
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

fn submit_and_wait(
    portal: &WqPortal,
    dedicated: bool,
    name: &str,
    desc: &mut DsaHwDesc,
    completion: &mut DsaCompletionRecord,
) {
    completion.clear();
    desc.set_completion(completion);

    // SAFETY: This test keeps the descriptor, completion record, and referenced
    // buffers alive until a non-empty completion status is observed.
    unsafe { portal.submit_dsa(desc, dedicated) };

    let deadline = Instant::now() + POLL_TIMEOUT;
    loop {
        let status = completion.status();
        if status != DsaCompletionStatus::None.as_u8() {
            assert!(
                success_like(status),
                "{name} completed with non-success status={status:#04x} result={:#04x} bytes_completed={} fault_addr={:#x}",
                completion.result(),
                completion.bytes_completed(),
                completion.fault_addr(),
            );
            return;
        }
        assert!(
            Instant::now() < deadline,
            "{name} timed out waiting for DSA completion"
        );
        core::hint::spin_loop();
    }
}

fn submit_one<F>(portal: &WqPortal, dedicated: bool, name: &str, fill: F)
where
    F: FnOnce(&mut DsaHwDesc),
{
    let mut desc = DsaHwDesc::default();
    let mut completion = DsaCompletionRecord::default();
    fill(&mut desc);
    submit_and_wait(portal, dedicated, name, &mut desc, &mut completion);
}

#[test]
fn hardware_dsa_operations_complete_successfully() {
    let Some(device) = dsa_device_path() else {
        eprintln!(
            "skipping hardware DSA operation test: set {DEVICE_ENV}=/dev/dsa/wqX.Y and run through launch"
        );
        return;
    };

    let path = Path::new(&device);
    let dedicated = detect_wq_mode(path);
    let portal = WqPortal::open(path).expect("open DSA work queue");

    submit_one(&portal, dedicated, "noop", |desc| {
        desc.noop(DsaFlag::RequestCompletionRecord | DsaFlag::CompletionRecordAddressValid);
    });

    submit_one(&portal, dedicated, "drain", |desc| {
        desc.drain(DsaFlag::RequestCompletionRecord | DsaFlag::CompletionRecordAddressValid);
    });

    let src = [0x5a_u8; 256];
    let mut dst = [0_u8; 256];
    submit_one(&portal, dedicated, "memmove", |desc| {
        desc.memmove(&src, &mut dst);
    });
    assert_eq!(dst, src);

    let mut fill_dst = [0_u8; 64];
    submit_one(&portal, dedicated, "memfill", |desc| {
        desc.memfill(0x1122_3344_5566_7788, &mut fill_dst);
    });
    assert_eq!(
        fill_dst,
        [0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11]
            .repeat(8)
            .as_slice()
    );

    let cmp_a = [0x33_u8; 128];
    let cmp_b = [0x33_u8; 128];
    submit_one(&portal, dedicated, "compare", |desc| {
        desc.compare(&cmp_a, &cmp_b);
    });

    submit_one(&portal, dedicated, "compare_value", |desc| {
        desc.compare_value(&cmp_a, 0x3333_3333_3333_3333);
    });

    submit_one(&portal, dedicated, "crc_gen", |desc| {
        desc.crc_gen(&src, 0);
    });

    let mut copy_crc_dst = [0_u8; 256];
    submit_one(&portal, dedicated, "copy_crc", |desc| {
        desc.copy_crc(&src, &mut copy_crc_dst, 0);
    });
    assert_eq!(copy_crc_dst, src);

    let mut dual_dst = vec![0_u8; 8192];
    let (dst1, rest) = dual_dst.split_at_mut(4096);
    let dst2 = &mut rest[..4096];
    submit_one(&portal, dedicated, "dualcast", |desc| {
        desc.dualcast(&src, dst1, dst2);
    });
    assert_eq!(&dst1[..src.len()], &src);
    assert_eq!(&dst2[..src.len()], &src);

    let flush_target = [1_u8; 64];
    submit_one(&portal, dedicated, "cache_flush", |desc| {
        desc.cache_flush(&flush_target);
    });

    let mut delta = [0_u8; 256];
    submit_one(&portal, dedicated, "create_delta", |desc| {
        desc.create_delta(&cmp_a, &cmp_b, &mut delta, 0xff);
    });

    let mut apply_dst = [0_u8; 256];
    submit_one(&portal, dedicated, "apply_delta", |desc| {
        desc.apply_delta(&delta, &mut apply_dst);
    });

    let dif_check = DsaDifCheck::default();
    submit_one(&portal, dedicated, "dif_check", |desc| {
        desc.dif_check(&src, dif_check);
    });

    let dif_insert = DsaDifInsert::default();
    let mut dif_insert_dst = [0_u8; 512];
    submit_one(&portal, dedicated, "dif_insert", |desc| {
        desc.dif_insert(&src, &mut dif_insert_dst, dif_insert);
    });

    let mut dif_strip_dst = [0_u8; 512];
    submit_one(&portal, dedicated, "dif_strip", |desc| {
        desc.dif_strip(&dif_insert_dst, &mut dif_strip_dst, dif_check);
    });

    let dif_update = DsaDifUpdate::default();
    let mut dif_update_dst = [0_u8; 512];
    submit_one(&portal, dedicated, "dif_update", |desc| {
        desc.dif_update(&src, &mut dif_update_dst, dif_update);
    });

    let mut sub_desc = [DsaHwDesc::default()];
    let mut sub_completion = [DsaCompletionRecord::default()];
    let batch_src = [0x7b_u8; 64];
    let mut batch_dst = [0_u8; 64];
    sub_desc[0].memmove(&batch_src, &mut batch_dst);
    sub_desc[0].set_completion(&mut sub_completion[0]);
    submit_one(&portal, dedicated, "batch", |desc| {
        desc.batch(
            &sub_desc,
            DsaFlag::RequestCompletionRecord | DsaFlag::CompletionRecordAddressValid,
        );
    });
    assert_eq!(batch_dst, batch_src);
    assert!(success_like(sub_completion[0].status()));
}
