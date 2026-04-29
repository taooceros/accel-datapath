use idxd_sys::{EnqcmdSubmission, WqPortal};
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
