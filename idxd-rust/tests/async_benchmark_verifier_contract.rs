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
    std::env::temp_dir().join(format!("idxd-rust-tokio-bench-verifier-{name}-{nanos}"))
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
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/verify_tokio_memmove_bench.sh")
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

fn write_fake_bench(path: &Path, scenario_body: &str) {
    write_executable(
        path,
        &format!(
            r##"#!/usr/bin/env bash
set -euo pipefail
artifact=
device=/dev/dsa/wq0.0
backend=hardware
suite=canonical
bytes=64
iterations=2
concurrency=2
duration_ms=10
while [[ $# -gt 0 ]]; do
  case "$1" in
    --bytes)
      if [[ ${{2:-}} == abc ]]; then
        echo 'tokio_memmove_bench: invalid value `abc` for `--bytes`; expected an integer in 1..=1073741824' >&2
        exit 2
      fi
      bytes=$2
      shift 2
      ;;
    --artifact)
      artifact=$2
      shift 2
      ;;
    --device)
      device=$2
      shift 2
      ;;
    --backend)
      backend=$2
      shift 2
      ;;
    --suite)
      suite=$2
      shift 2
      ;;
    --iterations)
      iterations=$2
      shift 2
      ;;
    --concurrency)
      concurrency=$2
      shift 2
      ;;
    --duration-ms)
      duration_ms=$2
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
        r#"python3 - "$artifact" "$device" "$backend" "$suite" "$bytes" "$iterations" "$concurrency" "$duration_ms" <<'PY'
import json
import sys
from pathlib import Path
artifact = Path(sys.argv[1])
device, backend, suite = sys.argv[2], sys.argv[3], sys.argv[4]
bytes_, iterations, concurrency, duration_ms = map(int, sys.argv[5:9])
{python_body}
artifact.write_text(json.dumps(report, separators=(',', ':')), encoding='utf-8')
print(artifact.read_text(encoding='utf-8'))
PY
exit {exit_code}"#
    )
}

fn success_report_body(backend_literal: &str, claim_eligible_literal: &str) -> String {
    format!(
        r#"def row(mode, target, comparison_target, claim_eligible):
    return {{
        "mode": mode,
        "target": target,
        "comparison_target": comparison_target,
        "requested_bytes": bytes_,
        "iterations": iterations,
        "concurrency": concurrency,
        "duration_ms": duration_ms,
        "completed_operations": 1,
        "failed_operations": 0,
        "elapsed_ns": 1000,
        "min_latency_ns": 1000,
        "mean_latency_ns": 1000,
        "max_latency_ns": 1000,
        "ops_per_sec": 1000000.0,
        "bytes_per_sec": float(bytes_) * 1000000.0,
        "verdict": "pass",
        "failure_class": None,
        "error_kind": None,
        "direct_failure_kind": None,
        "validation_phase": None,
        "validation_error_kind": None,
        "direct_retry_budget": None,
        "direct_retry_count": None,
        "completion_status": None,
        "completion_result": None,
        "completion_bytes_completed": None,
        "completion_fault_addr": None,
        "claim_eligible": claim_eligible,
    }}
backend = "{backend_literal}"
claim_eligible = {claim_eligible_literal}
target = "direct_async" if backend == "hardware" else "software_direct_async_diagnostic"
results = [
    row("single_latency", target, "direct_sync" if backend == "hardware" else None, claim_eligible),
    row("concurrent_submissions", target, None, claim_eligible),
    row("fixed_duration_throughput", target, None, claim_eligible),
]
if backend == "hardware":
    results.append(row("single_latency", "direct_sync", "direct_async", True))
report = {{
    "schema_version": 1,
    "ok": True,
    "verdict": "pass",
    "device_path": device,
    "backend": backend,
    "claim_eligible": claim_eligible,
    "suite": suite,
    "runtime_flavor": "current_thread",
    "worker_threads": 1,
    "requested_bytes": bytes_,
    "iterations": iterations,
    "concurrency": concurrency,
    "duration_ms": duration_ms,
    "failure_class": None,
    "error_kind": None,
    "direct_failure_kind": None,
    "validation_phase": None,
    "validation_error_kind": None,
    "direct_retry_budget": None,
    "direct_retry_count": None,
    "completion_status": None,
    "completion_result": None,
    "completion_bytes_completed": None,
    "completion_fault_addr": None,
    "results": results,
}}"#
    )
}

fn classified_failure_body() -> String {
    r#"report = {
    "schema_version": 1,
    "ok": False,
    "verdict": "expected_failure",
    "device_path": device,
    "backend": backend,
    "claim_eligible": False,
    "suite": suite,
    "runtime_flavor": "current_thread",
    "worker_threads": 1,
    "requested_bytes": bytes_,
    "iterations": iterations,
    "concurrency": concurrency,
    "duration_ms": duration_ms,
    "failure_class": "queue_open",
    "error_kind": "queue_open",
    "direct_failure_kind": None,
    "validation_phase": "queue_open",
    "validation_error_kind": "queue_open",
    "direct_retry_budget": None,
    "direct_retry_count": None,
    "completion_status": None,
    "completion_result": None,
    "completion_bytes_completed": None,
    "completion_fault_addr": None,
    "results": [],
}"#
    .to_string()
}

fn run_verifier(output_dir: &Path, envs: &[(&str, String)]) -> Output {
    let mut command = Command::new("bash");
    command.arg(verifier_script());
    command.env("IDXD_RUST_VERIFY_OUTPUT_DIR", output_dir);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("verifier should launch")
}

#[test]
fn prepared_host_hardware_pass_accepts_direct_async_and_sync_rows() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let fake_binary = temp_root.join("fake_tokio_memmove_bench");
    write_fake_bench(
        &fake_binary,
        &json_writer(&success_report_body("hardware", "True"), 0),
    );
    let output_dir = unique_temp_path("prepared-pass-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let output = run_verifier(
        &output_dir,
        &[
            ("PATH", path_override),
            ("IDXD_RUST_VERIFY_SKIP_BUILD", "1".to_string()),
            ("IDXD_RUST_VERIFY_BINARY", fake_binary.display().to_string()),
            ("IDXD_RUST_VERIFY_DEVICE", "/dev/dsa/test0.0".to_string()),
            (
                "IDXD_RUST_VERIFY_LAUNCHER_PATH",
                launcher_path.display().to_string(),
            ),
        ],
    );

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"));
    assert!(stdout.contains("verdict=pass"));
    assert!(stdout.contains("backend=hardware"));
    assert!(stdout.contains("claim_eligible=true"));
    assert!(stdout.contains("targets=direct_async,direct_async,direct_async,direct_sync"));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());
}

#[test]
fn software_diagnostic_mode_passes_without_launcher_and_is_not_claim_eligible() {
    let output_dir = unique_temp_path("software-pass-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let output = run_verifier(
        &output_dir,
        &[
            ("IDXD_RUST_VERIFY_BACKEND", "software".to_string()),
            ("IDXD_RUST_VERIFY_SKIP_BUILD", "1".to_string()),
            (
                "IDXD_RUST_VERIFY_BINARY",
                env!("CARGO_BIN_EXE_tokio_memmove_bench").to_string(),
            ),
            ("IDXD_RUST_VERIFY_BYTES", "1".to_string()),
            ("IDXD_RUST_VERIFY_ITERATIONS", "1".to_string()),
            ("IDXD_RUST_VERIFY_CONCURRENCY", "1".to_string()),
            ("IDXD_RUST_VERIFY_DURATION_MS", "1".to_string()),
        ],
    );

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"));
    assert!(stdout.contains("verdict=pass"));
    assert!(stdout.contains("backend=software"));
    assert!(stdout.contains("claim_eligible=false"));
    assert!(stdout.contains("targets=software_direct_async_diagnostic,software_direct_async_diagnostic,software_direct_async_diagnostic"));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());
}

#[test]
fn missing_launcher_is_expected_failure_with_default_repo_root_path() {
    let output_dir = unique_temp_path("missing-launcher-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let output = run_verifier(
        &output_dir,
        &[
            ("IDXD_RUST_VERIFY_SKIP_BUILD", "1".to_string()),
            (
                "IDXD_RUST_VERIFY_BINARY",
                env!("CARGO_BIN_EXE_tokio_memmove_bench").to_string(),
            ),
            ("IDXD_RUST_VERIFY_DEVICE", "/dev/dsa/test0.0".to_string()),
        ],
    );

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected_launcher = repo_root().join("tools/build/dsa_launcher");
    assert!(stdout.contains("phase=done"));
    assert!(stdout.contains("verdict=expected_failure"));
    assert!(stdout.contains("failure_phase=preflight"));
    assert!(stdout.contains("launcher_status=missing_launcher"));
    assert!(stdout.contains(&format!("launcher_path={}", expected_launcher.display())));
}

#[test]
fn missing_launcher_capability_is_expected_failure() {
    let (_temp_root, launcher_path, path_override) = fake_launcher_env(false);
    let output_dir = unique_temp_path("missing-cap-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let output = run_verifier(
        &output_dir,
        &[
            ("PATH", path_override),
            ("IDXD_RUST_VERIFY_SKIP_BUILD", "1".to_string()),
            (
                "IDXD_RUST_VERIFY_BINARY",
                env!("CARGO_BIN_EXE_tokio_memmove_bench").to_string(),
            ),
            ("IDXD_RUST_VERIFY_DEVICE", "/dev/dsa/test0.0".to_string()),
            (
                "IDXD_RUST_VERIFY_LAUNCHER_PATH",
                launcher_path.display().to_string(),
            ),
        ],
    );

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"));
    assert!(stdout.contains("verdict=expected_failure"));
    assert!(stdout.contains("failure_phase=preflight"));
    assert!(stdout.contains("launcher_status=missing_capability"));
}

#[test]
fn hardware_classified_binary_failure_is_expected_failure_and_preserves_paths() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let fake_binary = temp_root.join("fake_tokio_memmove_bench");
    write_fake_bench(&fake_binary, &json_writer(&classified_failure_body(), 1));
    let output_dir = unique_temp_path("classified-failure-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let output = run_verifier(
        &output_dir,
        &[
            ("PATH", path_override),
            ("IDXD_RUST_VERIFY_SKIP_BUILD", "1".to_string()),
            ("IDXD_RUST_VERIFY_BINARY", fake_binary.display().to_string()),
            ("IDXD_RUST_VERIFY_DEVICE", "/dev/dsa/test0.0".to_string()),
            (
                "IDXD_RUST_VERIFY_LAUNCHER_PATH",
                launcher_path.display().to_string(),
            ),
        ],
    );

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"));
    assert!(stdout.contains("verdict=expected_failure"));
    assert!(stdout.contains("failure_phase=runtime"));
    assert!(stdout.contains("failure_class=queue_open"));
    assert!(stdout.contains("error_kind=queue_open"));
    assert!(stdout.contains(&format!(
        "stdout={}",
        output_dir.join("tokio_memmove_bench.stdout").display()
    )));
    assert!(stdout.contains(&format!(
        "stderr={}",
        output_dir.join("tokio_memmove_bench.stderr").display()
    )));
}

#[test]
fn malformed_json_is_hard_artifact_validation_failure() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let fake_binary = temp_root.join("fake_tokio_memmove_bench");
    write_fake_bench(
        &fake_binary,
        "printf '{not-json' > \"$artifact\"\nprintf '{not-json\\n'\nexit 0",
    );
    let output_dir = unique_temp_path("malformed-json-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let output = run_verifier(
        &output_dir,
        &[
            ("PATH", path_override),
            ("IDXD_RUST_VERIFY_SKIP_BUILD", "1".to_string()),
            ("IDXD_RUST_VERIFY_BINARY", fake_binary.display().to_string()),
            ("IDXD_RUST_VERIFY_DEVICE", "/dev/dsa/test0.0".to_string()),
            (
                "IDXD_RUST_VERIFY_LAUNCHER_PATH",
                launcher_path.display().to_string(),
            ),
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=artifact_validation"));
    assert!(stderr.contains("artifact validation failed"));
}

#[test]
fn stdout_artifact_mismatch_is_hard_artifact_validation_failure() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let fake_binary = temp_root.join("fake_tokio_memmove_bench");
    write_fake_bench(
        &fake_binary,
        "printf '{\"schema_version\":1}' > \"$artifact\"\nprintf '{\"schema_version\":2}\\n'\nexit 0",
    );
    let output_dir = unique_temp_path("mismatch-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let output = run_verifier(
        &output_dir,
        &[
            ("PATH", path_override),
            ("IDXD_RUST_VERIFY_SKIP_BUILD", "1".to_string()),
            ("IDXD_RUST_VERIFY_BINARY", fake_binary.display().to_string()),
            ("IDXD_RUST_VERIFY_DEVICE", "/dev/dsa/test0.0".to_string()),
            (
                "IDXD_RUST_VERIFY_LAUNCHER_PATH",
                launcher_path.display().to_string(),
            ),
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=artifact_validation"));
    assert!(stderr.contains("stdout and artifact diverged"));
}

#[test]
fn software_claim_eligible_contradiction_is_hard_failure() {
    let (temp_root, _launcher_path, _path_override) = fake_launcher_env(true);
    let fake_binary = temp_root.join("fake_tokio_memmove_bench");
    write_fake_bench(
        &fake_binary,
        &json_writer(&success_report_body("software", "True"), 0),
    );
    let output_dir = unique_temp_path("software-claim-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let output = run_verifier(
        &output_dir,
        &[
            ("IDXD_RUST_VERIFY_BACKEND", "software".to_string()),
            ("IDXD_RUST_VERIFY_SKIP_BUILD", "1".to_string()),
            ("IDXD_RUST_VERIFY_BINARY", fake_binary.display().to_string()),
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=artifact_validation"));
    assert!(stderr.contains("artifact validation failed"));
}

#[test]
fn hardware_success_missing_sync_comparison_is_hard_failure() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let fake_binary = temp_root.join("fake_tokio_memmove_bench");
    let body = r#"def row(mode):
    return {
        "mode": mode,
        "target": "direct_async",
        "comparison_target": "direct_sync" if mode == "single_latency" else None,
        "requested_bytes": bytes_,
        "iterations": iterations,
        "concurrency": concurrency,
        "duration_ms": duration_ms,
        "completed_operations": 1,
        "failed_operations": 0,
        "elapsed_ns": 1000,
        "min_latency_ns": 1000,
        "mean_latency_ns": 1000,
        "max_latency_ns": 1000,
        "ops_per_sec": 1000000.0,
        "bytes_per_sec": float(bytes_) * 1000000.0,
        "verdict": "pass",
        "failure_class": None,
        "error_kind": None,
        "direct_failure_kind": None,
        "validation_phase": None,
        "validation_error_kind": None,
        "direct_retry_budget": None,
        "direct_retry_count": None,
        "completion_status": None,
        "completion_result": None,
        "completion_bytes_completed": None,
        "completion_fault_addr": None,
        "claim_eligible": True,
    }
report = {
    "schema_version": 1,
    "ok": True,
    "verdict": "pass",
    "device_path": device,
    "backend": "hardware",
    "claim_eligible": True,
    "suite": suite,
    "runtime_flavor": "current_thread",
    "worker_threads": 1,
    "requested_bytes": bytes_,
    "iterations": iterations,
    "concurrency": concurrency,
    "duration_ms": duration_ms,
    "failure_class": None,
    "error_kind": None,
    "direct_failure_kind": None,
    "validation_phase": None,
    "validation_error_kind": None,
    "direct_retry_budget": None,
    "direct_retry_count": None,
    "completion_status": None,
    "completion_result": None,
    "completion_bytes_completed": None,
    "completion_fault_addr": None,
    "results": [row("single_latency"), row("concurrent_submissions"), row("fixed_duration_throughput")],
}"#;
    write_fake_bench(&fake_binary, &json_writer(body, 0));
    let output_dir = unique_temp_path("missing-sync-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let output = run_verifier(
        &output_dir,
        &[
            ("PATH", path_override),
            ("IDXD_RUST_VERIFY_SKIP_BUILD", "1".to_string()),
            ("IDXD_RUST_VERIFY_BINARY", fake_binary.display().to_string()),
            ("IDXD_RUST_VERIFY_DEVICE", "/dev/dsa/test0.0".to_string()),
            (
                "IDXD_RUST_VERIFY_LAUNCHER_PATH",
                launcher_path.display().to_string(),
            ),
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=artifact_validation"));
    assert!(stderr.contains("artifact validation failed"));
}
