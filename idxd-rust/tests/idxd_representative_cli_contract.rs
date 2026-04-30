use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

const SCHEMA_FIELDS: &[&str] = &[
    "ok",
    "operation",
    "accelerator",
    "device_path",
    "requested_bytes",
    "page_fault_retries",
    "final_status",
    "phase",
    "error_kind",
    "completion_error_code",
    "invalid_flags",
    "fault_addr",
    "crc64",
    "expected_crc64",
    "crc64_verified",
    "message",
];

fn live_idxd_op_bin() -> &'static str {
    env!("CARGO_BIN_EXE_live_idxd_op")
}

fn crate_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn unique_temp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("idxd-rust-representative-{name}-{nanos}"))
}

fn run(args: &[&str]) -> Output {
    Command::new(live_idxd_op_bin())
        .args(args)
        .output()
        .expect("binary should launch")
}

fn stdout_json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("stdout should be valid json")
}

fn assert_required_schema(value: &Value) {
    let object = value.as_object().expect("artifact should be a json object");
    for field in SCHEMA_FIELDS {
        assert!(
            object.contains_key(*field),
            "missing representative schema field `{field}` in {value}"
        );
    }
}

fn assert_no_payload_dump_fields(value: &Value) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                assert!(
                    !matches!(
                        key.as_str(),
                        "payload"
                            | "payload_bytes"
                            | "source"
                            | "source_bytes"
                            | "source_payload"
                            | "destination"
                            | "destination_bytes"
                            | "destination_payload"
                    ),
                    "unexpected payload dump field `{key}` in {value}"
                );
                assert_no_payload_dump_fields(child);
            }
        }
        Value::Array(items) => {
            for child in items {
                assert_no_payload_dump_fields(child);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

#[test]
fn proof_binary_uses_generic_sessions_not_legacy_or_raw_surfaces() {
    let source = fs::read_to_string(crate_path("src/bin/live_idxd_op.rs"))
        .expect("live_idxd_op source should be tracked");

    assert!(
        source.contains("IdxdSession::<Dsa>::open"),
        "DSA proof path must open through the generic session seam"
    );
    assert!(
        source.contains("IdxdSession::<Iax>::open"),
        "IAX proof path must open through the generic session seam"
    );
    assert!(
        source.contains("crc64_t10dif_field"),
        "IAX success path must compare hardware CRC against the reference helper"
    );
    for forbidden in [
        "DsaSession",
        "WqPortal",
        "submit_movdir64b",
        "submit_enqcmd",
        "hw-eval",
        "unsafe",
    ] {
        assert!(
            !source.contains(forbidden),
            "representative proof binary must not use forbidden surface {forbidden:?}"
        );
    }
}

#[test]
fn prints_help_without_touching_hardware() {
    let output = run(&["--help"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("live_idxd_op"));
    assert!(stdout.contains("--op dsa-memmove|iax-crc64"));
    assert!(stdout.contains("IdxdSession<Dsa>"));
    assert!(stdout.contains("IdxdSession<Iax>"));
}

#[test]
fn rejects_missing_or_unknown_operation_before_touching_hardware() {
    let missing = run(&["--device", "/dev/dsa/does-not-exist", "--bytes", "64"]);
    assert_eq!(missing.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&missing.stdout).is_empty());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("missing required `--op`"));

    let unknown = run(&[
        "--op",
        "noop",
        "--device",
        "/dev/dsa/does-not-exist",
        "--bytes",
        "64",
    ]);
    assert_eq!(unknown.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&unknown.stdout).is_empty());
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("unsupported operation `noop`"));
}

#[test]
fn rejects_malformed_cli_inputs_before_touching_hardware() {
    for args in [
        [
            "--op",
            "dsa-memmove",
            "--device",
            "/dev/dsa/nope",
            "--bytes",
            "abc",
        ]
        .as_slice(),
        [
            "--op",
            "dsa-memmove",
            "--device",
            "/dev/dsa/nope",
            "--bytes",
            "0",
        ]
        .as_slice(),
        ["--op", "iax-crc64", "--device", ""].as_slice(),
        [
            "--op",
            "iax-crc64",
            "--device",
            "/dev/iax/nope",
            "--format",
            "xml",
        ]
        .as_slice(),
    ] {
        let output = run(args);
        assert_eq!(output.status.code(), Some(2), "args={args:?}");
        assert!(String::from_utf8_lossy(&output.stdout).is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).is_empty());
    }
}

#[test]
fn rejects_bad_artifact_paths_before_touching_hardware() {
    let missing_value = run(&[
        "--op",
        "dsa-memmove",
        "--device",
        "/dev/dsa/does-not-exist",
        "--artifact",
    ]);
    assert_eq!(missing_value.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&missing_value.stdout).is_empty());
    assert!(
        String::from_utf8_lossy(&missing_value.stderr).contains("missing value for `--artifact`")
    );

    let temp_dir = unique_temp_path("artifact-dir");
    fs::create_dir_all(&temp_dir).expect("temp dir should be creatable");

    let directory_artifact = run(&[
        "--op",
        "iax-crc64",
        "--device",
        "/dev/iax/does-not-exist",
        "--artifact",
        temp_dir.to_str().expect("temp dir should be utf-8"),
    ]);
    assert_eq!(directory_artifact.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&directory_artifact.stdout).is_empty());
    assert!(
        String::from_utf8_lossy(&directory_artifact.stderr)
            .contains("expected a writable file path")
    );

    fs::remove_dir_all(&temp_dir).expect("temp dir cleanup should succeed");
}

#[test]
fn dsa_queue_open_failure_has_stable_no_payload_json_schema() {
    let output = run(&[
        "--op",
        "dsa-memmove",
        "--device",
        "/dev/dsa/does-not-exist",
        "--bytes",
        "64",
        "--format",
        "json",
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());

    let artifact = stdout_json(&output);
    assert_required_schema(&artifact);
    assert_eq!(artifact["ok"], false);
    assert_eq!(artifact["operation"], "dsa-memmove");
    assert_eq!(artifact["accelerator"], "dsa");
    assert_eq!(artifact["device_path"], "/dev/dsa/does-not-exist");
    assert_eq!(artifact["requested_bytes"], 64);
    assert!(artifact["page_fault_retries"].is_null());
    assert!(artifact["final_status"].is_null());
    assert_eq!(artifact["phase"], "queue_open");
    assert_eq!(artifact["error_kind"], "queue_open");
    assert!(artifact["completion_error_code"].is_null());
    assert!(artifact["invalid_flags"].is_null());
    assert!(artifact["fault_addr"].is_null());
    assert!(artifact["crc64"].is_null());
    assert!(artifact["expected_crc64"].is_null());
    assert!(artifact["crc64_verified"].is_null());
    assert_no_payload_dump_fields(&artifact);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"operation\":\"dsa-memmove\""));
    assert!(stdout.contains("\"accelerator\":\"dsa\""));
    assert!(stdout.contains("\"phase\":\"queue_open\""));
    assert!(stdout.contains("\"error_kind\":\"queue_open\""));
}

#[test]
fn iax_queue_open_failure_includes_crc_fields_without_hardware() {
    let output = run(&[
        "--op",
        "iax-crc64",
        "--device",
        "/dev/iax/does-not-exist",
        "--bytes",
        "64",
        "--format",
        "json",
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());

    let artifact = stdout_json(&output);
    assert_required_schema(&artifact);
    assert_eq!(artifact["ok"], false);
    assert_eq!(artifact["operation"], "iax-crc64");
    assert_eq!(artifact["accelerator"], "iax");
    assert_eq!(artifact["device_path"], "/dev/iax/does-not-exist");
    assert_eq!(artifact["requested_bytes"], 64);
    assert_eq!(artifact["phase"], "queue_open");
    assert_eq!(artifact["error_kind"], "queue_open");
    assert!(artifact["completion_error_code"].is_null());
    assert!(artifact["invalid_flags"].is_null());
    assert!(artifact["fault_addr"].is_null());
    assert!(artifact["crc64"].is_null());
    assert!(artifact["expected_crc64"].is_null());
    assert!(artifact["crc64_verified"].is_null());
    assert_no_payload_dump_fields(&artifact);
}

#[test]
fn text_output_uses_the_same_representative_schema_fields() {
    let output = run(&[
        "--op",
        "iax-crc64",
        "--device",
        "/dev/iax/does-not-exist",
        "--bytes",
        "64",
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());

    let stdout = String::from_utf8_lossy(&output.stdout);
    for field in SCHEMA_FIELDS {
        assert!(
            stdout.contains(&format!("{field}=")),
            "text output missing field `{field}`: {stdout}"
        );
    }
    assert!(stdout.contains("operation=iax-crc64"));
    assert!(stdout.contains("accelerator=iax"));
    assert!(stdout.contains("crc64=null"));
    assert!(stdout.contains("expected_crc64=null"));
    assert!(stdout.contains("crc64_verified=null"));
}

#[test]
fn writes_artifact_matching_stdout_exactly_for_failure_schema() {
    let artifact_path = unique_temp_path("artifact.json");

    let output = run(&[
        "--op",
        "dsa-memmove",
        "--device",
        "/dev/dsa/does-not-exist",
        "--bytes",
        "4096",
        "--format",
        "json",
        "--artifact",
        artifact_path
            .to_str()
            .expect("artifact path should be valid utf-8"),
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let artifact = fs::read_to_string(&artifact_path).expect("artifact should be written");
    assert_eq!(artifact, stdout);

    let parsed: Value = serde_json::from_str(&artifact).expect("artifact should parse as json");
    assert_required_schema(&parsed);
    assert_eq!(parsed["operation"], "dsa-memmove");
    assert_eq!(parsed["requested_bytes"], 4096);
    assert_eq!(parsed["phase"], "queue_open");
    assert_no_payload_dump_fields(&parsed);

    fs::remove_file(&artifact_path).expect("artifact cleanup should succeed");
}
