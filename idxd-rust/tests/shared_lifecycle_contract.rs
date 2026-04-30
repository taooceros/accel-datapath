use std::fs;
use std::path::PathBuf;

use idxd_rust::{
    AsyncDsaHandle, AsyncDsaSession, DsaConfig, DsaSession, MemmoveError, MemmoveValidationReport,
};

fn crate_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn read_crate_file(relative: &str) -> String {
    fs::read_to_string(crate_path(relative))
        .unwrap_or_else(|error| panic!("failed to read tracked crate file {relative}: {error}"))
}

fn maybe_read_crate_file(relative: &str) -> Option<String> {
    fs::read_to_string(crate_path(relative)).ok()
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
fn operation_modules_do_not_branch_on_work_queue_mode() {
    let operation_modules = [
        (
            "src/direct_memmove.rs",
            read_crate_file("src/direct_memmove.rs"),
        ),
        (
            "src/iax_crc64.rs",
            maybe_read_crate_file("src/iax_crc64.rs").unwrap_or_default(),
        ),
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
    let _async_owner_type: Option<AsyncDsaSession> = None;
    let _async_handle_type: Option<AsyncDsaHandle> = None;
    let _memmove: fn(
        &DsaSession,
        &mut [u8],
        &[u8],
    ) -> Result<MemmoveValidationReport, MemmoveError> = DsaSession::memmove;
}
