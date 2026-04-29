use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

fn live_memmove_bin() -> &'static str {
    env!("CARGO_BIN_EXE_live_memmove")
}

fn unique_temp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("idxd-rust-{name}-{nanos}"))
}

fn stdout_json(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("stdout should be valid json")
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
fn rejects_invalid_byte_string_before_touching_hardware() {
    let output = Command::new(live_memmove_bin())
        .args(["--bytes", "abc"])
        .output()
        .expect("binary should launch");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stdout).is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("invalid value `abc` for `--bytes`; expected a positive integer")
    );
}

#[test]
fn rejects_zero_bytes_before_touching_hardware() {
    let output = Command::new(live_memmove_bin())
        .args(["--bytes", "0"])
        .output()
        .expect("binary should launch");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stdout).is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("invalid memmove length 0; expected 1..=")
    );
}

#[test]
fn rejects_empty_device_path_before_touching_hardware() {
    let output = Command::new(live_memmove_bin())
        .args(["--device", ""])
        .output()
        .expect("binary should launch");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stdout).is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("device path must not be empty"));
}

#[test]
fn rejects_missing_artifact_value_before_touching_hardware() {
    let output = Command::new(live_memmove_bin())
        .args(["--artifact"])
        .output()
        .expect("binary should launch");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stdout).is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing value for `--artifact`"));
}

#[test]
fn rejects_directory_artifact_path_before_touching_hardware() {
    let temp_dir = unique_temp_path("artifact-dir");
    fs::create_dir_all(&temp_dir).expect("temp dir should be creatable");

    let output = Command::new(live_memmove_bin())
        .args([
            "--artifact",
            temp_dir.to_str().expect("temp dir should be utf-8"),
        ])
        .output()
        .expect("binary should launch");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stdout).is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("expected a writable file path"));

    fs::remove_dir_all(&temp_dir).expect("temp dir cleanup should succeed");
}

#[test]
fn reports_queue_open_failure_as_stable_json_schema() {
    let output = Command::new(live_memmove_bin())
        .args([
            "--device",
            "/dev/dsa/does-not-exist",
            "--bytes",
            "64",
            "--format",
            "json",
        ])
        .output()
        .expect("binary should launch");

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());

    let artifact = stdout_json(&output);
    assert_eq!(artifact["ok"], false);
    assert_eq!(artifact["device_path"], "/dev/dsa/does-not-exist");
    assert_eq!(artifact["requested_bytes"], 64);
    assert!(artifact["page_fault_retries"].is_null());
    assert!(artifact["final_status"].is_null());
    assert_eq!(artifact["phase"], "queue_open");
    assert_eq!(artifact["error_kind"], "queue_open");
    assert_no_payload_dump_fields(&artifact);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"ok\":false"));
    assert!(stdout.contains("\"device_path\":\"/dev/dsa/does-not-exist\""));
    assert!(stdout.contains("\"requested_bytes\":64"));
    assert!(stdout.contains("\"page_fault_retries\":null"));
    assert!(stdout.contains("\"final_status\":null"));
    assert!(stdout.contains("\"phase\":\"queue_open\""));
    assert!(stdout.contains("\"error_kind\":\"queue_open\""));
    assert!(
        stdout.contains("failed to open DSA work queue /dev/dsa/does-not-exist during queue_open")
    );
}

#[test]
fn supports_page_sized_request_and_writes_matching_artifact() {
    let artifact_path = unique_temp_path("artifact.json");

    let output = Command::new(live_memmove_bin())
        .args([
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
        ])
        .output()
        .expect("binary should launch");

    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let artifact = fs::read_to_string(&artifact_path).expect("artifact should be written");
    assert_eq!(artifact, stdout);
    let parsed: Value = serde_json::from_str(&artifact).expect("artifact should parse as json");
    assert_eq!(parsed["requested_bytes"], 4096);
    assert_eq!(parsed["phase"], "queue_open");
    assert_no_payload_dump_fields(&parsed);

    fs::remove_file(&artifact_path).expect("artifact cleanup should succeed");
}
