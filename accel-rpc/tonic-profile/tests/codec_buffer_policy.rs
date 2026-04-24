use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use std::net::TcpListener;

const DEFAULT_CODEC_BUFFER_SIZE: u64 = 8 * 1024;
const DEFAULT_CODEC_YIELD_THRESHOLD: u64 = 32 * 1024;
const HEADER_SIZE: u64 = 5;

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

fn parse_report(path: &Path) -> Value {
    let raw = fs::read_to_string(path).expect("read report");
    serde_json::from_str(&raw).expect("parse report")
}

fn reserve_addr() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    addr.to_string()
}

fn run_selftest(buffer_policy: &str, payload_size: u64) -> Value {
    let addr = reserve_addr();
    let json_out = unique_path(buffer_policy);
    let output = Command::new(tonic_profile_bin())
        .args([
            "--mode",
            "selftest",
            "--bind",
            &addr,
            "--target",
            &addr,
            "--payload-size",
            &payload_size.to_string(),
            "--payload-kind",
            "structured",
            "--concurrency",
            "1",
            "--warmup-ms",
            "10",
            "--measure-ms",
            "20",
            "--requests",
            "1",
            "--buffer-policy",
            buffer_policy,
            "--json-out",
            json_out.to_str().expect("utf8 path"),
        ])
        .output()
        .expect("spawn tonic-profile selftest");

    assert!(
        output.status.success(),
        "selftest failed for buffer policy {buffer_policy}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    parse_report(&json_out)
}

fn effective_settings(report: &Value) -> (u64, u64) {
    (
        report["metadata"]["effective_codec_buffer_size"]
            .as_u64()
            .expect("effective_codec_buffer_size as u64"),
        report["metadata"]["effective_codec_yield_threshold"]
            .as_u64()
            .expect("effective_codec_yield_threshold as u64"),
    )
}

#[test]
fn build_script_generates_stubs_against_the_local_profile_codec() {
    let build_rs = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("build.rs"))
        .expect("read tonic-profile build.rs");
    assert!(
        build_rs.contains(".codec_path(\"crate::custom_codec::ProfileCodec\")"),
        "build.rs must generate stubs against the local profile codec\n{build_rs}"
    );
}

#[test]
fn buffer_policies_produce_distinct_effective_codec_settings() {
    let payload_size = 9000;
    let default_report = run_selftest("default", payload_size);
    let pooled_report = run_selftest("pooled", payload_size);
    let copy_minimized_report = run_selftest("copy-minimized", payload_size);

    assert_eq!(default_report["metadata"]["buffer_policy"], "default");
    assert_eq!(pooled_report["metadata"]["buffer_policy"], "pooled");
    assert_eq!(
        copy_minimized_report["metadata"]["buffer_policy"],
        "copy_minimized"
    );

    let default_settings = effective_settings(&default_report);
    let pooled_settings = effective_settings(&pooled_report);
    let copy_minimized_settings = effective_settings(&copy_minimized_report);
    let request_serialized_size = default_report["metadata"]["request_serialized_size"]
        .as_u64()
        .expect("request_serialized_size as u64");

    assert_eq!(
        default_settings,
        (DEFAULT_CODEC_BUFFER_SIZE, DEFAULT_CODEC_YIELD_THRESHOLD)
    );
    assert_eq!(pooled_settings, (16 * 1024, DEFAULT_CODEC_YIELD_THRESHOLD));
    assert_eq!(
        copy_minimized_settings,
        (
            request_serialized_size + HEADER_SIZE,
            DEFAULT_CODEC_YIELD_THRESHOLD,
        )
    );

    assert_ne!(default_settings, pooled_settings);
    assert_ne!(pooled_settings, copy_minimized_settings);
    assert_ne!(default_settings, copy_minimized_settings);
}

#[test]
fn small_payload_copy_minimized_run_keeps_bytes_path_and_valid_effective_settings() {
    let report = run_selftest("copy-minimized", 1);
    assert_eq!(report["metadata"]["rpc"], "unary-bytes");
    assert_eq!(report["metadata"]["mode"], "selftest");
    assert_eq!(report["metadata"]["buffer_policy"], "copy_minimized");

    let (buffer_size, yield_threshold) = effective_settings(&report);
    let request_serialized_size = report["metadata"]["request_serialized_size"]
        .as_u64()
        .expect("request_serialized_size as u64");
    assert_eq!(buffer_size, request_serialized_size + HEADER_SIZE);
    assert_eq!(yield_threshold, DEFAULT_CODEC_YIELD_THRESHOLD);

    let requests_completed = report["metrics"]["requests_completed"]
        .as_u64()
        .expect("requests_completed as u64");
    assert_eq!(
        requests_completed, 1,
        "expected exactly one measured request"
    );
}
