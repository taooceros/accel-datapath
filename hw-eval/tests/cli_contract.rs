use std::collections::BTreeSet;
use std::process::{Command, Output};

use serde_json::Value;

fn hw_eval(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_hw-eval"))
        .args(args)
        .output()
        .expect("failed to run hw-eval binary")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn parse_stdout_json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout was not valid JSON: {error}\nstdout:\n{}\nstderr:\n{}",
            stdout(output),
            stderr(output)
        )
    })
}

fn assert_no_payload_bytes(text: &str) {
    let lower = text.to_ascii_lowercase();
    for forbidden in [
        "0xab",
        "source_bytes",
        "destination_bytes",
        "payload_bytes",
        "src_bytes",
        "dst_bytes",
        "[171",
        "171, 171",
    ] {
        assert!(
            !lower.contains(forbidden),
            "diagnostics must not expose benchmark payload bytes: {text}"
        );
    }
}

fn assert_exact_object_keys(object_name: &str, value: &Value, expected: &[&str]) {
    let object = value
        .as_object()
        .unwrap_or_else(|| panic!("{object_name} is not a JSON object: {value}"));
    let actual_keys = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected_keys = expected.iter().copied().collect::<BTreeSet<_>>();

    assert_eq!(
        actual_keys, expected_keys,
        "{object_name} JSON keys drifted: {object:?}"
    );
}

fn latency_benchmark_names(report: &Value) -> BTreeSet<&str> {
    report["latency"]
        .as_array()
        .expect("latency is a JSON array")
        .iter()
        .map(|row| {
            row["benchmark"]
                .as_str()
                .unwrap_or_else(|| panic!("latency row missing string benchmark: {row}"))
        })
        .collect()
}

fn assert_malformed_sizes_fail_without_panic(raw_sizes: &str, expected_fragment: &str) {
    let output = hw_eval(&["--sw-only", "--sizes", raw_sizes, "--iterations", "1"]);

    assert!(
        !output.status.success(),
        "malformed --sizes should fail; stdout:\n{}\nstderr:\n{}",
        stdout(&output),
        stderr(&output)
    );

    let stdout = stdout(&output);
    let stderr = stderr(&output);
    assert!(
        stderr.contains(expected_fragment),
        "stderr should contain {expected_fragment:?}: {stderr}"
    );
    assert!(
        stderr.contains(raw_sizes),
        "stderr should include original --sizes list: {stderr}"
    );
    assert!(
        !stderr.to_ascii_lowercase().contains("panicked"),
        "CLI validation should not panic: {stderr}"
    );
    assert!(
        stdout.trim().is_empty() || serde_json::from_str::<Value>(&stdout).is_err(),
        "failed CLI validation should not emit a valid JSON report: {stdout}"
    );
    assert_no_payload_bytes(&stderr);
}

#[test]
fn malformed_size_reports_token_and_raw_list_without_json_or_panic() {
    assert_malformed_sizes_fail_without_panic("64,abc,128", "abc");
}

#[test]
fn malformed_size_boundaries_fail_without_json_or_panic() {
    for (raw_sizes, expected_fragment) in [
        ("", "--sizes must not contain empty entries"),
        (",", "--sizes must not contain empty entries"),
        ("64,,128", "--sizes must not contain empty entries"),
        ("0", "greater than zero"),
        ("64,0,128", "greater than zero"),
    ] {
        assert_malformed_sizes_fail_without_panic(raw_sizes, expected_fragment);
    }
}

#[test]
fn software_only_json_preserves_top_level_report_contract() {
    let missing_device = std::env::temp_dir().join(format!(
        "hw-eval-sw-only-missing-wq-contract-{}",
        std::process::id()
    ));
    let missing_device = missing_device
        .to_str()
        .expect("temp path should be valid UTF-8 for CLI test");
    let output = hw_eval(&[
        "--sw-only",
        "--json",
        "--device",
        missing_device,
        "--sizes",
        "64",
        "--iterations",
        "1",
    ]);

    assert!(
        output.status.success(),
        "software-only JSON run should not access the missing WQ; stdout:\n{}\nstderr:\n{}",
        stdout(&output),
        stderr(&output)
    );

    let report = parse_stdout_json(&output);

    assert_exact_object_keys(
        "top-level report",
        &report,
        &["latency", "metadata", "throughput"],
    );
    assert_exact_object_keys(
        "metadata",
        &report["metadata"],
        &[
            "accelerator",
            "cold_cache",
            "cpu_numa_node",
            "device",
            "device_numa_node",
            "iterations",
            "pinned_core",
            "tsc_freq_hz",
            "wq_dedicated",
        ],
    );

    let metadata = report["metadata"]
        .as_object()
        .expect("metadata is a JSON object");

    assert_eq!(metadata["iterations"], Value::from(1));
    assert_eq!(metadata["cold_cache"], Value::Bool(false));
    assert_eq!(metadata["accelerator"], Value::from("dsa"));
    assert_eq!(metadata["device"], Value::from(missing_device));
    assert!(
        metadata["tsc_freq_hz"].is_number(),
        "missing numeric TSC frequency: {metadata:?}"
    );
    assert!(
        metadata["pinned_core"].is_number(),
        "missing numeric pinned core: {metadata:?}"
    );
    assert!(metadata["cpu_numa_node"].is_number() || metadata["cpu_numa_node"].is_null());
    assert!(metadata["device_numa_node"].is_null());
    assert!(metadata["wq_dedicated"].is_null());

    assert!(report["latency"].is_array(), "latency should be an array");
    assert!(
        report["throughput"].is_array(),
        "throughput should be an array"
    );

    let latency_names = latency_benchmark_names(&report);
    assert!(
        latency_names.contains("sw_memcpy"),
        "missing sw_memcpy latency row: {latency_names:?}"
    );
    assert!(
        latency_names.contains("sw_crc32c"),
        "missing sw_crc32c latency row: {latency_names:?}"
    );

    assert_no_payload_bytes(&stdout(&output));
    assert_no_payload_bytes(&stderr(&output));
}

#[test]
fn missing_hardware_device_is_structured_nonzero_error() {
    let missing_device = std::env::temp_dir().join(format!(
        "hw-eval-missing-wq-contract-{}",
        std::process::id()
    ));
    let missing_device = missing_device
        .to_str()
        .expect("temp path should be valid UTF-8 for CLI test");
    let output = hw_eval(&[
        "--device",
        missing_device,
        "--sizes",
        "64",
        "--iterations",
        "1",
    ]);

    assert!(
        !output.status.success(),
        "missing hardware WQ should fail visibly; stdout:\n{}\nstderr:\n{}",
        stdout(&output),
        stderr(&output)
    );

    let stderr = stderr(&output);
    assert!(stderr.contains("open_wq"), "missing operation: {stderr}");
    assert!(stderr.contains("dsa"), "missing accelerator: {stderr}");
    assert!(
        stderr.contains(missing_device),
        "missing device path: {stderr}"
    );
    assert!(
        stderr.contains("CAP_SYS_RAWIO") && stderr.contains("dsa_launcher"),
        "missing operator hint: {stderr}"
    );
    assert!(
        !stderr.to_ascii_lowercase().contains("panicked"),
        "WQ open failure should not panic: {stderr}"
    );
    assert_no_payload_bytes(&stderr);
}

#[test]
fn invalid_pin_core_warns_without_breaking_json_report() {
    let output = hw_eval(&[
        "--sw-only",
        "--json",
        "--sizes",
        "64",
        "--iterations",
        "1",
        "--pin-core",
        "999999",
    ]);

    assert!(
        output.status.success(),
        "pinning failure should be non-fatal; stdout:\n{}\nstderr:\n{}",
        stdout(&output),
        stderr(&output)
    );

    let stderr = stderr(&output);
    assert!(
        stderr.to_ascii_lowercase().contains("warning"),
        "invalid pin core should produce a warning: {stderr}"
    );
    assert!(
        stderr.contains("999999"),
        "pin warning should include requested core: {stderr}"
    );
    assert!(
        !stderr.to_ascii_lowercase().contains("panicked"),
        "pin warning should not be a panic: {stderr}"
    );

    let report = parse_stdout_json(&output);
    assert!(report["metadata"].is_object(), "missing metadata: {report}");
    assert!(report["latency"].is_array(), "missing latency: {report}");
    assert!(
        report["throughput"].is_array(),
        "missing throughput: {report}"
    );

    assert_no_payload_bytes(&stdout(&output));
    assert_no_payload_bytes(&stderr);
}
