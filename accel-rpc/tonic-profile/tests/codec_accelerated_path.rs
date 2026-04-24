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
    listener.local_addr().expect("local addr").to_string()
}

fn parse_report(path: &PathBuf) -> Value {
    let raw = fs::read_to_string(path).expect("read report");
    serde_json::from_str(&raw).expect("parse report")
}

#[test]
fn software_selftest_stays_healthy_for_bytes_and_proto_workloads() {
    for (name, extra_args, expected_label_prefix) in [
        (
            "software-bytes",
            vec![
                "--payload-size",
                "64",
                "--payload-kind",
                "repeated",
                "--instrumentation",
                "on",
            ],
            "ordinary/unary-bytes/repeated-64",
        ),
        (
            "software-proto",
            vec![
                "--rpc",
                "unary-proto-shape",
                "--proto-shape",
                "fleet-response-heavy",
                "--response-shape",
                "fleet-response-heavy",
                "--instrumentation",
                "on",
            ],
            "ordinary/unary-proto-shape/fleet-response-heavy-to-fleet-response-heavy",
        ),
    ] {
        let addr = reserve_addr();
        let json_out = unique_path(name);
        let mut args = vec![
            "--mode",
            "selftest",
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
        ];
        args.extend(extra_args);

        let output = Command::new(tonic_profile_bin())
            .args(&args)
            .output()
            .expect("spawn tonic-profile software selftest");

        assert!(
            output.status.success(),
            "software selftest failed for {name}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let report = parse_report(&json_out);
        assert_eq!(report["metadata"]["selected_path"], "software");
        assert!(report["metadata"]["accelerated_device_path"].is_null());
        assert!(report["metadata"]["accelerated_lane"].is_null());
        assert_eq!(report["metadata"]["endpoint_role"], "selftest");
        assert_eq!(report["metadata"]["seam"], "codec_body");
        assert_eq!(report["metrics"]["requests_completed"], 1);
        assert!(report["metadata"]["workload_label"]
            .as_str()
            .expect("workload label")
            .starts_with(expected_label_prefix));
        assert!(report["stages"]["encode"]["count"].as_u64().unwrap() > 0);
        assert!(report["stages"]["decode"]["count"].as_u64().unwrap() > 0);
    }
}

#[test]
fn idxd_mode_fails_explicitly_on_queue_open_instead_of_falling_back() {
    let bad_device = "/dev/dsa/this-device-should-not-exist";

    for (name, extra_args) in [
        (
            "idxd-bytes-queue-open",
            vec!["--payload-size", "64", "--payload-kind", "repeated"],
        ),
        (
            "idxd-proto-queue-open",
            vec![
                "--rpc",
                "unary-proto-shape",
                "--proto-shape",
                "fleet-response-heavy",
                "--response-shape",
                "fleet-response-heavy",
            ],
        ),
    ] {
        let addr = reserve_addr();
        let json_out = unique_path(name);
        let mut args = vec![
            "--mode",
            "selftest",
            "--bind",
            &addr,
            "--target",
            &addr,
            "--warmup-ms",
            "0",
            "--measure-ms",
            "20",
            "--requests",
            "1",
            "--accelerated-path",
            "idxd",
            "--accelerator-device",
            bad_device,
            "--json-out",
            json_out.to_str().expect("utf8 path"),
        ];
        args.extend(extra_args);

        let output = Command::new(tonic_profile_bin())
            .args(&args)
            .output()
            .expect("spawn tonic-profile idxd selftest");

        assert!(
            !output.status.success(),
            "idxd selftest unexpectedly succeeded for {name}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("idxd codec copy lane failure during queue_open"),
            "stderr should expose the queue-open phase\nstderr:\n{stderr}"
        );
        assert!(
            stderr.contains(bad_device),
            "stderr should retain the requested device path\nstderr:\n{stderr}"
        );
        assert!(
            stderr.contains("Error: \"idxd codec copy lane failure")
                || stderr.contains("selftest client execution failed")
                || stderr.contains("worker 0 unary request failed"),
            "stderr should show the explicit queue-open failure surface\nstderr:\n{stderr}"
        );
        assert!(
            !json_out.exists(),
            "failed idxd runs must not emit a bogus success artifact at {}",
            json_out.display()
        );
    }
}
