use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "idxd-rust-representative-bench-verifier-{name}-{}-{nanos}",
        std::process::id()
    ))
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
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/verify_idxd_representative_bench.sh")
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

fn write_fake_bench(path: &Path, scenario_body: &str) {
    write_executable(
        path,
        &format!(
            r##"#!/usr/bin/env bash
set -euo pipefail
artifact=
dsa_device=/dev/dsa/wq0.0
iax_device=/dev/iax/wq1.0
shared_device=
bytes=4096
iterations=1000
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dsa-device)
      dsa_device=${{2:-}}
      shift 2
      ;;
    --iax-device)
      iax_device=${{2:-}}
      shift 2
      ;;
    --dsa-shared-device)
      shared_device=${{2:-}}
      shift 2
      ;;
    --bytes)
      if [[ ${{2:-}} == abc ]]; then
        echo 'idxd_representative_bench: invalid value `abc` for `--bytes`; expected a positive integer' >&2
        exit 2
      fi
      bytes=${{2:-}}
      shift 2
      ;;
    --iterations)
      iterations=${{2:-}}
      shift 2
      ;;
    --artifact)
      artifact=${{2:-}}
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
{scenario_body}
"##,
        ),
    );
}

fn json_writer(python_body: &str, exit_code: i32) -> String {
    format!(
        r#"python3 - "$artifact" "$dsa_device" "$iax_device" "$shared_device" "$bytes" "$iterations" <<'PY'
import json
import sys
from pathlib import Path
artifact = Path(sys.argv[1])
dsa_device = sys.argv[2]
iax_device = sys.argv[3]
shared_device = sys.argv[4]
bytes_ = int(sys.argv[5])
iterations = int(sys.argv[6])
{python_body}
artifact.write_text(json.dumps(report, separators=(',', ':')), encoding='utf-8')
print(artifact.read_text(encoding='utf-8'))
PY
exit {exit_code}"#
    )
}

fn success_report_python_body(profile: &str, top_claim_eligible: &str) -> String {
    r#"def pass_row(target, operation, family, device_path, target_role, crc_required):
    return {
        "target": target,
        "operation": operation,
        "family": family,
        "device_path": device_path,
        "work_queue_mode": "shared" if target_role == "optional-shared" else "dedicated",
        "target_role": target_role,
        "requested_bytes": bytes_,
        "iterations": iterations,
        "warmup_iterations": 1,
        "ok": True,
        "verdict": "pass",
        "claim_eligible": __ROW_CLAIM_ELIGIBLE__,
        "completed_operations": iterations,
        "failed_operations": 0,
        "elapsed_ns": max(1, iterations * 1000),
        "min_latency_ns": 1000,
        "mean_latency_ns": 1000,
        "max_latency_ns": 1000,
        "ops_per_sec": 1000000.0,
        "bytes_per_sec": float(bytes_) * 1000000.0,
        "total_page_fault_retries": 0,
        "last_page_fault_retries": 0,
        "final_status": "0x00",
        "completion_error_code": None,
        "invalid_flags": None,
        "fault_addr": None,
        "crc64": "0x1234" if crc_required else None,
        "expected_crc64": "0x1234" if crc_required else None,
        "crc64_verified": True if crc_required else None,
        "failure_phase": None,
        "error_kind": None,
        "message": f"measured {iterations} fake {target} operations",
    }
targets = [
    pass_row("dsa-memmove", "memmove", "dsa", dsa_device, "required", False),
    pass_row("iax-crc64", "crc64", "iax", iax_device, "required", True),
]
if shared_device:
    targets.append(pass_row("dsa-shared-memmove", "memmove", "dsa", shared_device, "optional-shared", False))
report = {
    "schema_version": 1,
    "ok": True,
    "verdict": "pass",
    "claim_eligible": __TOP_CLAIM_ELIGIBLE__,
    "suite": "idxd_representative_bench",
    "profile": "__PROFILE__",
    "requested_bytes": bytes_,
    "iterations": iterations,
    "warmup_iterations": 1,
    "clock": "std::time::Instant",
    "failure_phase": None,
    "error_kind": None,
    "failure_target": None,
    "failure_accelerator": None,
    "targets": targets,
}"#
        .replace("__PROFILE__", profile)
        .replace("__TOP_CLAIM_ELIGIBLE__", top_claim_eligible)
        .replace(
            "__ROW_CLAIM_ELIGIBLE__",
            if profile == "release" { "True" } else { "False" },
        )
}

fn success_report_body(exit_code: i32) -> String {
    json_writer(&success_report_python_body("release", "True"), exit_code)
}

fn failure_report_python_body() -> String {
    r#"def pass_row(target, operation, family, device_path, target_role, crc_required):
    return {
        "target": target,
        "operation": operation,
        "family": family,
        "device_path": device_path,
        "work_queue_mode": "shared" if target_role == "optional-shared" else "dedicated",
        "target_role": target_role,
        "requested_bytes": bytes_,
        "iterations": iterations,
        "warmup_iterations": 1,
        "ok": True,
        "verdict": "pass",
        "claim_eligible": True,
        "completed_operations": iterations,
        "failed_operations": 0,
        "elapsed_ns": max(1, iterations * 1000),
        "min_latency_ns": 1000,
        "mean_latency_ns": 1000,
        "max_latency_ns": 1000,
        "ops_per_sec": 1000000.0,
        "bytes_per_sec": float(bytes_) * 1000000.0,
        "total_page_fault_retries": 0,
        "last_page_fault_retries": 0,
        "final_status": "0x00",
        "completion_error_code": None,
        "invalid_flags": None,
        "fault_addr": None,
        "crc64": "0x1234" if crc_required else None,
        "expected_crc64": "0x1234" if crc_required else None,
        "crc64_verified": True if crc_required else None,
        "failure_phase": None,
        "error_kind": None,
        "message": f"measured fake {target}",
    }
def fail_row(target, operation, family, device_path, target_role):
    return {
        "target": target,
        "operation": operation,
        "family": family,
        "device_path": device_path,
        "work_queue_mode": None,
        "target_role": target_role,
        "requested_bytes": bytes_,
        "iterations": iterations,
        "warmup_iterations": 1,
        "ok": False,
        "verdict": "expected_failure",
        "claim_eligible": False,
        "completed_operations": 0,
        "failed_operations": 1,
        "elapsed_ns": None,
        "min_latency_ns": None,
        "mean_latency_ns": None,
        "max_latency_ns": None,
        "ops_per_sec": None,
        "bytes_per_sec": None,
        "total_page_fault_retries": None,
        "last_page_fault_retries": None,
        "final_status": None,
        "completion_error_code": None,
        "invalid_flags": None,
        "fault_addr": None,
        "crc64": None,
        "expected_crc64": None,
        "crc64_verified": None,
        "failure_phase": "queue_open",
        "error_kind": "queue_open",
        "message": f"queue-open failure for {device_path}",
    }
targets = [
    fail_row("dsa-memmove", "memmove", "dsa", dsa_device, "required"),
    pass_row("iax-crc64", "crc64", "iax", iax_device, "required", True),
]
if shared_device:
    targets.append(pass_row("dsa-shared-memmove", "memmove", "dsa", shared_device, "optional-shared", False))
report = {
    "schema_version": 1,
    "ok": False,
    "verdict": "expected_failure",
    "claim_eligible": False,
    "suite": "idxd_representative_bench",
    "profile": "release",
    "requested_bytes": bytes_,
    "iterations": iterations,
    "warmup_iterations": 1,
    "clock": "std::time::Instant",
    "failure_phase": "queue_open",
    "error_kind": "queue_open",
    "failure_target": "dsa-memmove",
    "failure_accelerator": "dsa",
    "targets": targets,
}"#
    .to_string()
}

fn failure_report_body(exit_code: i32) -> String {
    json_writer(&failure_report_python_body(), exit_code)
}

fn run_verifier(output_dir: &Path, envs: &[(&str, String)]) -> Output {
    let mut command = Command::new("bash");
    command
        .arg(verifier_script())
        .env("IDXD_RUST_VERIFY_OUTPUT_DIR", output_dir);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("verifier should launch")
}

fn base_env(
    path_override: String,
    launcher_path: &Path,
    fake_binary: &Path,
) -> Vec<(&'static str, String)> {
    vec![
        ("PATH", path_override),
        ("IDXD_RUST_VERIFY_SKIP_BUILD", "1".to_string()),
        ("IDXD_RUST_VERIFY_BINARY", fake_binary.display().to_string()),
        (
            "IDXD_RUST_VERIFY_LAUNCHER_PATH",
            launcher_path.display().to_string(),
        ),
        ("IDXD_RUST_VERIFY_DSA_DEVICE", "/dev/dsa/wq0.0".to_string()),
        ("IDXD_RUST_VERIFY_IAX_DEVICE", "/dev/iax/wq1.0".to_string()),
    ]
}

fn run_with_fake_bench(scenario_body: &str, output_name: &str) -> (Output, PathBuf, PathBuf) {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let fake_binary = temp_root.join("fake_idxd_representative_bench");
    write_fake_bench(&fake_binary, scenario_body);
    let output_dir = unique_temp_path(output_name);
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");
    let output = run_verifier(
        &output_dir,
        &base_env(path_override, &launcher_path, &fake_binary),
    );
    (output, output_dir, fake_binary)
}

#[test]
fn benchmark_verifier_uses_representative_bench_not_prior_proof_surfaces() {
    let source = fs::read_to_string(verifier_script()).expect("verifier script should be tracked");

    assert!(source.contains("idxd_representative_bench"));
    for forbidden in [
        "live_idxd_op",
        "tokio_memmove_bench",
        "live_memmove",
        "hw-eval",
    ] {
        assert!(
            !source.contains(forbidden),
            "benchmark verifier must not validate forbidden surface {forbidden:?}"
        );
    }
}

#[test]
fn prepared_host_pass_final_line_includes_release_claim_paths_and_required_targets() {
    let (output, output_dir, _fake_binary) =
        run_with_fake_bench(&success_report_body(0), "pass-output");

    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"), "stdout was: {stdout}");
    assert!(stdout.contains("verdict=pass"), "stdout was: {stdout}");
    assert!(
        stdout.contains("launcher_status=ready"),
        "stdout was: {stdout}"
    );
    assert!(stdout.contains("profile=release"), "stdout was: {stdout}");
    assert!(
        stdout.contains("requested_bytes=4096"),
        "stdout was: {stdout}"
    );
    assert!(stdout.contains("iterations=1000"), "stdout was: {stdout}");
    assert!(
        stdout.contains("claim_eligible=true"),
        "stdout was: {stdout}"
    );
    assert!(stdout.contains("targets=dsa-memmove:/dev/dsa/wq0.0,iax-crc64:/dev/iax/wq1.0"));
    assert!(!stdout.contains("dsa-shared-memmove:/dev/dsa/wq0.1"));
    assert!(stdout.contains(&format!(
        "artifact={}",
        output_dir.join("idxd_representative_bench.json").display()
    )));
    assert!(stdout.contains(
        &format!("stdout={}", output_dir.join("idxd_representative_bench.stdout").display())
    ));
    assert!(stdout.contains(
        &format!("stderr={}", output_dir.join("idxd_representative_bench.stderr").display())
    ));
    assert!(
        stdout.contains(&format!(
            "raw_stdout={}",
            output_dir
                .join("idxd_representative_bench.stdout.raw")
                .display()
        ))
    );
}

#[test]
fn optional_shared_dsa_target_is_reported_only_when_configured() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let fake_binary = temp_root.join("fake_idxd_representative_bench");
    write_fake_bench(&fake_binary, &success_report_body(0));
    let output_dir = unique_temp_path("shared-pass-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");
    let mut envs = base_env(path_override, &launcher_path, &fake_binary);
    envs.push((
        "IDXD_RUST_VERIFY_DSA_SHARED_DEVICE",
        "/dev/dsa/wq0.1".to_string(),
    ));

    let output = run_verifier(&output_dir, &envs);

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"), "stdout was: {stdout}");
    assert!(stdout.contains("verdict=pass"), "stdout was: {stdout}");
    assert!(stdout.contains(
        "targets=dsa-memmove:/dev/dsa/wq0.0,iax-crc64:/dev/iax/wq1.0,dsa-shared-memmove:/dev/dsa/wq0.1"
    ));
    assert!(stdout.contains("artifact_targets=dsa-memmove,iax-crc64,dsa-shared-memmove"));
}

#[test]
fn missing_launcher_capability_is_expected_failure() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(false);
    let fake_binary = temp_root.join("fake_idxd_representative_bench");
    write_fake_bench(&fake_binary, &success_report_body(0));
    let output_dir = unique_temp_path("missing-cap-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let output = run_verifier(
        &output_dir,
        &base_env(path_override, &launcher_path, &fake_binary),
    );

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"));
    assert!(stdout.contains("verdict=expected_failure"));
    assert!(stdout.contains("failure_phase=preflight"));
    assert!(stdout.contains("launcher_status=missing_capability"));
}

#[test]
fn missing_launcher_is_expected_failure() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let missing_launcher = launcher_path.with_file_name("missing_dsa_launcher");
    let fake_binary = temp_root.join("fake_idxd_representative_bench");
    write_fake_bench(&fake_binary, &success_report_body(0));
    let output_dir = unique_temp_path("missing-launcher-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let output = run_verifier(
        &output_dir,
        &base_env(path_override, &missing_launcher, &fake_binary),
    );

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"));
    assert!(stdout.contains("verdict=expected_failure"));
    assert!(stdout.contains("failure_phase=preflight"));
    assert!(stdout.contains("launcher_status=missing_launcher"));
}

#[test]
fn classified_runtime_failure_artifact_is_expected_failure_and_preserves_paths() {
    let (output, output_dir, _fake_binary) =
        run_with_fake_bench(&failure_report_body(1), "runtime-failure-output");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"), "stdout was: {stdout}");
    assert!(
        stdout.contains("verdict=expected_failure"),
        "stdout was: {stdout}"
    );
    assert!(
        stdout.contains("failure_phase=runtime"),
        "stdout was: {stdout}"
    );
    assert!(
        stdout.contains("artifact_failure_phase=queue_open"),
        "stdout was: {stdout}"
    );
    assert!(
        stdout.contains("artifact_error_kind=queue_open"),
        "stdout was: {stdout}"
    );
    assert!(stdout.contains(&format!(
        "artifact={}",
        output_dir.join("idxd_representative_bench.json").display()
    )));
    assert!(stdout.contains(
        &format!("stdout={}", output_dir.join("idxd_representative_bench.stdout").display())
    ));
    assert!(stdout.contains(
        &format!("stderr={}", output_dir.join("idxd_representative_bench.stderr").display())
    ));
}

#[test]
fn runtime_timeout_is_expected_failure_with_output_metadata() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let fake_binary = temp_root.join("fake_idxd_representative_bench");
    write_fake_bench(&fake_binary, "sleep 2");
    let output_dir = unique_temp_path("runtime-timeout-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");
    let mut envs = base_env(path_override, &launcher_path, &fake_binary);
    envs.push(("IDXD_RUST_VERIFY_RUN_TIMEOUT", "1s".to_string()));

    let output = run_verifier(&output_dir, &envs);

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"), "stdout was: {stdout}");
    assert!(
        stdout.contains("verdict=expected_failure"),
        "stdout was: {stdout}"
    );
    assert!(
        stdout.contains("failure_phase=runtime_timeout"),
        "stdout was: {stdout}"
    );
    assert!(
        stdout.contains("dsa_device=/dev/dsa/wq0.0"),
        "stdout was: {stdout}"
    );
    assert!(
        stdout.contains("iax_device=/dev/iax/wq1.0"),
        "stdout was: {stdout}"
    );
}

#[test]
fn malformed_json_is_hard_artifact_validation_failure() {
    let (output, _output_dir, _fake_binary) = run_with_fake_bench(
        "printf '{not-json' > \"$artifact\"\nprintf '{not-json\\n'\nexit 0",
        "malformed-json-output",
    );

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("phase=artifact_validation"),
        "stderr was: {stderr}"
    );
    assert!(
        stderr.contains("artifact is not valid JSON"),
        "stderr was: {stderr}"
    );
}

#[test]
fn missing_schema_fields_are_hard_artifact_validation_failures() {
    let mut body = success_report_python_body("release", "True");
    body.push_str("\ndel report['clock']\n");
    let (output, _output_dir, _fake_binary) =
        run_with_fake_bench(&json_writer(&body, 0), "missing-field-output");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=artifact_validation"));
    assert!(stderr.contains("artifact missing required top-level fields: clock"));
}

#[test]
fn payload_fields_are_hard_artifact_validation_failures() {
    let mut body = success_report_python_body("release", "True");
    body.push_str("\nreport['targets'][0]['source_payload'] = [1, 2, 3]\n");
    let (output, _output_dir, _fake_binary) =
        run_with_fake_bench(&json_writer(&body, 0), "payload-field-output");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=artifact_validation"));
    assert!(stderr.contains("forbidden payload dump field report.targets[0].source_payload"));
}

#[test]
fn non_release_profile_artifact_is_rejected() {
    let body = success_report_python_body("debug", "False");
    let (output, _output_dir, _fake_binary) =
        run_with_fake_bench(&json_writer(&body, 0), "non-release-output");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=artifact_validation"));
    assert!(stderr.contains("artifact profile="));
    assert!(stderr.contains("expected release"));
}

#[test]
fn zero_pass_metrics_are_rejected() {
    let body = success_report_python_body("release", "True").replace(
        "\"elapsed_ns\": max(1, iterations * 1000),",
        "\"elapsed_ns\": 0,",
    );
    let (output, _output_dir, _fake_binary) =
        run_with_fake_bench(&json_writer(&body, 0), "zero-metric-output");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=artifact_validation"));
    assert!(stderr.contains("elapsed_ns must be a positive integer on pass"));
}

#[test]
fn missing_required_row_is_rejected() {
    let mut body = success_report_python_body("release", "True");
    body.push_str("\nreport['targets'] = [row for row in report['targets'] if row['target'] != 'iax-crc64']\n");
    let (output, _output_dir, _fake_binary) =
        run_with_fake_bench(&json_writer(&body, 0), "missing-row-output");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=artifact_validation"));
    assert!(stderr.contains("artifact missing required benchmark targets: iax-crc64"));
}

#[test]
fn stdout_artifact_mismatch_is_rejected() {
    let (output, _output_dir, _fake_binary) = run_with_fake_bench(
        "printf '{\"schema_version\":1}' > \"$artifact\"\nprintf '{\"schema_version\":2}\\n'\nexit 0",
        "stdout-mismatch-output",
    );

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=artifact_validation"));
    assert!(stderr.contains("stdout and artifact diverged"));
}

#[test]
fn zero_exit_with_failure_artifact_is_rejected_as_contradictory() {
    let (output, _output_dir, _fake_binary) =
        run_with_fake_bench(&failure_report_body(0), "zero-exit-failure-artifact-output");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=runtime"));
    assert!(stderr.contains("exited zero despite a failure artifact"));
}

#[test]
fn nonzero_exit_with_pass_artifact_is_rejected_as_contradictory() {
    let (output, _output_dir, _fake_binary) =
        run_with_fake_bench(&success_report_body(1), "nonzero-exit-pass-artifact-output");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=runtime"));
    assert!(stderr.contains("exited non-zero despite a success artifact"));
}

#[test]
fn binary_override_without_skip_build_is_rejected_before_building() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let fake_binary = temp_root.join("fake_idxd_representative_bench");
    write_fake_bench(&fake_binary, &success_report_body(0));
    let output_dir = unique_temp_path("contradictory-override-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");
    let mut envs = base_env(path_override, &launcher_path, &fake_binary);
    envs.retain(|(key, _)| *key != "IDXD_RUST_VERIFY_SKIP_BUILD");

    let output = run_verifier(&output_dir, &envs);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=preflight"));
    assert!(stderr.contains("launcher_status=contradictory_overrides"));
}

#[test]
fn invalid_iteration_override_is_rejected_in_preflight() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let fake_binary = temp_root.join("fake_idxd_representative_bench");
    write_fake_bench(&fake_binary, &success_report_body(0));
    let output_dir = unique_temp_path("invalid-iterations-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");
    let mut envs = base_env(path_override, &launcher_path, &fake_binary);
    envs.push(("IDXD_RUST_VERIFY_ITERATIONS", "0".to_string()));

    let output = run_verifier(&output_dir, &envs);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=preflight"));
    assert!(stderr.contains("launcher_status=invalid_iterations"));
}
