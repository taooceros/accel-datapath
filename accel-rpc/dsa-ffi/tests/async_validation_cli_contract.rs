use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const TEST_SCENARIO_ENV: &str = "DSA_FFI_AWAIT_MEMMOVE_TEST_SCENARIO";

fn await_memmove_bin() -> &'static str {
    env!("CARGO_BIN_EXE_await_memmove")
}

fn unique_temp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("dsa-ffi-async-{name}-{nanos}"))
}

fn run_with_optional_scenario(args: &[&str], scenario: Option<&str>) -> Output {
    let mut command = Command::new(await_memmove_bin());
    command.args(args);
    if let Some(scenario) = scenario {
        command.env(TEST_SCENARIO_ENV, scenario);
    }
    command.output().expect("binary should launch")
}

#[test]
fn rejects_invalid_byte_string_before_touching_hardware() {
    let output = run_with_optional_scenario(&["--bytes", "abc"], None);

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stdout).is_empty());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("invalid value `abc` for `--bytes`; expected a positive integer"));
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
    assert!(stdout.contains("\"validation_phase\":\"completion_poll\""));
    assert!(stdout.contains("\"validation_error_kind\":\"completion_timeout\""));
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
    assert!(artifact.contains("\"ok\":true"));
    assert!(artifact.contains("\"requested_bytes\":1"));
    assert!(artifact.contains("\"phase\":\"completed\""));
    assert!(artifact.contains("\"error_kind\":null"));
    assert!(artifact.contains("\"lifecycle_failure_kind\":null"));
    assert!(artifact.contains("\"worker_failure_kind\":null"));
    assert!(artifact.contains("\"validation_phase\":\"completed\""));
    assert!(artifact.contains("\"validation_error_kind\":null"));
    assert!(artifact.contains("\"final_status\":\"0x01\""));
    assert!(artifact.contains("verified 1 copied bytes via async wrapper on /dev/dsa/test0.0"));

    fs::remove_file(&artifact_path).expect("artifact cleanup should succeed");
}
