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
    std::env::temp_dir().join(format!(
        "idxd-rust-async-throughput-matrix-{name}-{}-{nanos}",
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

fn matrix_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/bench_async_throughput_matrix.sh")
}

fn fake_launcher_env() -> (PathBuf, PathBuf, String) {
    let temp_root = unique_temp_path("launcher-ready");
    let shim_dir = temp_root.join("bin");
    fs::create_dir_all(&shim_dir).expect("shim dir should be creatable");

    let launcher_path = temp_root.join("dsa_launcher");
    write_executable(
        &launcher_path,
        "#!/usr/bin/env bash\nset -euo pipefail\nexec \"$@\"\n",
    );

    write_executable(
        &shim_dir.join("devenv"),
        &format!(
            "#!/usr/bin/env bash
set -euo pipefail
if [[ ${{1:-}} != shell || ${{2:-}} != -- || ${{3:-}} != launch ]]; then
  echo \"unexpected devenv invocation: $*\" >&2
  exit 90
fi
shift 3
printf 'Running: {} %s\\n' \"$*\"
exec \"$@\"
",
            launcher_path.display()
        ),
    );

    write_executable(
        &shim_dir.join("getcap"),
        &format!(
            "#!/usr/bin/env bash
set -euo pipefail
printf '%s cap_sys_rawio+eip\\n' {:?}
",
            launcher_path.display().to_string()
        ),
    );

    let mut path_entries = vec![shim_dir.display().to_string()];
    if let Some(existing) = std::env::var_os("PATH") {
        path_entries.push(existing.to_string_lossy().into_owned());
    }
    (temp_root, launcher_path, path_entries.join(":"))
}

fn write_fake_tokio_bench(path: &Path) {
    write_executable(
        path,
        r##"#!/usr/bin/env bash
set -euo pipefail
backend=hardware
suite=throughput
device=/dev/dsa/wq-test
bytes=64
iterations=1
concurrency=1
duration_ms=10
max_page_fault_retries=1
artifact=
while [[ $# -gt 0 ]]; do
  case "$1" in
    --backend)
      backend=${2:-}
      shift 2
      ;;
    --suite)
      suite=${2:-}
      shift 2
      ;;
    --device)
      device=${2:-}
      shift 2
      ;;
    --bytes)
      if [[ ${2:-} == abc ]]; then
        echo 'tokio_memmove_bench: invalid value `abc` for `--bytes`; expected an integer in 1..=1073741824' >&2
        exit 2
      fi
      bytes=${2:-}
      shift 2
      ;;
    --iterations)
      iterations=${2:-}
      shift 2
      ;;
    --concurrency)
      concurrency=${2:-}
      shift 2
      ;;
    --duration-ms)
      duration_ms=${2:-}
      shift 2
      ;;
    --max-page-fault-retries)
      max_page_fault_retries=${2:-}
      shift 2
      ;;
    --artifact)
      artifact=${2:-}
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
python3 - "$artifact" "$backend" "$suite" "$device" "$bytes" "$iterations" "$concurrency" "$duration_ms" "$max_page_fault_retries" <<'PY'
import json
import sys
from pathlib import Path

artifact = Path(sys.argv[1])
backend = sys.argv[2]
suite = sys.argv[3]
device = sys.argv[4]
bytes_ = int(sys.argv[5])
iterations = int(sys.argv[6])
concurrency = int(sys.argv[7])
duration_ms = int(sys.argv[8])
max_page_fault_retries = int(sys.argv[9])
completed = max(1, concurrency * 10)
elapsed_ns = max(1, duration_ms * 1_000_000)
ops_per_sec = completed * 1_000_000_000.0 / elapsed_ns
bytes_per_sec = completed * bytes_ * 1_000_000_000.0 / elapsed_ns
row = {
    "mode": "fixed_duration_throughput",
    "target": "direct_async",
    "comparison_target": None,
    "requested_bytes": bytes_,
    "iterations": iterations,
    "concurrency": concurrency,
    "duration_ms": duration_ms,
    "completed_operations": completed,
    "failed_operations": 0,
    "elapsed_ns": elapsed_ns,
    "min_latency_ns": 1000,
    "mean_latency_ns": 1000,
    "max_latency_ns": 1000,
    "ops_per_sec": ops_per_sec,
    "bytes_per_sec": bytes_per_sec,
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
    "claim_eligible": backend == "hardware",
}
report = {
    "schema_version": 1,
    "ok": True,
    "verdict": "pass",
    "device_path": device,
    "backend": backend,
    "claim_eligible": backend == "hardware",
    "suite": suite,
    "runtime_flavor": "current_thread",
    "worker_threads": 1,
    "requested_bytes": bytes_,
    "iterations": iterations,
    "concurrency": concurrency,
    "duration_ms": duration_ms,
    "max_page_fault_retries": max_page_fault_retries,
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
    "results": [row],
}
text = json.dumps(report, separators=(",", ":"))
artifact.write_text(text, encoding="utf-8")
print(text)
PY
"##,
    );
}

#[test]
fn matrix_script_aggregates_fake_hardware_points_into_csv() {
    let (_temp_root, launcher_path, path_override) = fake_launcher_env();
    let fake_binary = unique_temp_path("fake-tokio-bench");
    write_fake_tokio_bench(&fake_binary);
    let output_dir = unique_temp_path("matrix-output");

    let output = Command::new("bash")
        .arg(matrix_script())
        .env("PATH", path_override)
        .env("IDXD_RUST_BENCH_SKIP_BUILD", "1")
        .env("IDXD_RUST_BENCH_BINARY", &fake_binary)
        .env("IDXD_RUST_BENCH_LAUNCHER_PATH", &launcher_path)
        .env("IDXD_RUST_BENCH_DEVICE", "/dev/dsa/wq-test")
        .env("IDXD_RUST_BENCH_OUTPUT_DIR", &output_dir)
        .env("IDXD_RUST_BENCH_BYTES", "64,128")
        .env("IDXD_RUST_BENCH_CONCURRENCY", "1,2")
        .env("IDXD_RUST_BENCH_DURATION_MS", "7")
        .env("IDXD_RUST_BENCH_MAX_PAGE_FAULT_RETRIES", "9")
        .output()
        .expect("matrix script should launch");

    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"));
    assert!(stdout.contains("verdict=pass"));
    assert!(stdout.contains("points=4"));

    let csv_path = output_dir.join("async_throughput_matrix.csv");
    let csv = fs::read_to_string(&csv_path).expect("CSV should be written");
    let rows: Vec<&str> = csv.lines().collect();
    assert_eq!(rows.len(), 5, "header + four matrix rows expected: {csv}");
    assert!(rows[0].contains("backend,device_path,bytes,concurrency,duration_ms,max_page_fault_retries"));
    assert!(csv.contains("hardware,/dev/dsa/wq-test,64,1,7,9"));
    assert!(csv.contains("hardware,/dev/dsa/wq-test,128,2,7,9"));
    assert!(csv.contains("tokio_memmove_bench.json"));
    assert!(output_dir.join("bytes-64/concurrency-1/tokio_memmove_bench.json").is_file());
}

#[test]
fn matrix_script_rejects_invalid_numeric_lists_before_running() {
    let output_dir = unique_temp_path("invalid-list-output");
    let output = Command::new("bash")
        .arg(matrix_script())
        .env("IDXD_RUST_BENCH_SKIP_BUILD", "1")
        .env("IDXD_RUST_BENCH_BINARY", "/does/not/matter")
        .env("IDXD_RUST_BENCH_OUTPUT_DIR", &output_dir)
        .env("IDXD_RUST_BENCH_BYTES", "64,abc")
        .output()
        .expect("matrix script should launch");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=preflight"));
    assert!(stderr.contains("IDXD_RUST_BENCH_BYTES entries must be positive integers"));
}
