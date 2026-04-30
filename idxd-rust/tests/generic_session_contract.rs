use std::any::type_name;
use std::error::Error as StdError;
use std::path::Path;

use idxd_rust::{Accelerator, Dsa, Iaa, Iax, IdxdSession, IdxdSessionConfig, IdxdSessionError};

fn assert_display_excludes_payload_markers(message: &str) {
    for forbidden in [
        "[1, 2, 3, 4]",
        "source_bytes",
        "destination_bytes",
        "payload",
        "secret-token",
    ] {
        assert!(
            !message.contains(forbidden),
            "display leaked forbidden payload marker {forbidden:?}: {message}"
        );
    }
}

fn assert_accelerator<A: Accelerator>() {}

#[test]
fn public_imports_compile_and_marker_defaults_are_host_free() {
    assert_accelerator::<Dsa>();
    assert_accelerator::<Iax>();
    assert_accelerator::<Iaa>();

    let dsa = IdxdSessionConfig::<Dsa>::default();
    assert_eq!(dsa.accelerator_name(), "dsa");
    assert_eq!(dsa.device_path(), Path::new("/dev/dsa/wq0.0"));
    assert_eq!(
        dsa.device_path(),
        Path::new(<Dsa as Accelerator>::DEFAULT_DEVICE_PATH)
    );

    let iax = IdxdSessionConfig::<Iax>::default();
    assert_eq!(iax.accelerator_name(), "iax");
    assert_eq!(iax.device_path(), Path::new("/dev/iax/wq1.0"));
    assert_eq!(
        iax.device_path(),
        Path::new(<Iax as Accelerator>::DEFAULT_DEVICE_PATH)
    );

    let iaa = IdxdSessionConfig::<Iaa>::default();
    assert_eq!(<Iaa as Accelerator>::NAME, <Iax as Accelerator>::NAME);
    assert_eq!(
        <Iaa as Accelerator>::DEFAULT_DEVICE_PATH,
        <Iax as Accelerator>::DEFAULT_DEVICE_PATH
    );
    assert_eq!(iaa, iax);

    let _session_type: Option<IdxdSession<Dsa>> = None;
    let _error_type: Option<IdxdSessionError> = None;
    let _is_dedicated: fn(&IdxdSession<Dsa>) -> bool = IdxdSession::<Dsa>::is_dedicated_wq;
    let _device_path: fn(&IdxdSession<Dsa>) -> &Path = IdxdSession::<Dsa>::device_path;
    let _accelerator_name: fn(&IdxdSession<Dsa>) -> &'static str =
        IdxdSession::<Dsa>::accelerator_name;
}

#[test]
fn iaa_alias_is_equivalent_to_iax_for_generic_type_identity() {
    assert_eq!(
        type_name::<IdxdSession<Iaa>>(),
        type_name::<IdxdSession<Iax>>()
    );
    assert_eq!(
        type_name::<IdxdSessionConfig<Iaa>>(),
        type_name::<IdxdSessionConfig<Iax>>()
    );
}

#[test]
fn dsa_and_iax_sessions_are_distinct_static_types() {
    let dsa = type_name::<IdxdSession<Dsa>>();
    let iax = type_name::<IdxdSession<Iax>>();

    assert_ne!(dsa, iax);
    assert!(dsa.contains("Dsa"), "unexpected DSA type name: {dsa}");
    assert!(iax.contains("Iax"), "unexpected IAX type name: {iax}");
}

#[test]
fn legacy_dsa_and_generic_session_imports_coexist_without_aliasing() {
    mod downstream_style {
        use std::any::type_name;

        use idxd_rust::{AsyncDsaHandle, AsyncDsaSession, Dsa, DsaConfig, DsaSession, IdxdSession};

        pub fn assert_public_surfaces_are_distinct() {
            let legacy_session = type_name::<DsaSession>();
            let generic_session = type_name::<IdxdSession<Dsa>>();

            assert_ne!(
                legacy_session, generic_session,
                "DsaSession must stay distinct from IdxdSession<Dsa>"
            );
            assert!(legacy_session.contains("DsaSession"));
            assert!(generic_session.contains("IdxdSession"));
            assert!(type_name::<DsaConfig>().contains("DsaConfig"));
            assert!(type_name::<AsyncDsaSession>().contains("AsyncDsaSession"));
            assert!(type_name::<AsyncDsaHandle>().contains("AsyncDsaHandle"));
        }
    }

    downstream_style::assert_public_surfaces_are_distinct();
}

#[test]
fn config_accepts_explicit_paths_without_opening_a_queue() {
    let config = IdxdSessionConfig::<Iax>::new("/dev/iax/wq3.0")
        .expect("non-empty IAX paths should validate before queue-open");

    assert_eq!(config.accelerator_name(), "iax");
    assert_eq!(config.device_path(), Path::new("/dev/iax/wq3.0"));
}

#[test]
fn empty_paths_reject_before_queue_open() {
    let err = IdxdSession::<Dsa>::open("")
        .err()
        .expect("empty DSA paths should fail validation before queue-open");

    assert!(matches!(err, IdxdSessionError::InvalidDevicePath { .. }));
    assert_eq!(err.kind(), "invalid_device_path");
    assert_eq!(err.accelerator_name(), "dsa");
    assert_eq!(err.device_path().and_then(Path::to_str), Some(""));
    assert!(
        StdError::source(&err).is_none(),
        "invalid paths should fail before queue-open and have no OS source"
    );
    assert_display_excludes_payload_markers(&err.to_string());

    let config_err = IdxdSessionConfig::<Iax>::new("")
        .err()
        .expect("empty IAX config paths should fail validation before queue-open");
    assert!(matches!(
        config_err,
        IdxdSessionError::InvalidDevicePath { .. }
    ));
    assert_eq!(config_err.kind(), "invalid_device_path");
    assert_eq!(config_err.accelerator_name(), "iax");
    assert!(StdError::source(&config_err).is_none());
    assert_display_excludes_payload_markers(&config_err.to_string());
}

fn assert_missing_queue_reports_context<A: Accelerator>(device_path: &'static str) {
    let err = IdxdSession::<A>::open(device_path)
        .err()
        .expect("missing work queues should surface queue-open diagnostics");

    assert!(matches!(err, IdxdSessionError::QueueOpen { .. }));
    assert_eq!(err.kind(), "queue_open");
    assert_eq!(err.accelerator_name(), A::NAME);
    assert_eq!(err.device_path().and_then(Path::to_str), Some(device_path));

    let source = StdError::source(&err).expect("queue-open errors should expose io source");
    assert!(source.is::<std::io::Error>());

    let message = err.to_string();
    assert!(
        message.contains(A::NAME),
        "message should include accelerator family {family:?}: {message}",
        family = A::NAME
    );
    assert!(
        message.contains(device_path),
        "message should include device path {device_path:?}: {message}"
    );
    assert_display_excludes_payload_markers(&message);
}

#[test]
fn missing_dsa_queue_reports_family_path_and_source() {
    assert_missing_queue_reports_context::<Dsa>("/dev/dsa/nonexistent-generic-session-test");
}

#[test]
fn missing_iax_queue_reports_family_path_and_source() {
    assert_missing_queue_reports_context::<Iax>("/dev/iax/nonexistent-generic-session-test");
}
