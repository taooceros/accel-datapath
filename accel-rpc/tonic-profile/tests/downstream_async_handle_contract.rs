use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

const TEST_SCENARIO_ENV: &str = "IDXD_TONIC_ASYNC_HANDLE_TEST_SCENARIO";
const DEVICE_PATH: &str = "/dev/dsa/test0.0";

fn downstream_async_handle_bin() -> &'static str {
    env!("CARGO_BIN_EXE_downstream_async_handle")
}

fn verifier_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/verify_downstream_async_handle.sh")
}

fn unique_temp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "tonic-profile-downstream-async-handle-{name}-{}-{nanos}.json",
        std::process::id()
    ))
}

fn run_with_scenario(args: &[&str], scenario: Option<&str>) -> Output {
    let mut command = Command::new(downstream_async_handle_bin());
    command.args(args);
    if let Some(scenario) = scenario {
        command.env(TEST_SCENARIO_ENV, scenario);
    } else {
        command.env_remove(TEST_SCENARIO_ENV);
    }
    command
        .output()
        .expect("downstream_async_handle should launch")
}

fn write_executable(path: &Path, content: &str) {
    fs::write(path, content).expect("script should be writable");
    let mut perms = fs::metadata(path)
        .expect("script metadata should exist")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("script should be executable");
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

fn write_fake_downstream_binary(path: &Path, body: &str) {
    write_executable(
        path,
        &format!(
            r##"#!/usr/bin/env bash
set -euo pipefail
if [[ ${{1:-}} == --bytes && ${{2:-}} == abc ]]; then
  echo 'downstream_async_handle: invalid value `abc` for `--bytes`; expected a positive integer' >&2
  exit 2
fi
artifact=
device=/dev/dsa/test0.0
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

fn parse_stdout_json(output: &Output) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(stdout.trim()).unwrap_or_else(|err| {
        panic!(
            "stdout should be valid JSON: {err}\nstdout:\n{stdout}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn parse_artifact_json(path: &PathBuf, output: &Output) -> Value {
    let raw = fs::read_to_string(path).unwrap_or_else(|err| {
        panic!(
            "artifact should be readable at {}: {err}\nstdout:\n{}\nstderr:\n{}",
            path.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    serde_json::from_str(raw.trim()).unwrap_or_else(|err| {
        panic!(
            "artifact should be valid JSON: {err}\nartifact:\n{raw}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn assert_status(output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "unexpected status\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_common_metadata(json: &Value) {
    assert_eq!(json["proof_seam"], "downstream_async_handle");
    assert_eq!(json["consumer_package"], "tonic-profile");
    assert_eq!(json["binding_package"], "idxd-rust");
    assert_eq!(json["composition"], "tokio_join");
    assert_eq!(json["operation_count"], 2);
    assert_eq!(json["device_path"], DEVICE_PATH);
}

fn assert_success_schema(json: &Value) {
    assert_common_metadata(json);
    assert_eq!(json["ok"], true);
    assert_eq!(json["requested_bytes"], 16);
    assert_eq!(json["phase"], "completed");
    assert!(json["error_kind"].is_null());
    assert!(json["lifecycle_failure_kind"].is_null());
    assert!(json["worker_failure_kind"].is_null());
    assert_eq!(json["validation_phase"], "completed");
    assert!(json["validation_error_kind"].is_null());
    assert!(json["message"]
        .as_str()
        .expect("message should be a string")
        .contains("verified 2 joined cloned-handle async memmoves"));
}

fn assert_no_payload_bytes(serialized: &str) {
    assert!(
        !serialized.contains("17,48") && !serialized.contains("[17"),
        "serialized proof must not include deterministic payload bytes\n{serialized}"
    );
    for forbidden_field in ["source_bytes", "destination_bytes", "payload", "bytes\":[]"] {
        assert!(
            !serialized.contains(forbidden_field),
            "serialized proof leaked forbidden payload field `{forbidden_field}`\n{serialized}"
        );
    }
}

#[test]
fn success_scenario_emits_matching_stdout_and_artifact_json_without_payload_bytes() {
    let artifact_path = unique_temp_path("success");
    let output = run_with_scenario(
        &[
            "--device",
            DEVICE_PATH,
            "--bytes",
            "16",
            "--format",
            "json",
            "--artifact",
            artifact_path
                .to_str()
                .expect("artifact path should be valid utf-8"),
        ],
        Some("success"),
    );

    assert_status(&output, 0);
    assert!(
        String::from_utf8_lossy(&output.stderr).is_empty(),
        "success scenario should not write stderr\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout_json = parse_stdout_json(&output);
    let artifact_json = parse_artifact_json(&artifact_path, &output);
    assert_eq!(artifact_json, stdout_json);
    assert_success_schema(&stdout_json);
    assert_no_payload_bytes(&String::from_utf8_lossy(&output.stdout));
    assert_no_payload_bytes(&fs::read_to_string(&artifact_path).expect("artifact should exist"));

    fs::remove_file(&artifact_path).expect("artifact cleanup should succeed");
}

#[test]
fn owner_shutdown_scenario_preserves_lifecycle_classification() {
    let output = run_with_scenario(
        &["--device", DEVICE_PATH, "--bytes", "16", "--format", "json"],
        Some("owner_shutdown"),
    );

    assert_status(&output, 1);
    let json = parse_stdout_json(&output);
    assert_common_metadata(&json);
    assert_eq!(json["ok"], false);
    assert_eq!(json["phase"], "async_lifecycle");
    assert_eq!(json["error_kind"], "lifecycle_failure");
    assert_eq!(json["lifecycle_failure_kind"], "owner_shutdown");
    assert!(json["worker_failure_kind"].is_null());
    assert!(json["validation_phase"].is_null());
    assert!(json["validation_error_kind"].is_null());
}

#[test]
fn worker_failure_scenario_preserves_worker_classification() {
    let output = run_with_scenario(
        &["--device", DEVICE_PATH, "--bytes", "16", "--format", "json"],
        Some("worker_failure"),
    );

    assert_status(&output, 1);
    let json = parse_stdout_json(&output);
    assert_common_metadata(&json);
    assert_eq!(json["ok"], false);
    assert_eq!(json["phase"], "async_worker");
    assert_eq!(json["error_kind"], "worker_failure");
    assert!(json["lifecycle_failure_kind"].is_null());
    assert_eq!(json["worker_failure_kind"], "worker_panicked");
    assert!(json["validation_phase"].is_null());
    assert!(json["validation_error_kind"].is_null());
}

#[test]
fn completion_timeout_scenario_preserves_validation_classification() {
    let output = run_with_scenario(
        &["--device", DEVICE_PATH, "--bytes", "16", "--format", "json"],
        Some("completion_timeout"),
    );

    assert_status(&output, 1);
    let json = parse_stdout_json(&output);
    assert_common_metadata(&json);
    assert_eq!(json["ok"], false);
    assert_eq!(json["phase"], "completion_poll");
    assert_eq!(json["error_kind"], "validation_failure");
    assert!(json["lifecycle_failure_kind"].is_null());
    assert!(json["worker_failure_kind"].is_null());
    assert_eq!(json["validation_phase"], "completion_poll");
    assert_eq!(json["validation_error_kind"], "completion_timeout");
}

#[test]
fn invalid_destination_scenario_preserves_validation_classification() {
    let output = run_with_scenario(
        &["--device", DEVICE_PATH, "--bytes", "16", "--format", "json"],
        Some("invalid_destination_len"),
    );

    assert_status(&output, 1);
    let json = parse_stdout_json(&output);
    assert_common_metadata(&json);
    assert_eq!(json["ok"], false);
    assert_eq!(json["phase"], "argument_validation");
    assert_eq!(json["error_kind"], "validation_failure");
    assert_eq!(json["validation_phase"], "argument_validation");
    assert_eq!(json["validation_error_kind"], "destination_too_small");
}

#[test]
fn invalid_cli_args_and_bad_scenario_fail_cleanly() {
    let invalid_bytes = run_with_scenario(&["--bytes", "abc"], Some("success"));
    assert_status(&invalid_bytes, 2);
    assert!(String::from_utf8_lossy(&invalid_bytes.stdout).is_empty());
    assert!(String::from_utf8_lossy(&invalid_bytes.stderr)
        .contains("invalid value `abc` for `--bytes`"));

    let zero_bytes = run_with_scenario(&["--bytes", "0"], Some("success"));
    assert_status(&zero_bytes, 2);
    assert!(String::from_utf8_lossy(&zero_bytes.stderr).contains("invalid memmove length 0"));

    let invalid_format = run_with_scenario(&["--format", "yaml"], Some("success"));
    assert_status(&invalid_format, 2);
    assert!(String::from_utf8_lossy(&invalid_format.stderr)
        .contains("unsupported output format `yaml`"));

    let bad_scenario = run_with_scenario(
        &["--device", DEVICE_PATH, "--bytes", "16", "--format", "json"],
        Some("not-a-scenario"),
    );
    assert_status(&bad_scenario, 1);
    let json = parse_stdout_json(&bad_scenario);
    assert_common_metadata(&json);
    assert_eq!(json["ok"], false);
    assert_eq!(json["error_kind"], "validation_failure");
    assert_eq!(json["validation_phase"], "argument_validation");
    assert_eq!(json["validation_error_kind"], "invalid_test_scenario");
}

#[test]
fn invalid_artifact_paths_are_rejected_before_scenario_execution() {
    let missing_parent = unique_temp_path("missing-parent").join("artifact.json");
    let missing_parent_output = run_with_scenario(
        &[
            "--artifact",
            missing_parent
                .to_str()
                .expect("artifact path should be valid utf-8"),
        ],
        Some("success"),
    );
    assert_status(&missing_parent_output, 2);
    assert!(String::from_utf8_lossy(&missing_parent_output.stdout).is_empty());
    assert!(String::from_utf8_lossy(&missing_parent_output.stderr)
        .contains("artifact parent directory"));

    let artifact_dir = unique_temp_path("artifact-dir");
    fs::create_dir_all(&artifact_dir).expect("temp artifact dir should be creatable");
    let directory_output = run_with_scenario(
        &[
            "--artifact",
            artifact_dir
                .to_str()
                .expect("artifact dir should be valid utf-8"),
        ],
        Some("success"),
    );
    assert_status(&directory_output, 2);
    assert!(String::from_utf8_lossy(&directory_output.stderr)
        .contains("is a directory; expected a writable file path"));
    fs::remove_dir_all(&artifact_dir).expect("artifact dir cleanup should succeed");
}

#[test]
fn verifier_reports_missing_launcher_capability_as_expected_failure() {
    let (_temp_root, launcher_path, path_override) = fake_launcher_env(false);
    let output_dir = unique_temp_path("verifier-missing-cap-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let output = Command::new("bash")
        .arg(verifier_script())
        .env("PATH", path_override)
        .env("TONIC_PROFILE_DOWNSTREAM_ASYNC_SKIP_BUILD", "1")
        .env(
            "TONIC_PROFILE_DOWNSTREAM_ASYNC_BINARY",
            downstream_async_handle_bin(),
        )
        .env("TONIC_PROFILE_DOWNSTREAM_ASYNC_DEVICE", DEVICE_PATH)
        .env(
            "TONIC_PROFILE_DOWNSTREAM_ASYNC_LAUNCHER_PATH",
            &launcher_path,
        )
        .env("TONIC_PROFILE_DOWNSTREAM_ASYNC_OUTPUT_DIR", &output_dir)
        .output()
        .expect("verifier should launch");

    assert_status(&output, 0);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"));
    assert!(stdout.contains("verdict=expected_failure"));
    assert!(stdout.contains("failure_phase=preflight"));
    assert!(stdout.contains("launcher_status=missing_capability"));
    assert!(stdout.contains(&format!("launcher_path={}", launcher_path.display())));
}

#[test]
fn verifier_preserves_queue_open_failure_with_downstream_metadata() {
    let (_temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let output_dir = unique_temp_path("verifier-queue-open-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let output = Command::new("bash")
        .arg(verifier_script())
        .env("PATH", path_override)
        .env("TONIC_PROFILE_DOWNSTREAM_ASYNC_SKIP_BUILD", "1")
        .env(
            "TONIC_PROFILE_DOWNSTREAM_ASYNC_BINARY",
            downstream_async_handle_bin(),
        )
        .env(
            "TONIC_PROFILE_DOWNSTREAM_ASYNC_DEVICE",
            "/dev/dsa/does-not-exist",
        )
        .env(
            "TONIC_PROFILE_DOWNSTREAM_ASYNC_LAUNCHER_PATH",
            &launcher_path,
        )
        .env("TONIC_PROFILE_DOWNSTREAM_ASYNC_OUTPUT_DIR", &output_dir)
        .output()
        .expect("verifier should launch");

    assert_status(&output, 0);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"));
    assert!(stdout.contains("verdict=expected_failure"));
    assert!(stdout.contains("failure_phase=runtime"));
    assert!(stdout.contains("proof_seam=downstream_async_handle"));
    assert!(stdout.contains("consumer_package=tonic-profile"));
    assert!(stdout.contains("binding_package=idxd-rust"));
    assert!(stdout.contains("composition=tokio_join"));
    assert!(stdout.contains("operation_count=2"));
    assert!(stdout.contains("error_kind=validation_failure"));
    assert!(stdout.contains("validation_phase=queue_open"));
    assert!(stdout.contains("validation_error_kind=queue_open"));
    assert!(stdout.contains(&format!(
        "artifact={}",
        output_dir.join("downstream_async_handle.json").display()
    )));
}

#[test]
fn verifier_rejects_wrong_downstream_proof_metadata() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let output_dir = unique_temp_path("verifier-wrong-metadata-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let fake_binary = temp_root.join("fake_downstream_async_handle");
    write_fake_downstream_binary(
        &fake_binary,
        r#"json=$(printf '{"ok":true,"proof_seam":"await_memmove","consumer_package":"idxd-rust","binding_package":"idxd-rust","composition":"tokio_join","operation_count":2,"device_path":"%s","requested_bytes":%s,"phase":"completed","error_kind":null,"lifecycle_failure_kind":null,"worker_failure_kind":null,"validation_phase":"completed","validation_error_kind":null,"message":"verified 2 joined cloned-handle async memmoves"}' "$device" "$bytes")
printf '%s\n' "$json" | tee "$artifact"
exit 0"#,
    );

    let output = Command::new("bash")
        .arg(verifier_script())
        .env("PATH", path_override)
        .env("TONIC_PROFILE_DOWNSTREAM_ASYNC_SKIP_BUILD", "1")
        .env("TONIC_PROFILE_DOWNSTREAM_ASYNC_BINARY", &fake_binary)
        .env("TONIC_PROFILE_DOWNSTREAM_ASYNC_DEVICE", DEVICE_PATH)
        .env(
            "TONIC_PROFILE_DOWNSTREAM_ASYNC_LAUNCHER_PATH",
            &launcher_path,
        )
        .env("TONIC_PROFILE_DOWNSTREAM_ASYNC_OUTPUT_DIR", &output_dir)
        .output()
        .expect("verifier should launch");

    assert_status(&output, 1);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=artifact_validation"));
    assert!(
        stderr.contains("artifact proof_seam='await_memmove' expected 'downstream_async_handle'")
    );
}

#[test]
fn verifier_rejects_malformed_downstream_artifact() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let output_dir = unique_temp_path("verifier-malformed-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let fake_binary = temp_root.join("fake_downstream_async_handle");
    write_fake_downstream_binary(
        &fake_binary,
        r#"printf '{"ok":' > "$artifact"
printf '{"ok":\n'
exit 1"#,
    );

    let output = Command::new("bash")
        .arg(verifier_script())
        .env("PATH", path_override)
        .env("TONIC_PROFILE_DOWNSTREAM_ASYNC_SKIP_BUILD", "1")
        .env("TONIC_PROFILE_DOWNSTREAM_ASYNC_BINARY", &fake_binary)
        .env("TONIC_PROFILE_DOWNSTREAM_ASYNC_DEVICE", DEVICE_PATH)
        .env(
            "TONIC_PROFILE_DOWNSTREAM_ASYNC_LAUNCHER_PATH",
            &launcher_path,
        )
        .env("TONIC_PROFILE_DOWNSTREAM_ASYNC_OUTPUT_DIR", &output_dir)
        .output()
        .expect("verifier should launch");

    assert_status(&output, 1);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=artifact_validation"));
    assert!(stderr.contains("artifact is not valid JSON"));
}

#[test]
fn custom_codec_preserves_synchronous_seam() {
    let codec_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/custom_codec.rs");
    let source = fs::read_to_string(&codec_path).unwrap_or_else(|err| {
        panic!(
            "custom codec source should be readable at {}: {err}",
            codec_path.display()
        )
    });

    for forbidden in ["AsyncDsaHandle", "block_on", "spawn_blocking"] {
        assert!(
            !source.contains(forbidden),
            "custom_codec.rs must not smuggle async handle behavior through `{forbidden}`"
        );
    }
}
