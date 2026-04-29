use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const TEST_SCENARIO_ENV: &str = "IDXD_RUST_AWAIT_MEMMOVE_TEST_SCENARIO";

fn unique_temp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("idxd-rust-async-verifier-{name}-{nanos}"))
}

fn write_executable(path: &Path, content: &str) {
    fs::write(path, content).expect("script should be writable");
    let mut perms = fs::metadata(path)
        .expect("script metadata should exist")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("script should be executable");
}

fn async_verifier_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/verify_async_memmove.sh")
}

fn live_verifier_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/verify_live_memmove.sh")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("idxd-rust should live at the repository root")
        .to_path_buf()
}

fn fake_launcher_env(capability_ok: bool) -> (PathBuf, PathBuf, String) {
    let temp_root = unique_temp_path(if capability_ok {
        "launcher-ready"
    } else {
        "launcher-missing-cap"
    });
    let shim_dir = temp_root.join("bin");
    fs::create_dir_all(&shim_dir).expect("shim dir should be creatable");

    let launcher_path = temp_root.join("dsa_launcher");
    write_executable(
        &launcher_path,
        "#!/usr/bin/env bash\nset -euo pipefail\nexec \"$@\"\n",
    );

    write_executable(
        &shim_dir.join("devenv"),
        "#!/usr/bin/env bash
set -euo pipefail
if [[ ${1:-} != shell || ${2:-} != -- || ${3:-} != launch ]]; then
  echo \"unexpected devenv invocation: $*\" >&2
  exit 90
fi
shift 3
exec \"$@\"
",
    );

    let getcap_output = if capability_ok {
        format!("{} cap_sys_rawio+eip\n", launcher_path.display())
    } else {
        format!("{} cap_net_raw+eip\n", launcher_path.display())
    };
    write_executable(
        &shim_dir.join("getcap"),
        &format!(
            "#!/usr/bin/env bash
set -euo pipefail
printf '%s' {:?}
",
            getcap_output
        ),
    );

    let mut path_entries = vec![shim_dir.display().to_string()];
    if let Some(existing) = std::env::var_os("PATH") {
        path_entries.push(existing.to_string_lossy().into_owned());
    }
    let joined_path = path_entries.join(":");

    (temp_root, launcher_path, joined_path)
}

fn write_fake_binary(path: &Path, body: &str) {
    write_executable(
        path,
        &format!(
            r##"#!/usr/bin/env bash
set -euo pipefail
if [[ ${{1:-}} == --bytes && ${{2:-}} == abc ]]; then
  echo 'await_memmove: invalid value `abc` for `--bytes`; expected a positive integer' >&2
  exit 2
fi
artifact=
device=/dev/dsa/wq0.0
bytes=64
while [[ $# -gt 0 ]]; do
  case "$1" in
    --artifact)
      artifact=$2
      shift 2
      ;;
    --device)
      device=$2
      shift 2
      ;;
    --bytes)
      bytes=$2
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done
{body}
"##,
        ),
    );
}

#[test]
fn live_verifier_fails_preflight_when_binary_override_and_build_flags_conflict() {
    let (_temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let output_dir = unique_temp_path("live-contradictory-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let output = Command::new("bash")
        .arg(live_verifier_script())
        .env("PATH", path_override)
        .env("IDXD_RUST_VERIFY_DEVICE", "/dev/dsa/does-not-exist")
        .env("IDXD_RUST_VERIFY_LAUNCHER_PATH", &launcher_path)
        .env("IDXD_RUST_VERIFY_OUTPUT_DIR", &output_dir)
        .env(
            "IDXD_RUST_VERIFY_BINARY",
            env!("CARGO_BIN_EXE_live_memmove"),
        )
        .output()
        .expect("verifier should launch");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=preflight"));
    assert!(stderr.contains("launcher_status=contradictory_overrides"));
    assert!(stderr.contains("IDXD_RUST_VERIFY_BINARY requires IDXD_RUST_VERIFY_SKIP_BUILD=1"));
}

#[test]
fn live_verifier_reports_default_root_launcher_path_as_expected_failure() {
    let output_dir = unique_temp_path("live-default-launcher-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let output = Command::new("bash")
        .arg(live_verifier_script())
        .env("IDXD_RUST_VERIFY_SKIP_BUILD", "1")
        .env(
            "IDXD_RUST_VERIFY_BINARY",
            env!("CARGO_BIN_EXE_live_memmove"),
        )
        .env("IDXD_RUST_VERIFY_DEVICE", "/dev/dsa/does-not-exist")
        .env("IDXD_RUST_VERIFY_OUTPUT_DIR", &output_dir)
        .output()
        .expect("verifier should launch");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected_launcher = repo_root().join("tools/build/dsa_launcher");
    assert!(stdout.contains("phase=done"));
    assert!(stdout.contains("verdict=expected_failure"));
    assert!(stdout.contains("failure_phase=preflight"));
    assert!(
        stdout.contains("launcher_status=missing_launcher"),
        "stdout should report missing launcher, got: {stdout}"
    );
    assert!(stdout.contains(&format!("launcher_path={}", expected_launcher.display())));
    assert!(!stdout.contains(&format!(
        "launcher_path={}",
        repo_root().parent().unwrap().join("tools/build/dsa_launcher").display()
    )));
}

#[test]
fn async_verifier_reports_default_root_launcher_path_as_expected_failure() {
    let output_dir = unique_temp_path("async-default-launcher-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let output = Command::new("bash")
        .arg(async_verifier_script())
        .env("IDXD_RUST_VERIFY_SKIP_BUILD", "1")
        .env(
            "IDXD_RUST_VERIFY_BINARY",
            env!("CARGO_BIN_EXE_await_memmove"),
        )
        .env("IDXD_RUST_VERIFY_DEVICE", "/dev/dsa/does-not-exist")
        .env("IDXD_RUST_VERIFY_OUTPUT_DIR", &output_dir)
        .output()
        .expect("verifier should launch");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected_launcher = repo_root().join("tools/build/dsa_launcher");
    assert!(stdout.contains("phase=done"));
    assert!(stdout.contains("verdict=expected_failure"));
    assert!(stdout.contains("failure_phase=preflight"));
    assert!(
        stdout.contains("launcher_status=missing_launcher"),
        "stdout should report missing launcher, got: {stdout}"
    );
    assert!(stdout.contains(&format!("launcher_path={}", expected_launcher.display())));
    assert!(!stdout.contains(&format!(
        "launcher_path={}",
        repo_root().parent().unwrap().join("tools/build/dsa_launcher").display()
    )));
}

#[test]
fn verifier_fails_preflight_when_binary_override_and_build_flags_conflict() {
    let (_temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let output_dir = unique_temp_path("contradictory-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let output = Command::new("bash")
        .arg(async_verifier_script())
        .env("PATH", path_override)
        .env("IDXD_RUST_VERIFY_DEVICE", "/dev/dsa/does-not-exist")
        .env("IDXD_RUST_VERIFY_LAUNCHER_PATH", &launcher_path)
        .env("IDXD_RUST_VERIFY_OUTPUT_DIR", &output_dir)
        .env(
            "IDXD_RUST_VERIFY_BINARY",
            env!("CARGO_BIN_EXE_await_memmove"),
        )
        .output()
        .expect("verifier should launch");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=preflight"));
    assert!(stderr.contains("launcher_status=contradictory_overrides"));
    assert!(stderr.contains("IDXD_RUST_VERIFY_BINARY requires IDXD_RUST_VERIFY_SKIP_BUILD=1"));
}

#[test]
fn verifier_fails_preflight_when_launcher_capability_is_missing() {
    let (_temp_root, launcher_path, path_override) = fake_launcher_env(false);
    let output_dir = unique_temp_path("missing-cap-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let output = Command::new("bash")
        .arg(async_verifier_script())
        .env("PATH", path_override)
        .env("IDXD_RUST_VERIFY_SKIP_BUILD", "1")
        .env(
            "IDXD_RUST_VERIFY_BINARY",
            env!("CARGO_BIN_EXE_await_memmove"),
        )
        .env("IDXD_RUST_VERIFY_DEVICE", "/dev/dsa/does-not-exist")
        .env("IDXD_RUST_VERIFY_LAUNCHER_PATH", &launcher_path)
        .env("IDXD_RUST_VERIFY_OUTPUT_DIR", &output_dir)
        .output()
        .expect("verifier should launch");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"));
    assert!(stdout.contains("verdict=expected_failure"));
    assert!(stdout.contains("failure_phase=preflight"));
    assert!(stdout.contains("launcher_status=missing_capability"));
    assert!(stdout.contains(&format!("launcher_path={}", launcher_path.display())));
}

#[test]
fn verifier_preserves_queue_open_failure_and_async_fields() {
    let (_temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let output_dir = unique_temp_path("queue-open-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let output = Command::new("bash")
        .arg(async_verifier_script())
        .env("PATH", path_override)
        .env("IDXD_RUST_VERIFY_SKIP_BUILD", "1")
        .env(
            "IDXD_RUST_VERIFY_BINARY",
            env!("CARGO_BIN_EXE_await_memmove"),
        )
        .env("IDXD_RUST_VERIFY_DEVICE", "/dev/dsa/does-not-exist")
        .env("IDXD_RUST_VERIFY_LAUNCHER_PATH", &launcher_path)
        .env("IDXD_RUST_VERIFY_OUTPUT_DIR", &output_dir)
        .output()
        .expect("verifier should launch");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"));
    assert!(stdout.contains("verdict=expected_failure"));
    assert!(stdout.contains("failure_phase=runtime"));
    assert!(stdout.contains("launcher_status=ready"));
    assert!(stdout.contains("error_kind=validation_failure"));
    assert!(stdout.contains("async_lifecycle_failure_kind=null"));
    assert!(stdout.contains("async_worker_failure_kind=null"));
    assert!(stdout.contains("validation_phase=queue_open"));
    assert!(stdout.contains("validation_error_kind=queue_open"));
    assert!(stdout.contains(&format!(
        "artifact={}",
        output_dir.join("await_memmove.json").display()
    )));
    assert!(stdout.contains(&format!(
        "stdout={}",
        output_dir.join("await_memmove.stdout").display()
    )));
    assert!(stdout.contains(&format!(
        "stderr={}",
        output_dir.join("await_memmove.stderr").display()
    )));
}

#[test]
fn verifier_preserves_async_lifecycle_failure_kind() {
    let (_temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let output_dir = unique_temp_path("lifecycle-failure-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let output = Command::new("bash")
        .arg(async_verifier_script())
        .env("PATH", path_override)
        .env("IDXD_RUST_VERIFY_SKIP_BUILD", "1")
        .env(
            "IDXD_RUST_VERIFY_BINARY",
            env!("CARGO_BIN_EXE_await_memmove"),
        )
        .env("IDXD_RUST_VERIFY_DEVICE", "/dev/dsa/test0.0")
        .env("IDXD_RUST_VERIFY_LAUNCHER_PATH", &launcher_path)
        .env("IDXD_RUST_VERIFY_OUTPUT_DIR", &output_dir)
        .env(TEST_SCENARIO_ENV, "owner_shutdown")
        .output()
        .expect("verifier should launch");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"));
    assert!(stdout.contains("verdict=expected_failure"));
    assert!(stdout.contains("failure_phase=runtime"));
    assert!(stdout.contains("error_kind=lifecycle_failure"));
    assert!(stdout.contains("async_lifecycle_failure_kind=owner_shutdown"));
    assert!(stdout.contains("async_worker_failure_kind=null"));
    assert!(stdout.contains("validation_phase=null"));
    assert!(stdout.contains("validation_error_kind=null"));
}

#[test]
fn verifier_preserves_async_worker_failure_kind() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let output_dir = unique_temp_path("worker-failure-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let fake_binary = temp_root.join("fake_await_memmove");
    write_fake_binary(
        &fake_binary,
        r#"json=$(printf '{"ok":false,"device_path":"%s","requested_bytes":%s,"page_fault_retries":null,"final_status":null,"phase":"async_worker","error_kind":"worker_failure","lifecycle_failure_kind":null,"worker_failure_kind":"response_channel_closed","direct_failure_kind":null,"retry_budget":null,"retry_count":null,"completion_result":null,"completion_bytes_completed":null,"completion_fault_addr":null,"validation_phase":null,"validation_error_kind":null,"message":"async memmove worker failure: response_channel_closed"}' "$device" "$bytes")
printf '%s\n' "$json" | tee "$artifact"
exit 1"#,
    );

    let output = Command::new("bash")
        .arg(async_verifier_script())
        .env("PATH", path_override)
        .env("IDXD_RUST_VERIFY_SKIP_BUILD", "1")
        .env("IDXD_RUST_VERIFY_BINARY", &fake_binary)
        .env("IDXD_RUST_VERIFY_DEVICE", "/dev/dsa/does-not-exist")
        .env("IDXD_RUST_VERIFY_LAUNCHER_PATH", &launcher_path)
        .env("IDXD_RUST_VERIFY_OUTPUT_DIR", &output_dir)
        .output()
        .expect("verifier should launch");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"));
    assert!(stdout.contains("verdict=expected_failure"));
    assert!(stdout.contains("failure_phase=runtime"));
    assert!(stdout.contains("error_kind=worker_failure"));
    assert!(stdout.contains("async_lifecycle_failure_kind=null"));
    assert!(stdout.contains("async_worker_failure_kind=response_channel_closed"));
    assert!(stdout.contains("validation_phase=null"));
    assert!(stdout.contains("validation_error_kind=null"));
}

#[test]
fn verifier_preserves_async_direct_failure_metadata() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let output_dir = unique_temp_path("direct-failure-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let fake_binary = temp_root.join("fake_await_memmove");
    write_fake_binary(
        &fake_binary,
        r#"json=$(printf '{"ok":false,"device_path":"%s","requested_bytes":%s,"page_fault_retries":1,"final_status":"0x0d","phase":"async_direct","error_kind":"direct_failure","lifecycle_failure_kind":null,"worker_failure_kind":null,"direct_failure_kind":"backpressure_exceeded","retry_budget":1,"retry_count":1,"completion_result":0,"completion_bytes_completed":5,"completion_fault_addr":"0xfeed","validation_phase":null,"validation_error_kind":null,"message":"async direct memmove failure: backpressure_exceeded requested_bytes=%s retry_count=1 retry_budget=1 completion_status=0x0d completion_result=0 bytes_completed=5 fault_addr=0xfeed"}' "$device" "$bytes" "$bytes")
printf '%s\n' "$json" | tee "$artifact"
exit 1"#,
    );

    let output = Command::new("bash")
        .arg(async_verifier_script())
        .env("PATH", path_override)
        .env("IDXD_RUST_VERIFY_SKIP_BUILD", "1")
        .env("IDXD_RUST_VERIFY_BINARY", &fake_binary)
        .env("IDXD_RUST_VERIFY_DEVICE", "/dev/dsa/test0.0")
        .env("IDXD_RUST_VERIFY_LAUNCHER_PATH", &launcher_path)
        .env("IDXD_RUST_VERIFY_OUTPUT_DIR", &output_dir)
        .output()
        .expect("verifier should launch");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"));
    assert!(stdout.contains("verdict=expected_failure"));
    assert!(stdout.contains("failure_phase=runtime"));
    assert!(stdout.contains("error_kind=direct_failure"));
    assert!(stdout.contains("async_direct_failure_kind=backpressure_exceeded"));
    assert!(stdout.contains("retry_budget=1"));
    assert!(stdout.contains("retry_count=1"));
    assert!(stdout.contains("completion_result=0"));
    assert!(stdout.contains("completion_bytes_completed=5"));
    assert!(stdout.contains("completion_fault_addr=0xfeed"));
    assert!(!stdout.contains("secret-payload"));
}

#[test]
fn verifier_rejects_contradictory_artifact_fields() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let output_dir = unique_temp_path("malformed-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let fake_binary = temp_root.join("fake_await_memmove");
    write_fake_binary(
        &fake_binary,
        r#"json=$(printf '{"ok":true,"device_path":"%s","requested_bytes":%s,"page_fault_retries":0,"final_status":"0x01","phase":"completed","error_kind":null,"lifecycle_failure_kind":"owner_shutdown","worker_failure_kind":null,"direct_failure_kind":null,"retry_budget":0,"retry_count":0,"completion_result":null,"completion_bytes_completed":null,"completion_fault_addr":null,"validation_phase":"completed","validation_error_kind":null,"message":"verified %s copied bytes via direct async memmove on %s"}' "$device" "$bytes" "$bytes" "$device")
printf '%s\n' "$json" | tee "$artifact"
exit 0"#,
    );

    let output = Command::new("bash")
        .arg(async_verifier_script())
        .env("PATH", path_override)
        .env("IDXD_RUST_VERIFY_SKIP_BUILD", "1")
        .env("IDXD_RUST_VERIFY_BINARY", &fake_binary)
        .env("IDXD_RUST_VERIFY_DEVICE", "/dev/dsa/does-not-exist")
        .env("IDXD_RUST_VERIFY_LAUNCHER_PATH", &launcher_path)
        .env("IDXD_RUST_VERIFY_OUTPUT_DIR", &output_dir)
        .output()
        .expect("verifier should launch");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=artifact_validation"));
    assert!(stderr.contains("launcher_status=ready"));
    assert!(stderr.contains(&format!(
        "artifact={}",
        output_dir.join("await_memmove.json").display()
    )));
}

#[test]
fn verifier_rejects_missing_lifecycle_classification() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let output_dir = unique_temp_path("missing-lifecycle-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let fake_binary = temp_root.join("fake_await_memmove");
    write_fake_binary(
        &fake_binary,
        r#"json=$(printf '{"ok":false,"device_path":"%s","requested_bytes":%s,"page_fault_retries":null,"final_status":null,"phase":"async_lifecycle","error_kind":"lifecycle_failure","lifecycle_failure_kind":null,"worker_failure_kind":null,"direct_failure_kind":null,"retry_budget":null,"retry_count":null,"completion_result":null,"completion_bytes_completed":null,"completion_fault_addr":null,"validation_phase":null,"validation_error_kind":null,"message":"async memmove lifecycle failure: owner_shutdown"}' "$device" "$bytes")
printf '%s\n' "$json" | tee "$artifact"
exit 1"#,
    );

    let output = Command::new("bash")
        .arg(async_verifier_script())
        .env("PATH", path_override)
        .env("IDXD_RUST_VERIFY_SKIP_BUILD", "1")
        .env("IDXD_RUST_VERIFY_BINARY", &fake_binary)
        .env("IDXD_RUST_VERIFY_DEVICE", "/dev/dsa/test0.0")
        .env("IDXD_RUST_VERIFY_LAUNCHER_PATH", &launcher_path)
        .env("IDXD_RUST_VERIFY_OUTPUT_DIR", &output_dir)
        .output()
        .expect("verifier should launch");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=artifact_validation"));
    assert!(
        stderr.contains(
            "lifecycle-failure artifact lifecycle_failure_kind must be a non-empty string"
        )
    );
}

#[test]
fn verifier_rejects_stdout_artifact_divergence() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let output_dir = unique_temp_path("divergent-stdout-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let fake_binary = temp_root.join("fake_await_memmove");
    write_fake_binary(
        &fake_binary,
        r#"artifact_json=$(printf '{"ok":false,"device_path":"%s","requested_bytes":%s,"page_fault_retries":null,"final_status":null,"phase":"async_worker","error_kind":"worker_failure","lifecycle_failure_kind":null,"worker_failure_kind":"response_channel_closed","direct_failure_kind":null,"retry_budget":null,"retry_count":null,"completion_result":null,"completion_bytes_completed":null,"completion_fault_addr":null,"validation_phase":null,"validation_error_kind":null,"message":"async memmove worker failure: response_channel_closed"}' "$device" "$bytes")
stdout_json=$(printf '{"ok":false,"device_path":"%s","requested_bytes":%s,"page_fault_retries":null,"final_status":null,"phase":"async_worker","error_kind":"worker_failure","lifecycle_failure_kind":null,"worker_failure_kind":"worker_panicked","direct_failure_kind":null,"retry_budget":null,"retry_count":null,"completion_result":null,"completion_bytes_completed":null,"completion_fault_addr":null,"validation_phase":null,"validation_error_kind":null,"message":"async memmove worker failure: worker_panicked"}' "$device" "$bytes")
printf '%s\n' "$artifact_json" > "$artifact"
printf '%s\n' "$stdout_json"
exit 1"#,
    );

    let output = Command::new("bash")
        .arg(async_verifier_script())
        .env("PATH", path_override)
        .env("IDXD_RUST_VERIFY_SKIP_BUILD", "1")
        .env("IDXD_RUST_VERIFY_BINARY", &fake_binary)
        .env("IDXD_RUST_VERIFY_DEVICE", "/dev/dsa/test0.0")
        .env("IDXD_RUST_VERIFY_LAUNCHER_PATH", &launcher_path)
        .env("IDXD_RUST_VERIFY_OUTPUT_DIR", &output_dir)
        .output()
        .expect("verifier should launch");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=artifact_validation"));
    assert!(stderr.contains("stdout and artifact diverged"));
    assert!(stderr.contains(&format!(
        "stdout={}",
        output_dir.join("await_memmove.stdout").display()
    )));
}

#[test]
fn verifier_rejects_nested_payload_dump_fields() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let output_dir = unique_temp_path("nested-payload-dump-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let fake_binary = temp_root.join("fake_await_memmove");
    write_fake_binary(
        &fake_binary,
        r#"json=$(printf '{"ok":true,"device_path":"%s","requested_bytes":%s,"page_fault_retries":0,"final_status":"0x00","phase":"completed","error_kind":null,"lifecycle_failure_kind":null,"worker_failure_kind":null,"direct_failure_kind":null,"retry_budget":0,"retry_count":0,"completion_result":null,"completion_bytes_completed":null,"completion_fault_addr":null,"validation_phase":"completed","validation_error_kind":null,"message":"verified %s copied bytes via direct async memmove on %s","debug":{"destination_payload":[1,2,3]}}' "$device" "$bytes" "$bytes" "$device")
printf '%s\n' "$json" | tee "$artifact"
exit 0"#,
    );

    let output = Command::new("bash")
        .arg(async_verifier_script())
        .env("PATH", path_override)
        .env("IDXD_RUST_VERIFY_SKIP_BUILD", "1")
        .env("IDXD_RUST_VERIFY_BINARY", &fake_binary)
        .env("IDXD_RUST_VERIFY_DEVICE", "/dev/dsa/test0.0")
        .env("IDXD_RUST_VERIFY_LAUNCHER_PATH", &launcher_path)
        .env("IDXD_RUST_VERIFY_OUTPUT_DIR", &output_dir)
        .output()
        .expect("verifier should launch");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=artifact_validation"));
    assert!(stderr.contains("forbidden payload dump field report.debug.destination_payload"));
}
