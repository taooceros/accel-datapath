use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("idxd-rust-representative-verifier-{name}-{nanos}"))
}

fn write_executable(path: &Path, content: &str) {
    fs::write(path, content).expect("script should be writable");
    let mut perms = fs::metadata(path)
        .expect("script metadata should exist")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("script should be executable");
}

fn verifier_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/verify_idxd_representative_ops.sh")
}

fn live_idxd_op_bin() -> &'static str {
    env!("CARGO_BIN_EXE_live_idxd_op")
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
    let script = r#"#!/usr/bin/env bash
set -euo pipefail
op=
artifact=
device=/dev/unknown
bytes=64
while [[ $# -gt 0 ]]; do
  case "$1" in
    --op)
      op=${2:-}
      shift 2
      ;;
    --artifact)
      artifact=${2:-}
      shift 2
      ;;
    --device)
      device=${2:-}
      shift 2
      ;;
    --bytes)
      if [[ ${2:-} == abc ]]; then
        echo 'live_idxd_op: invalid value `abc` for `--bytes`; expected a positive integer' >&2
        exit 2
      fi
      bytes=${2:-}
      shift 2
      ;;
    --format)
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done
if [[ -z "$artifact" ]]; then
  echo 'missing artifact path' >&2
  exit 91
fi
__BODY__
"#
    .replace("__BODY__", body);
    write_executable(path, &script);
}

fn json_writer(python_body: &str, exit_code: i32) -> String {
    r#"python3 - "$artifact" "$op" "$device" "$bytes" <<'PY'
import json
import sys
from pathlib import Path

artifact = Path(sys.argv[1])
op = sys.argv[2]
device = sys.argv[3]
bytes_ = int(sys.argv[4])
__PYTHON_BODY__
artifact.write_text(json.dumps(report, separators=(',', ':')), encoding='utf-8')
print(artifact.read_text(encoding='utf-8'))
PY
exit __EXIT_CODE__"#
        .replace("__PYTHON_BODY__", python_body)
        .replace("__EXIT_CODE__", &exit_code.to_string())
}

fn success_report_body(exit_code: i32) -> String {
    json_writer(
        r#"if op == 'dsa-memmove':
    report = {
        'ok': True,
        'operation': 'dsa-memmove',
        'accelerator': 'dsa',
        'device_path': device,
        'requested_bytes': bytes_,
        'page_fault_retries': 0,
        'final_status': '0x00',
        'phase': 'completed',
        'error_kind': None,
        'completion_error_code': None,
        'invalid_flags': None,
        'fault_addr': None,
        'crc64': None,
        'expected_crc64': None,
        'crc64_verified': None,
        'message': f'verified {bytes_} copied bytes via fake IdxdSession<Dsa> memmove',
    }
elif op == 'iax-crc64':
    report = {
        'ok': True,
        'operation': 'iax-crc64',
        'accelerator': 'iax',
        'device_path': device,
        'requested_bytes': bytes_,
        'page_fault_retries': 0,
        'final_status': '0x00',
        'phase': 'completed',
        'error_kind': None,
        'completion_error_code': None,
        'invalid_flags': None,
        'fault_addr': None,
        'crc64': '0x1234',
        'expected_crc64': '0x1234',
        'crc64_verified': True,
        'message': 'verified crc64 result via fake IdxdSession<Iax>',
    }
else:
    raise SystemExit(f'unexpected op {op!r}')"#,
        exit_code,
    )
}

fn failure_report_body(exit_code: i32) -> String {
    json_writer(
        r#"accelerator = 'dsa' if op == 'dsa-memmove' else 'iax'
report = {
    'ok': False,
    'operation': op,
    'accelerator': accelerator,
    'device_path': device,
    'requested_bytes': bytes_,
    'page_fault_retries': None,
    'final_status': None,
    'phase': 'queue_open',
    'error_kind': 'queue_open',
    'completion_error_code': None,
    'invalid_flags': None,
    'fault_addr': None,
    'crc64': None,
    'expected_crc64': None,
    'crc64_verified': None,
    'message': f'queue-open failure for {device}',
}"#,
        exit_code,
    )
}

fn payload_leak_body() -> String {
    json_writer(
        r#"report = {
    'ok': True,
    'operation': 'dsa-memmove',
    'accelerator': 'dsa',
    'device_path': device,
    'requested_bytes': bytes_,
    'page_fault_retries': 0,
    'final_status': '0x00',
    'phase': 'completed',
    'error_kind': None,
    'completion_error_code': None,
    'invalid_flags': None,
    'fault_addr': None,
    'crc64': None,
    'expected_crc64': None,
    'crc64_verified': None,
    'message': f'verified {bytes_} copied bytes via fake',
    'source_payload': [1, 2, 3],
}"#,
        0,
    )
}

fn base_command(
    path_override: &str,
    launcher_path: &Path,
    output_dir: &Path,
    binary: &Path,
) -> Command {
    let mut command = Command::new("bash");
    command
        .arg(verifier_script())
        .env("PATH", path_override)
        .env("IDXD_RUST_VERIFY_SKIP_BUILD", "1")
        .env("IDXD_RUST_VERIFY_BINARY", binary)
        .env("IDXD_RUST_VERIFY_DSA_DEVICE", "/dev/dsa/wq0.0")
        .env("IDXD_RUST_VERIFY_IAX_DEVICE", "/dev/iax/wq1.0")
        .env("IDXD_RUST_VERIFY_LAUNCHER_PATH", launcher_path)
        .env("IDXD_RUST_VERIFY_OUTPUT_DIR", output_dir);
    command
}

#[test]
fn representative_verifier_uses_live_idxd_op_not_legacy_surfaces() {
    let source = fs::read_to_string(verifier_script()).expect("verifier script should be tracked");

    assert!(source.contains("live_idxd_op"));
    for forbidden in ["live_memmove", "DsaSession", "hw-eval"] {
        assert!(
            !source.contains(forbidden),
            "representative verifier must not call forbidden surface {forbidden:?}"
        );
    }
}

#[test]
fn verifier_fails_preflight_when_launcher_capability_is_missing() {
    let (_temp_root, launcher_path, path_override) = fake_launcher_env(false);
    let output_dir = unique_temp_path("missing-cap-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let output = base_command(
        &path_override,
        &launcher_path,
        &output_dir,
        Path::new(live_idxd_op_bin()),
    )
    .output()
    .expect("verifier should launch");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"));
    assert!(stdout.contains("verdict=expected_failure"));
    assert!(stdout.contains("failure_phase=preflight"));
    assert!(stdout.contains("launcher_status=missing_capability"));
    assert!(stdout.contains("dsa-memmove:/dev/dsa/wq0.0"));
    assert!(stdout.contains("iax-crc64:/dev/iax/wq1.0"));
    assert!(stdout.contains(&format!("launcher_path={}", launcher_path.display())));
}

#[test]
fn verifier_preserves_queue_open_failure_and_artifact_paths() {
    let (_temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let output_dir = unique_temp_path("queue-open-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let output = base_command(
        &path_override,
        &launcher_path,
        &output_dir,
        Path::new(live_idxd_op_bin()),
    )
    .output()
    .expect("verifier should launch");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"));
    assert!(stdout.contains("verdict=expected_failure"));
    assert!(stdout.contains("failure_phase=runtime"));
    assert!(stdout.contains("launcher_status=ready"));
    assert!(stdout.contains("target=dsa-memmove"));
    assert!(stdout.contains("validation_phase=queue_open"));
    assert!(stdout.contains("validation_error_kind=queue_open"));
    assert!(stdout.contains(&format!(
        "artifact={}",
        output_dir.join("dsa_memmove.json").display()
    )));
    assert!(stdout.contains(&format!(
        "stdout={}",
        output_dir.join("dsa_memmove.stdout").display()
    )));
    assert!(stdout.contains(&format!(
        "stderr={}",
        output_dir.join("dsa_memmove.stderr").display()
    )));
}

#[test]
fn verifier_rejects_malformed_artifact_output() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let output_dir = unique_temp_path("malformed-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let fake_binary = temp_root.join("fake_live_idxd_op");
    write_fake_binary(
        &fake_binary,
        "printf '{\"ok\":' | tee \"$artifact\"\nexit 1",
    );

    let output = base_command(&path_override, &launcher_path, &output_dir, &fake_binary)
        .output()
        .expect("verifier should launch");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=artifact_validation"));
    assert!(stderr.contains("artifact is not valid JSON"));
    assert!(stderr.contains(&format!(
        "artifact={}",
        output_dir.join("dsa_memmove.json").display()
    )));
}

#[test]
fn verifier_rejects_payload_dump_fields_in_artifact() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let output_dir = unique_temp_path("payload-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let fake_binary = temp_root.join("fake_live_idxd_op");
    write_fake_binary(&fake_binary, &payload_leak_body());

    let output = base_command(&path_override, &launcher_path, &output_dir, &fake_binary)
        .output()
        .expect("verifier should launch");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=artifact_validation"));
    assert!(stderr.contains("forbidden payload dump field report.source_payload"));
}

#[test]
fn verifier_rejects_contradictory_success_exit_status() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let output_dir = unique_temp_path("contradictory-success-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let fake_binary = temp_root.join("fake_live_idxd_op");
    write_fake_binary(&fake_binary, &success_report_body(1));

    let output = base_command(&path_override, &launcher_path, &output_dir, &fake_binary)
        .output()
        .expect("verifier should launch");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=runtime"));
    assert!(stderr.contains("live_idxd_op exited non-zero despite a success artifact"));
    assert!(stderr.contains("launcher_status=ready"));
}

#[test]
fn verifier_reports_runtime_timeout_with_target_metadata() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let output_dir = unique_temp_path("timeout-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let fake_binary = temp_root.join("fake_live_idxd_op");
    write_fake_binary(&fake_binary, "sleep 2\n");

    let output = base_command(&path_override, &launcher_path, &output_dir, &fake_binary)
        .env("IDXD_RUST_VERIFY_RUN_TIMEOUT", "1s")
        .output()
        .expect("verifier should launch");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"));
    assert!(stdout.contains("verdict=expected_failure"));
    assert!(stdout.contains("failure_phase=runtime"));
    assert!(stdout.contains("failure_kind=timeout"));
    assert!(stdout.contains("target=dsa-memmove"));
    assert!(stdout.contains(&format!(
        "artifact={}",
        output_dir.join("dsa_memmove.json").display()
    )));
}

#[test]
fn verifier_passes_successful_required_and_shared_targets() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let output_dir = unique_temp_path("success-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let fake_binary = temp_root.join("fake_live_idxd_op");
    write_fake_binary(&fake_binary, &success_report_body(0));

    let output = base_command(&path_override, &launcher_path, &output_dir, &fake_binary)
        .env("IDXD_RUST_VERIFY_DSA_SHARED_DEVICE", "/dev/dsa/wq0.1")
        .output()
        .expect("verifier should launch");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"));
    assert!(stdout.contains("verdict=pass"));
    assert!(stdout.contains("launcher_status=ready"));
    assert!(stdout.contains(
        "targets=dsa-memmove:/dev/dsa/wq0.0,dsa-memmove-shared:/dev/dsa/wq0.1,iax-crc64:/dev/iax/wq1.0"
    ));
    assert!(stdout.contains(&format!(
        "artifact_paths={},{},{}",
        output_dir.join("dsa_memmove.json").display(),
        output_dir.join("dsa_memmove_shared.json").display(),
        output_dir.join("iax_crc64.json").display()
    )));
    assert!(stdout.contains(&format!(
        "stdout_paths={},{},{}",
        output_dir.join("dsa_memmove.stdout").display(),
        output_dir.join("dsa_memmove_shared.stdout").display(),
        output_dir.join("iax_crc64.stdout").display()
    )));
    assert!(stdout.contains(&format!(
        "stderr_paths={},{},{}",
        output_dir.join("dsa_memmove.stderr").display(),
        output_dir.join("dsa_memmove_shared.stderr").display(),
        output_dir.join("iax_crc64.stderr").display()
    )));

    let dsa_artifact = fs::read_to_string(output_dir.join("dsa_memmove.json"))
        .expect("DSA artifact should be written");
    assert!(dsa_artifact.contains("\"operation\":\"dsa-memmove\""));
    let iax_artifact = fs::read_to_string(output_dir.join("iax_crc64.json"))
        .expect("IAX artifact should be written");
    assert!(iax_artifact.contains("\"operation\":\"iax-crc64\""));
    assert!(iax_artifact.contains("\"crc64_verified\":true"));
}

#[test]
fn verifier_treats_runtime_failure_artifact_as_expected_failure() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let output_dir = unique_temp_path("failure-artifact-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let fake_binary = temp_root.join("fake_live_idxd_op");
    write_fake_binary(&fake_binary, &failure_report_body(1));

    let output = base_command(&path_override, &launcher_path, &output_dir, &fake_binary)
        .output()
        .expect("verifier should launch");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"));
    assert!(stdout.contains("verdict=expected_failure"));
    assert!(stdout.contains("failure_phase=runtime"));
    assert!(stdout.contains("validation_phase=queue_open"));
    assert!(stdout.contains("validation_error_kind=queue_open"));
    assert!(stdout.contains("artifact_paths="));
}
