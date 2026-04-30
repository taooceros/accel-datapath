use std::fs;
use std::path::PathBuf;

use idxd_rust::{
    AsyncDsaHandle, AsyncDsaSession, Dsa, DsaConfig, DsaSession, Iaa, Iax, IaxCrc64Result,
    IdxdSession, MemmoveError, MemmoveValidationReport,
};

fn crate_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn read_crate_file(relative: &str) -> String {
    fs::read_to_string(crate_path(relative))
        .unwrap_or_else(|error| panic!("failed to read tracked crate file {relative}: {error}"))
}

#[test]
fn lifecycle_module_is_crate_private_static_dispatch() {
    let lib = read_crate_file("src/lib.rs");
    let lifecycle = read_crate_file("src/lifecycle.rs");

    assert!(
        lib.contains("mod lifecycle;"),
        "crate root should register the private lifecycle module"
    );
    assert!(
        lib.contains("mod iax_crc64;"),
        "crate root should register the private IAX crc64 operation module"
    );
    assert!(
        !lib.contains("pub mod lifecycle"),
        "lifecycle must stay crate-private, not a public module"
    );
    assert!(
        lifecycle.contains("pub(crate) fn run_blocking_operation"),
        "shared blocking lifecycle entrypoint should be crate-private"
    );
    assert!(
        lifecycle.contains("pub(crate) trait BlockingOperation"),
        "operation contract should be crate-private"
    );
    assert!(
        lifecycle.contains("pub(crate) enum BlockingOperationDecision"),
        "retry/success decision should be crate-private"
    );

    for forbidden in [
        "dyn BlockingOperation",
        "Box<dyn",
        "pub trait BlockingOperation",
        "pub fn run_blocking_operation",
        "pub enum BlockingOperationDecision",
    ] {
        assert!(
            !lifecycle.contains(forbidden),
            "blocking lifecycle should use crate-private static dispatch, found {forbidden:?}"
        );
    }
}

#[test]
fn legacy_dsa_memmove_delegates_to_shared_lifecycle() {
    let lib = read_crate_file("src/lib.rs");
    let direct_memmove = read_crate_file("src/direct_memmove.rs");

    assert!(
        lib.contains("run_direct_memmove(&self.portal"),
        "DsaSession::memmove_inner should delegate to the DSA helper"
    );
    for forbidden in [
        "DirectMemmoveState::new",
        "poll_completion(",
        "touch_fault_page(",
        "portal.submit(",
    ] {
        assert!(
            !lib.contains(forbidden),
            "DsaSession::memmove_inner should not grow a fresh inline submit/poll loop: {forbidden:?}"
        );
    }

    assert!(
        direct_memmove.contains("impl BlockingOperation for DirectMemmoveOperation<'_>"),
        "direct memmove should implement the private lifecycle contract"
    );
    assert!(
        direct_memmove.contains("run_blocking_operation(portal"),
        "direct memmove helper should use the shared lifecycle loop"
    );
}

#[test]
fn generic_representative_operations_delegate_to_shared_lifecycle() {
    let session = read_crate_file("src/session.rs");
    let iax_crc64 = read_crate_file("src/iax_crc64.rs");

    let dsa_impl = session
        .split("impl IdxdSession<Dsa>")
        .nth(1)
        .and_then(|tail| tail.split("impl IdxdSession<Iax>").next())
        .expect("session should have a DSA-specific generic impl");
    assert!(
        dsa_impl.contains("run_direct_memmove(") && dsa_impl.contains("&self.portal"),
        "IdxdSession<Dsa>::memmove should reuse the DSA helper and the already-open portal"
    );
    assert!(
        !dsa_impl.contains("WqPortal::open"),
        "IdxdSession<Dsa>::memmove must not re-open the queue"
    );

    assert!(
        session.contains("impl IdxdSession<Iax>"),
        "session should expose an IAX-specific generic impl"
    );
    assert!(
        session.contains("run_iax_crc64(&self.portal"),
        "IdxdSession<Iax>::crc64 should reuse the already-open portal"
    );
    assert!(
        iax_crc64.contains("impl BlockingOperation for IaxCrc64State<'_>"),
        "IAX crc64 should implement the private lifecycle contract"
    );
    assert!(
        iax_crc64.contains("run_blocking_operation(portal"),
        "IAX crc64 helper should use the shared lifecycle loop"
    );
}

#[test]
fn operation_modules_do_not_branch_on_work_queue_mode() {
    let operation_modules = [
        (
            "src/direct_memmove.rs",
            read_crate_file("src/direct_memmove.rs"),
        ),
        ("src/iax_crc64.rs", read_crate_file("src/iax_crc64.rs")),
    ];

    for (path, source) in operation_modules {
        for forbidden in ["is_dedicated", "submit_movdir64b", "submit_enqcmd"] {
            assert!(
                !source.contains(forbidden),
                "operation module {path} must not branch on WQ mode or select portal primitives directly: found {forbidden:?}"
            );
        }
    }
}

#[test]
fn legacy_dsa_and_async_compatibility_surfaces_still_import() {
    let _config_type: Option<DsaConfig> = None;
    let _session_type: Option<DsaSession> = None;
    let _generic_dsa_type: Option<IdxdSession<Dsa>> = None;
    let _generic_iax_type: Option<IdxdSession<Iax>> = None;
    let _async_owner_type: Option<AsyncDsaSession> = None;
    let _async_handle_type: Option<AsyncDsaHandle> = None;
    let _memmove: fn(
        &DsaSession,
        &mut [u8],
        &[u8],
    ) -> Result<MemmoveValidationReport, MemmoveError> = DsaSession::memmove;
    let _generic_memmove: fn(
        &IdxdSession<Dsa>,
        &mut [u8],
        &[u8],
    ) -> Result<MemmoveValidationReport, MemmoveError> = IdxdSession::<Dsa>::memmove;
    let _iax_crc64: fn(&IdxdSession<Iax>, &[u8]) -> IaxCrc64Result = IdxdSession::<Iax>::crc64;
    let _iaa_crc64: fn(&IdxdSession<Iaa>, &[u8]) -> IaxCrc64Result = IdxdSession::<Iaa>::crc64;
}
