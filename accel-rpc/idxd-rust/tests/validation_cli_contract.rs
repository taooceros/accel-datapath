use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

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

#[test]
fn rejects_invalid_byte_string_before_touching_hardware() {
    let output = Command::new(live_memmove_bin())
        .args(["--bytes", "abc"])
        .output()
        .expect("binary should launch");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stdout).is_empty());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("invalid value `abc` for `--bytes`; expected a positive integer"));
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
    assert!(artifact.contains("\"requested_bytes\":4096"));
    assert!(artifact.contains("\"phase\":\"queue_open\""));

    fs::remove_file(&artifact_path).expect("artifact cleanup should succeed");
}
