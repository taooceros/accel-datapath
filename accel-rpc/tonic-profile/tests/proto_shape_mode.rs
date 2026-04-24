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
fn unary_bytes_mode_remains_backward_compatible() {
    let addr = reserve_addr();
    let json_out = unique_path("bytes-backcompat");
    let output = Command::new(tonic_profile_bin())
        .args([
            "--mode",
            "selftest",
            "--rpc",
            "unary-bytes",
            "--bind",
            &addr,
            "--target",
            &addr,
            "--payload-size",
            "64",
            "--payload-kind",
            "repeated",
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
        .expect("spawn tonic-profile bytes selftest");

    assert!(
        output.status.success(),
        "bytes selftest failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report = parse_report(&json_out);
    assert_eq!(report["metadata"]["rpc"], "unary-bytes");
    assert_eq!(report["metadata"]["selection_policy"], "echo_payload");
    assert!(report["metadata"]["request_shape"].is_null());
    assert!(report["metadata"]["response_shape"].is_null());
    assert_eq!(report["metadata"]["payload_size"], 64);
    assert_eq!(report["metadata"]["payload_kind"], "repeated");
}

#[test]
fn unary_proto_shape_mode_supports_same_shape_round_trip() {
    let addr = reserve_addr();
    let json_out = unique_path("proto-shape-same");
    let output = Command::new(tonic_profile_bin())
        .args([
            "--mode",
            "selftest",
            "--rpc",
            "unary-proto-shape",
            "--proto-shape",
            "fleet-small",
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
        .expect("spawn tonic-profile proto selftest");

    assert!(
        output.status.success(),
        "proto selftest failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report = parse_report(&json_out);
    assert_eq!(report["metadata"]["rpc"], "unary-proto-shape");
    assert_eq!(report["metadata"]["ordinary_path"], "software");
    assert_eq!(report["metadata"]["seam"], "codec_body");
    assert_eq!(report["metadata"]["selection_policy"], "same_as_request");
    assert_eq!(report["metadata"]["request_shape"], "fleet-small");
    assert_eq!(report["metadata"]["response_shape"], "fleet-small");
    assert!(report["metadata"]["request_serialized_size"].as_u64().unwrap() > 0);
    assert!(report["metadata"]["response_serialized_size"].as_u64().unwrap() > 0);
    assert!(report["metadata"]["payload_size"].is_null());
    assert!(report["metadata"]["payload_kind"].is_null());
}

#[test]
fn invalid_proto_shape_selection_fails_fast() {
    let output = Command::new(tonic_profile_bin())
        .args([
            "--mode",
            "client",
            "--rpc",
            "unary-proto-shape",
            "--proto-shape",
            "not-a-shape",
        ])
        .output()
        .expect("spawn tonic-profile with invalid proto shape");

    assert!(
        !output.status.success(),
        "invalid proto shape unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid value") && stderr.contains("not-a-shape"),
        "stderr should explain the invalid proto shape\nstderr:\n{stderr}"
    );
}
