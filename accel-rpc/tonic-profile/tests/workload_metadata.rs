use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

fn tonic_profile_bin() -> &'static str {
    env!("CARGO_BIN_EXE_tonic-profile")
}

fn unique_path(name: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "tonic-profile-{name}-{}-{ts}.json",
        std::process::id()
    ))
}

fn reserve_addr() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    addr.to_string()
}

fn parse_report(path: &PathBuf) -> Value {
    let raw = fs::read_to_string(path).expect("read report");
    serde_json::from_str(&raw).expect("parse report")
}

#[test]
fn asymmetric_response_metadata_reports_shape_and_size_separately() {
    let addr = reserve_addr();
    let json_out = unique_path("metadata-asymmetric");
    let output = Command::new(tonic_profile_bin())
        .args([
            "--mode",
            "selftest",
            "--rpc",
            "unary-proto-shape",
            "--proto-shape",
            "fleet-small",
            "--response-shape",
            "fleet-response-heavy",
            "--bind",
            &addr,
            "--target",
            &addr,
            "--warmup-ms",
            "10",
            "--measure-ms",
            "20",
            "--requests",
            "1",
            "--json-out",
            json_out.to_str().expect("utf8 path"),
        ])
        .output()
        .expect("spawn tonic-profile asymmetric metadata selftest");

    assert!(
        output.status.success(),
        "asymmetric proto selftest failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report = parse_report(&json_out);
    assert_eq!(report["metadata"]["selection_policy"], "explicit_response");
    assert_eq!(report["metadata"]["request_shape"], "fleet-small");
    assert_eq!(report["metadata"]["response_shape"], "fleet-response-heavy");
    assert!(report["metadata"]["workload_label"]
        .as_str()
        .unwrap()
        .contains("fleet-small-to-fleet-response-heavy"));

    let request_size = report["metadata"]["request_serialized_size"]
        .as_u64()
        .expect("request_serialized_size as u64");
    let response_size = report["metadata"]["response_serialized_size"]
        .as_u64()
        .expect("response_serialized_size as u64");
    assert!(request_size > 0, "request size should be recorded");
    assert!(
        response_size > request_size,
        "response-heavy workload should produce a larger response than request ({response_size} <= {request_size})"
    );
}

#[test]
fn response_shape_same_boundary_remains_valid_for_string_heavy_shape() {
    let addr = reserve_addr();
    let json_out = unique_path("metadata-same-boundary");
    let output = Command::new(tonic_profile_bin())
        .args([
            "--mode",
            "selftest",
            "--rpc",
            "unary-proto-shape",
            "--proto-shape",
            "fleet-string-heavy",
            "--response-shape",
            "same",
            "--bind",
            &addr,
            "--target",
            &addr,
            "--warmup-ms",
            "10",
            "--measure-ms",
            "20",
            "--requests",
            "1",
            "--json-out",
            json_out.to_str().expect("utf8 path"),
        ])
        .output()
        .expect("spawn tonic-profile same-shape boundary selftest");

    assert!(
        output.status.success(),
        "same-shape proto selftest failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report = parse_report(&json_out);
    assert_eq!(report["metadata"]["selection_policy"], "same_as_request");
    assert_eq!(report["metadata"]["request_shape"], "fleet-string-heavy");
    assert_eq!(report["metadata"]["response_shape"], "fleet-string-heavy");
    let request_size = report["metadata"]["request_serialized_size"]
        .as_u64()
        .expect("request_serialized_size as u64");
    let response_size = report["metadata"]["response_serialized_size"]
        .as_u64()
        .expect("response_serialized_size as u64");
    assert!(
        request_size >= response_size,
        "request should include the explicit response selector field"
    );
    assert!(
        request_size - response_size <= 8,
        "same-shape boundary should only differ by the small selector field overhead ({request_size} vs {response_size})"
    );
}
