use idxd_sys::{WqPortal, idxd, idxd_uapi};
use std::io::ErrorKind;
use std::path::Path;

#[test]
fn missing_wq_open_preserves_std_io_error_kind() {
    let path = Path::new("/tmp/idxd-sys-missing-wq-for-test");

    let err = match WqPortal::open(path) {
        Ok(_) => panic!("missing WQ path should surface the OS open error"),
        Err(err) => err,
    };

    assert_eq!(err.kind(), ErrorKind::NotFound);
}

#[test]
fn root_public_surface_is_bindgen_uapi_and_raw_portal_only() {
    let _open: fn(&Path) -> std::io::Result<WqPortal> = WqPortal::open;
    let _raw_movdir64b: unsafe fn(&WqPortal, *const u8) = WqPortal::submit_movdir64b_desc64;
    let _raw_enqcmd: unsafe fn(&WqPortal, *const u8) -> bool = WqPortal::submit_enqcmd_desc64;

    assert_eq!(
        std::mem::size_of::<idxd::dsa_hw_desc>(),
        std::mem::size_of::<idxd_uapi::dsa_hw_desc>()
    );
}
