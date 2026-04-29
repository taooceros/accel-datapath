use idxd_sys::{
    DsaCompletionRecord, EnqcmdSubmission, WqPortal, cpu_numa_node, current_core, cycles_to_ns,
    device_numa_node, drain_completions, flush_range, lfence, pin_to_core, poll_completion, rdtscp,
    reset_completion, touch_fault_page,
};
use std::io::ErrorKind;
use std::path::Path;

#[test]
fn missing_wq_open_preserves_std_io_error_kind() {
    let path = Path::new("/tmp/idxd-sys-missing-wq-for-test");

    let err = match WqPortal::open(path) {
        Ok(_) => panic!("missing WQ path should surface the OS open error"),
        Err(err) => err,
    };

    assert_eq!(
        err.kind(),
        ErrorKind::NotFound,
        "idxd-sys should preserve the raw std::io::ErrorKind from OpenOptions::open"
    );
}

#[test]
fn enqcmd_submission_is_a_small_typed_raw_backpressure_signal() {
    assert_eq!(EnqcmdSubmission::Accepted, EnqcmdSubmission::Accepted);
    assert_eq!(EnqcmdSubmission::Rejected, EnqcmdSubmission::Rejected);
    assert_ne!(EnqcmdSubmission::Accepted, EnqcmdSubmission::Rejected);
    assert_eq!(format!("{:?}", EnqcmdSubmission::Accepted), "Accepted");
    assert_eq!(format!("{:?}", EnqcmdSubmission::Rejected), "Rejected");
}

#[test]
fn root_public_raw_boundary_surface_remains_importable() {
    let _open: fn(&Path) -> std::io::Result<WqPortal> = WqPortal::open;
    let _poll: fn(&DsaCompletionRecord) -> u8 = poll_completion;
    let _reset: fn(&mut DsaCompletionRecord) = reset_completion;
    let _drain: fn(&[DsaCompletionRecord]) = drain_completions;
    let _touch_fault_page: fn(&DsaCompletionRecord) = touch_fault_page;
    let _rdtscp: fn() -> (u64, u32) = rdtscp;
    let _lfence: fn() = lfence;
    let _tsc_frequency_hz: fn() -> u64 = idxd_sys::tsc_frequency_hz;
    let _cycles_to_ns: fn(u64, u64) -> u64 = cycles_to_ns;
    let _pin_to_core: fn(usize) -> std::io::Result<usize> = pin_to_core;
    let _current_core: fn() -> usize = current_core;
    let _cpu_numa_node: fn(usize) -> Option<usize> = cpu_numa_node;
    let _device_numa_node: fn(&Path) -> Option<i32> = device_numa_node;
    let _flush_range: fn(*const u8, usize) = flush_range;

    assert_eq!(cycles_to_ns(3_000, 3_000_000_000), 1_000);
}
