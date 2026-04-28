use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

fn bench_bin() -> &'static str {
    env!("CARGO_BIN_EXE_tokio_memmove_bench")
}

fn unique_temp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("idxd-rust-tokio-bench-{name}-{nanos}"))
}

fn run(args: &[&str]) -> Output {
    Command::new(bench_bin())
        .args(args)
        .output()
        .expect("binary should launch")
}

fn stdout_json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("stdout should be valid json")
}

#[test]
fn prints_help_without_touching_hardware() {
    let output = run(&["--help"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("tokio_memmove_bench"));
    assert!(stdout.contains("--backend <hardware|software>"));
    assert!(stdout.contains("--suite <canonical|latency|concurrency|throughput>"));
}

#[test]
fn rejects_invalid_numeric_inputs_before_touching_hardware() {
    for (flag, value) in [
        ("--bytes", "abc"),
        ("--bytes", "0"),
        ("--iterations", "0"),
        ("--concurrency", "0"),
        ("--duration-ms", "0"),
    ] {
        let output = run(&[flag, value]);

        assert_eq!(output.status.code(), Some(2), "{flag} {value}");
        assert!(String::from_utf8_lossy(&output.stdout).is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(flag),
            "stderr for {flag} {value}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn rejects_invalid_enum_and_device_inputs_before_touching_hardware() {
    for args in [
        ["--backend", "simulated"].as_slice(),
        ["--suite", "all"].as_slice(),
        ["--format", "xml"].as_slice(),
        ["--device", ""].as_slice(),
    ] {
        let output = run(args);

        assert_eq!(output.status.code(), Some(2), "args={args:?}");
        assert!(String::from_utf8_lossy(&output.stdout).is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).is_empty());
    }
}

#[test]
fn rejects_directory_artifact_path_before_benchmark_execution() {
    let temp_dir = unique_temp_path("artifact-dir");
    fs::create_dir_all(&temp_dir).expect("temp dir should be creatable");

    let output = run(&[
        "--backend",
        "software",
        "--format",
        "json",
        "--artifact",
        temp_dir.to_str().expect("temp path should be utf-8"),
    ]);

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stdout).is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("expected a writable file path"));

    fs::remove_dir_all(&temp_dir).expect("temp dir cleanup should succeed");
}

#[test]
fn emits_software_canonical_json_with_required_schema_fields() {
    let output = run(&[
        "--backend",
        "software",
        "--suite",
        "canonical",
        "--bytes",
        "64",
        "--iterations",
        "8",
        "--concurrency",
        "4",
        "--duration-ms",
        "100",
        "--format",
        "json",
    ]);

    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());

    let artifact = stdout_json(&output);
    assert_eq!(artifact["schema_version"], 1);
    assert_eq!(artifact["ok"], true);
    assert_eq!(artifact["verdict"], "pass");
    assert_eq!(artifact["backend"], "software");
    assert_eq!(artifact["claim_eligible"], false);
    assert_eq!(artifact["suite"], "canonical");
    assert_eq!(artifact["runtime_flavor"], "current_thread");
    assert_eq!(artifact["worker_threads"], 1);
    assert_eq!(artifact["requested_bytes"], 64);
    assert_eq!(artifact["iterations"], 8);
    assert_eq!(artifact["concurrency"], 4);
    assert_eq!(artifact["duration_ms"], 100);
    assert!(artifact["failure_class"].is_null());
    assert!(artifact["error_kind"].is_null());
    assert!(artifact["direct_failure_kind"].is_null());
    assert!(artifact["validation_phase"].is_null());
    assert!(artifact["validation_error_kind"].is_null());

    let results = artifact["results"]
        .as_array()
        .expect("results should be array");
    let modes: Vec<_> = results
        .iter()
        .map(|row| row["mode"].as_str().expect("mode should be string"))
        .collect();
    assert_eq!(
        modes,
        [
            "single_latency",
            "concurrent_submissions",
            "fixed_duration_throughput"
        ]
    );

    for row in results {
        assert_eq!(row["target"], "software_direct_async_diagnostic");
        assert!(row["comparison_target"].is_null());
        assert_eq!(row["requested_bytes"], 64);
        assert!(row["completed_operations"].as_u64().unwrap() > 0);
        assert_eq!(row["failed_operations"], 0);
        assert_eq!(row["verdict"], "pass");
        assert_eq!(row["claim_eligible"], false);
        assert!(row["failure_class"].is_null());
        assert!(row["error_kind"].is_null());
        assert!(row["direct_failure_kind"].is_null());
        assert!(row["validation_phase"].is_null());
        assert!(row["validation_error_kind"].is_null());
        assert!(row["elapsed_ns"].as_u64().unwrap() > 0);
        assert!(row["min_latency_ns"].as_u64().unwrap() > 0);
        assert!(row["mean_latency_ns"].as_u64().unwrap() > 0);
        assert!(row["max_latency_ns"].as_u64().unwrap() > 0);
        assert!(row["ops_per_sec"].as_f64().unwrap() > 0.0);
        assert!(row["bytes_per_sec"].as_f64().unwrap() > 0.0);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("payload"));
    assert!(!stdout.contains("destination"));
    assert!(!stdout.contains("source"));
}

#[test]
fn individual_software_suites_emit_only_the_requested_mode_at_minimum_bounds() {
    for (suite, expected_mode) in [
        ("latency", "single_latency"),
        ("concurrency", "concurrent_submissions"),
        ("throughput", "fixed_duration_throughput"),
    ] {
        let output = run(&[
            "--backend",
            "software",
            "--suite",
            suite,
            "--bytes",
            "1",
            "--iterations",
            "1",
            "--concurrency",
            "1",
            "--duration-ms",
            "1",
            "--format",
            "json",
        ]);

        assert_eq!(output.status.code(), Some(0), "suite={suite}");
        assert!(String::from_utf8_lossy(&output.stderr).is_empty());
        let artifact = stdout_json(&output);
        assert_eq!(artifact["suite"], suite);
        assert_eq!(artifact["claim_eligible"], false);
        let results = artifact["results"]
            .as_array()
            .expect("results should be array");
        assert_eq!(results.len(), 1, "suite={suite}");
        let row = &results[0];
        assert_eq!(row["mode"], expected_mode);
        assert_eq!(row["target"], "software_direct_async_diagnostic");
        assert!(row["completed_operations"].as_u64().unwrap() > 0);
        assert_eq!(row["failed_operations"], 0);
        assert_eq!(row["claim_eligible"], false);
        assert_eq!(row["verdict"], "pass");
    }
}

#[test]
fn writes_artifact_matching_stdout_exactly() {
    let artifact_path = unique_temp_path("artifact.json");

    let output = run(&[
        "--backend",
        "software",
        "--suite",
        "canonical",
        "--bytes",
        "1",
        "--iterations",
        "1",
        "--concurrency",
        "1",
        "--duration-ms",
        "1",
        "--format",
        "json",
        "--artifact",
        artifact_path
            .to_str()
            .expect("artifact path should be valid utf-8"),
    ]);

    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let artifact = fs::read_to_string(&artifact_path).expect("artifact should be written");
    assert_eq!(artifact, stdout);
    let parsed: Value = serde_json::from_str(&artifact).expect("artifact should parse as json");
    assert_eq!(parsed["claim_eligible"], false);
    assert_eq!(parsed["requested_bytes"], 1);

    fs::remove_file(&artifact_path).expect("artifact cleanup should succeed");
}
