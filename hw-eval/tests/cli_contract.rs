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
    assert!(
        !text.contains("0xAB") && !text.contains("171"),
        "diagnostics must not expose benchmark payload bytes: {text}"
    );
}

#[test]
fn malformed_size_reports_token_and_raw_list_without_json_or_panic() {
    let output = hw_eval(&["--sw-only", "--sizes", "64,abc,128", "--iterations", "1"]);

    assert!(
        !output.status.success(),
        "malformed --sizes should fail; stdout:\n{}\nstderr:\n{}",
        stdout(&output),
        stderr(&output)
    );

    let stdout = stdout(&output);
    let stderr = stderr(&output);

    assert!(
        stderr.contains("abc"),
        "stderr should name invalid token: {stderr}"
    );
    assert!(
        stderr.contains("64,abc,128"),
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
fn software_only_json_preserves_top_level_report_contract() {
    let output = hw_eval(&["--sw-only", "--json", "--sizes", "64", "--iterations", "1"]);

    assert!(
        output.status.success(),
        "software-only JSON run should succeed; stdout:\n{}\nstderr:\n{}",
        stdout(&output),
        stderr(&output)
    );

    let report = parse_stdout_json(&output);
    let object = report
        .as_object()
        .expect("top-level report is a JSON object");

    assert!(
        object.contains_key("metadata"),
        "missing metadata: {report}"
    );
    assert!(object.contains_key("latency"), "missing latency: {report}");
    assert!(
        object.contains_key("throughput"),
        "missing throughput: {report}"
    );

    let metadata = report["metadata"]
        .as_object()
        .expect("metadata is a JSON object");
    for key in ["accelerator", "device", "iterations", "cold_cache"] {
        assert!(
            metadata.contains_key(key),
            "metadata missing {key}: {metadata:?}"
        );
    }

    assert_eq!(metadata["iterations"], Value::from(1));
    assert_eq!(metadata["cold_cache"], Value::Bool(false));
    assert!(report["latency"].is_array(), "latency should be an array");
    assert!(
        report["throughput"].is_array(),
        "throughput should be an array"
    );

    assert_no_payload_bytes(&stdout(&output));
    assert_no_payload_bytes(&stderr(&output));
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
