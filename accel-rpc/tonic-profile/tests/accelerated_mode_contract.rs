use std::fs;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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

fn wait_for_port(addr: &str, timeout: Duration) {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if TcpStream::connect(addr).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }
    panic!("timed out waiting for server to listen on {addr}");
}

fn wait_for_exit(child: &mut Child, timeout: Duration) {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if child.try_wait().expect("poll child status").is_some() {
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }
    let _ = child.kill();
    let _ = child.wait();
    panic!("timed out waiting for tonic-profile server process to exit");
}

fn assert_idxd_metadata(report: &Value, endpoint_role: &str, device_path: &str) {
    assert_eq!(report["metadata"]["endpoint_role"], endpoint_role);
    assert_eq!(report["metadata"]["ordinary_path"], "software");
    assert_eq!(report["metadata"]["selected_path"], "idxd");
    assert_eq!(report["metadata"]["accelerated_device_path"], device_path);
    assert_eq!(report["metadata"]["accelerated_lane"], "codec_memmove");
    assert_eq!(report["metadata"]["accelerated_direction"], "bidirectional");
    assert_eq!(report["metadata"]["seam"], "codec_body");
}

#[test]
fn idxd_mode_requires_an_explicit_device_path() {
    let output = Command::new(tonic_profile_bin())
        .args(["--mode", "client", "--accelerated-path", "idxd"])
        .output()
        .expect("spawn tonic-profile without idxd device path");

    assert!(
        !output.status.success(),
        "idxd mode unexpectedly succeeded without a device path\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--accelerator-device is required with --accelerated-path idxd"),
        "stderr should explain the missing idxd device path\nstderr:\n{stderr}"
    );
}

#[test]
fn software_mode_rejects_idxd_only_flags() {
    let output = Command::new(tonic_profile_bin())
        .args(["--mode", "client", "--accelerator-device", "/dev/dsa/wq0.0"])
        .output()
        .expect("spawn tonic-profile with software-only invalid flags");

    assert!(
        !output.status.success(),
        "software mode unexpectedly accepted idxd-only flags\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--accelerator-device is only supported with --accelerated-path idxd"),
        "stderr should explain the incompatible flag combination\nstderr:\n{stderr}"
    );
}

#[test]
fn invalid_accelerator_lane_fails_fast() {
    let output = Command::new(tonic_profile_bin())
        .args([
            "--mode",
            "client",
            "--accelerated-path",
            "idxd",
            "--accelerator-device",
            "/dev/dsa/wq0.0",
            "--accelerator-lane",
            "not-a-lane",
        ])
        .output()
        .expect("spawn tonic-profile with invalid accelerator lane");

    assert!(
        !output.status.success(),
        "invalid accelerator lane unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid value") && stderr.contains("not-a-lane"),
        "stderr should explain the invalid accelerator lane\nstderr:\n{stderr}"
    );
}

#[test]
fn selftest_reports_explicit_idxd_selection_metadata() {
    let addr = reserve_addr();
    let json_out = unique_path("idxd-selftest");
    let device_path = "/dev/dsa/wq0.0";
    let output = Command::new(tonic_profile_bin())
        .args([
            "--mode",
            "selftest",
            "--bind",
            &addr,
            "--target",
            &addr,
            "--payload-size",
            "64",
            "--concurrency",
            "1",
            "--warmup-ms",
            "10",
            "--measure-ms",
            "20",
            "--requests",
            "1",
            "--accelerated-path",
            "idxd",
            "--accelerator-device",
            device_path,
            "--json-out",
            json_out.to_str().expect("utf8 path"),
        ])
        .output()
        .expect("spawn tonic-profile idxd selftest");

    assert!(
        output.status.success(),
        "idxd selftest failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report = parse_report(&json_out);
    assert_eq!(report["metadata"]["mode"], "selftest");
    assert_idxd_metadata(&report, "selftest", device_path);
}

#[test]
fn split_client_and_server_preserve_idxd_selection_metadata() {
    let addr = reserve_addr();
    let server_json = unique_path("idxd-server");
    let client_json = unique_path("idxd-client");
    let run_id = format!("run-idxd-contract-{}", std::process::id());
    let device_path = "/dev/dsa/wq0.0";

    let mut server = Command::new(tonic_profile_bin())
        .args([
            "--mode",
            "server",
            "--bind",
            &addr,
            "--target",
            &addr,
            "--payload-size",
            "64",
            "--warmup-ms",
            "0",
            "--measure-ms",
            "20",
            "--requests",
            "1",
            "--instrumentation",
            "on",
            "--run-id",
            &run_id,
            "--shutdown-after-requests",
            "1",
            "--accelerated-path",
            "idxd",
            "--accelerator-device",
            device_path,
            "--server-json-out",
            server_json.to_str().expect("utf8 path"),
        ])
        .spawn()
        .expect("spawn tonic-profile idxd server");

    wait_for_port(&addr, Duration::from_secs(5));

    let client_output = Command::new(tonic_profile_bin())
        .args([
            "--mode",
            "client",
            "--bind",
            &addr,
            "--target",
            &addr,
            "--payload-size",
            "64",
            "--warmup-ms",
            "0",
            "--measure-ms",
            "20",
            "--requests",
            "1",
            "--instrumentation",
            "on",
            "--run-id",
            &run_id,
            "--accelerated-path",
            "idxd",
            "--accelerator-device",
            device_path,
            "--json-out",
            client_json.to_str().expect("utf8 path"),
        ])
        .output()
        .expect("spawn tonic-profile idxd client");

    assert!(
        client_output.status.success(),
        "idxd client run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&client_output.stdout),
        String::from_utf8_lossy(&client_output.stderr)
    );

    wait_for_exit(&mut server, Duration::from_secs(5));
    let server_status = server.wait().expect("wait for server status");
    assert!(
        server_status.success(),
        "server exited with {server_status}"
    );

    let client_report = parse_report(&client_json);
    let server_report = parse_report(&server_json);

    assert_eq!(client_report["metadata"]["run_id"], run_id);
    assert_eq!(server_report["metadata"]["run_id"], run_id);
    assert_eq!(
        client_report["metadata"]["workload_label"],
        server_report["metadata"]["workload_label"]
    );
    assert_idxd_metadata(&client_report, "client", device_path);
    assert_idxd_metadata(&server_report, "server", device_path);
}
