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

#[test]
fn short_selftest_window_still_emits_a_structurally_valid_report() {
    let addr = reserve_addr();
    let json_out = unique_path("short-selftest");
    let output = Command::new(tonic_profile_bin())
        .args([
            "--mode",
            "selftest",
            "--bind",
            &addr,
            "--target",
            &addr,
            "--payload-size",
            "32",
            "--payload-kind",
            "repeated",
            "--concurrency",
            "1",
            "--warmup-ms",
            "5",
            "--measure-ms",
            "10",
            "--requests",
            "1",
            "--instrumentation",
            "off",
            "--json-out",
            json_out.to_str().expect("utf8 path"),
        ])
        .output()
        .expect("spawn tonic-profile selftest");

    assert!(
        output.status.success(),
        "short selftest failed for {addr}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let raw = fs::read_to_string(&json_out).expect("read short selftest report");
    let report: Value = serde_json::from_str(&raw).expect("parse short selftest report");
    assert_eq!(report["metadata"]["mode"], "selftest");
    assert_eq!(report["metadata"]["instrumentation"], "off");
    assert_eq!(report["stages"]["enabled"], false);

    let requests = report["metrics"]["requests_completed"]
        .as_u64()
        .expect("requests_completed as u64");
    assert_eq!(requests, 1, "expected exactly one measured request");
}

#[test]
fn client_mode_without_server_fails_clearly() {
    let addr = reserve_addr();
    let json_out = unique_path("client-failure");
    let output = Command::new(tonic_profile_bin())
        .args([
            "--mode",
            "client",
            "--target",
            &addr,
            "--warmup-ms",
            "1",
            "--measure-ms",
            "1",
            "--requests",
            "1",
            "--json-out",
            json_out.to_str().expect("utf8 path"),
        ])
        .output()
        .expect("spawn tonic-profile client");

    assert!(
        !output.status.success(),
        "client unexpectedly succeeded against {addr}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !json_out.exists(),
        "client failure should not create a JSON artifact at {}",
        json_out.display()
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&addr)
            || stderr.contains("Connection refused")
            || stderr.contains("transport error")
            || stderr.contains("error trying to connect"),
        "stderr should identify the failed target or connection cause\nstderr:\n{stderr}"
    );
}
