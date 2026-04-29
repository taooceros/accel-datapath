use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

const TEST_SCENARIO_ENV: &str = "IDXD_RUST_AWAIT_MEMMOVE_TEST_SCENARIO";

fn await_memmove_bin() -> &'static str {
    env!("CARGO_BIN_EXE_await_memmove")
}

fn unique_temp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("idxd-rust-async-{name}-{nanos}"))
}

fn run_with_optional_scenario(args: &[&str], scenario: Option<&str>) -> Output {
    let mut command = Command::new(await_memmove_bin());
    command.args(args);
    if let Some(scenario) = scenario {
        command.env(TEST_SCENARIO_ENV, scenario);
    }
    command.output().expect("binary should launch")
}

fn stdout_json(output: &Output) -> Value {
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
    let output = run_with_optional_scenario(&["--bytes", "abc"], None);

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stdout).is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("invalid value `abc` for `--bytes`; expected a positive integer")
    );
}

#[test]
fn rejects_zero_bytes_before_touching_hardware() {
    let output = run_with_optional_scenario(&["--bytes", "0"], None);

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stdout).is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("invalid memmove length 0; expected 1..=")
    );
}

#[test]
fn rejects_empty_device_path_before_touching_hardware() {
    let output = run_with_optional_scenario(&["--device", ""], None);

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stdout).is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("device path must not be empty"));
}

#[test]
fn rejects_missing_artifact_value_before_touching_hardware() {
    let output = run_with_optional_scenario(&["--artifact"], None);

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stdout).is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing value for `--artifact`"));
}

#[test]
fn rejects_directory_artifact_path_before_touching_hardware() {
    let temp_dir = unique_temp_path("artifact-dir");
    fs::create_dir_all(&temp_dir).expect("temp dir should be creatable");

    let output = run_with_optional_scenario(
        &[
            "--artifact",
            temp_dir.to_str().expect("temp dir should be utf-8"),
        ],
        None,
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stdout).is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("expected a writable file path"));

    fs::remove_dir_all(&temp_dir).expect("temp dir cleanup should succeed");
}

#[test]
fn reports_queue_open_failure_as_stable_async_json_schema() {
    let output = run_with_optional_scenario(
        &[
            "--device",
            "/dev/dsa/does-not-exist",
            "--bytes",
            "64",
            "--format",
            "json",
        ],
        None,
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());

    let artifact = stdout_json(&output);
    assert_eq!(artifact["ok"], false);
    assert_eq!(artifact["device_path"], "/dev/dsa/does-not-exist");
    assert_eq!(artifact["requested_bytes"], 64);
    assert!(artifact["page_fault_retries"].is_null());
    assert!(artifact["final_status"].is_null());
    assert_eq!(artifact["phase"], "queue_open");
    assert_eq!(artifact["error_kind"], "validation_failure");
    assert_eq!(artifact["validation_phase"], "queue_open");
    assert_eq!(artifact["validation_error_kind"], "queue_open");
    assert_no_payload_dump_fields(&artifact);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"ok\":false"));
    assert!(stdout.contains("\"device_path\":\"/dev/dsa/does-not-exist\""));
    assert!(stdout.contains("\"requested_bytes\":64"));
    assert!(stdout.contains("\"page_fault_retries\":null"));
    assert!(stdout.contains("\"final_status\":null"));
    assert!(stdout.contains("\"phase\":\"queue_open\""));
    assert!(stdout.contains("\"error_kind\":\"validation_failure\""));
    assert!(stdout.contains("\"lifecycle_failure_kind\":null"));
    assert!(stdout.contains("\"worker_failure_kind\":null"));
    assert!(stdout.contains("\"direct_failure_kind\":null"));
    assert!(stdout.contains("\"retry_budget\":null"));
    assert!(stdout.contains("\"retry_count\":null"));
    assert!(stdout.contains("\"completion_result\":null"));
    assert!(stdout.contains("\"completion_bytes_completed\":null"));
    assert!(stdout.contains("\"completion_fault_addr\":null"));
    assert!(stdout.contains("\"validation_phase\":\"queue_open\""));
    assert!(stdout.contains("\"validation_error_kind\":\"queue_open\""));
    assert!(
        stdout.contains("failed to open DSA work queue /dev/dsa/does-not-exist during queue_open")
    );
}

#[test]
fn reports_owner_shutdown_as_distinct_lifecycle_failure_in_json() {
    let output = run_with_optional_scenario(
        &[
            "--device",
            "/dev/dsa/test0.0",
            "--bytes",
            "64",
            "--format",
            "json",
        ],
        Some("owner_shutdown"),
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"ok\":false"));
    assert!(stdout.contains("\"phase\":\"async_lifecycle\""));
    assert!(stdout.contains("\"error_kind\":\"lifecycle_failure\""));
    assert!(stdout.contains("\"lifecycle_failure_kind\":\"owner_shutdown\""));
    assert!(stdout.contains("\"worker_failure_kind\":null"));
    assert!(stdout.contains("\"direct_failure_kind\":null"));
    assert!(stdout.contains("\"retry_budget\":null"));
    assert!(stdout.contains("\"retry_count\":null"));
    assert!(stdout.contains("\"completion_result\":null"));
    assert!(stdout.contains("\"completion_bytes_completed\":null"));
    assert!(stdout.contains("\"completion_fault_addr\":null"));
    assert!(stdout.contains("\"validation_phase\":null"));
    assert!(stdout.contains("\"validation_error_kind\":null"));
    assert!(stdout.contains("async memmove lifecycle failure: owner_shutdown"));
}

#[test]
fn reports_worker_failure_without_lifecycle_fields_in_json() {
    let output = run_with_optional_scenario(
        &[
            "--device",
            "/dev/dsa/test0.0",
            "--bytes",
            "64",
            "--format",
            "json",
        ],
        Some("worker_failure"),
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"ok\":false"));
    assert!(stdout.contains("\"phase\":\"async_worker\""));
    assert!(stdout.contains("\"error_kind\":\"worker_failure\""));
    assert!(stdout.contains("\"lifecycle_failure_kind\":null"));
    assert!(stdout.contains("\"worker_failure_kind\":\"response_channel_closed\""));
    assert!(stdout.contains("\"direct_failure_kind\":null"));
    assert!(stdout.contains("\"retry_budget\":null"));
    assert!(stdout.contains("\"retry_count\":null"));
    assert!(stdout.contains("\"completion_result\":null"));
    assert!(stdout.contains("\"completion_bytes_completed\":null"));
    assert!(stdout.contains("\"completion_fault_addr\":null"));
    assert!(stdout.contains("\"validation_phase\":null"));
    assert!(stdout.contains("\"validation_error_kind\":null"));
    assert!(stdout.contains("async memmove worker failure: response_channel_closed"));
}

#[test]
fn preserves_wrapped_validation_failure_metadata_in_json() {
    let output = run_with_optional_scenario(
        &[
            "--device",
            "/dev/dsa/test0.0",
            "--bytes",
            "64",
            "--format",
            "json",
        ],
        Some("completion_timeout"),
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"ok\":false"));
    assert!(stdout.contains("\"phase\":\"completion_poll\""));
    assert!(stdout.contains("\"error_kind\":\"validation_failure\""));
    assert!(stdout.contains("\"lifecycle_failure_kind\":null"));
    assert!(stdout.contains("\"worker_failure_kind\":null"));
    assert!(stdout.contains("\"direct_failure_kind\":null"));
    assert!(stdout.contains("\"retry_budget\":null"));
    assert!(stdout.contains("\"completion_result\":null"));
    assert!(stdout.contains("\"completion_bytes_completed\":null"));
    assert!(stdout.contains("\"completion_fault_addr\":null"));
    assert!(stdout.contains("\"validation_phase\":\"completion_poll\""));
    assert!(stdout.contains("\"validation_error_kind\":\"completion_timeout\""));
    assert!(stdout.contains("\"retry_budget\":null"));
    assert!(stdout.contains("\"retry_count\":2"));
    assert!(stdout.contains("\"page_fault_retries\":2"));
    assert!(stdout.contains("\"final_status\":null"));
}

#[test]
fn supports_minimal_valid_request_and_writes_matching_artifact() {
    let artifact_path = unique_temp_path("artifact.json");

    let output = run_with_optional_scenario(
        &[
            "--device",
            "/dev/dsa/test0.0",
            "--bytes",
            "1",
            "--format",
            "json",
            "--artifact",
            artifact_path
                .to_str()
                .expect("artifact path should be valid utf-8"),
        ],
        Some("success"),
    );

    assert_eq!(output.status.code(), Some(0));

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let artifact = fs::read_to_string(&artifact_path).expect("artifact should be written");
    assert_eq!(artifact, stdout);
    let parsed: Value = serde_json::from_str(&artifact).expect("artifact should parse as json");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["requested_bytes"], 1);
    assert_eq!(parsed["phase"], "completed");
    assert_eq!(parsed["error_kind"], Value::Null);
    assert_eq!(parsed["lifecycle_failure_kind"], Value::Null);
    assert_eq!(parsed["worker_failure_kind"], Value::Null);
    assert_eq!(parsed["direct_failure_kind"], Value::Null);
    assert_eq!(parsed["retry_budget"], 0);
    assert_eq!(parsed["retry_count"], 0);
    assert_eq!(parsed["completion_result"], Value::Null);
    assert_eq!(parsed["completion_bytes_completed"], Value::Null);
    assert_eq!(parsed["completion_fault_addr"], Value::Null);
    assert_eq!(parsed["validation_phase"], "completed");
    assert_eq!(parsed["validation_error_kind"], Value::Null);
    assert_eq!(parsed["final_status"], "0x01");
    assert_no_payload_dump_fields(&parsed);
    assert!(
        artifact.contains("verified 1 copied bytes via direct async memmove on /dev/dsa/test0.0")
    );

    fs::remove_file(&artifact_path).expect("artifact cleanup should succeed");
}
