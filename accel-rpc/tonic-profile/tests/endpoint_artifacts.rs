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

fn placeholder_only(report: &Value) -> bool {
    [
        "encode",
        "decode",
        "compress",
        "decompress",
        "buffer_reserve",
        "body_accum",
        "frame_header",
    ]
    .into_iter()
    .all(|stage| {
        report["stages"][stage]["count"].as_u64().unwrap() == 0
            && report["stages"][stage]["nanos"].as_u64().unwrap() == 0
            && report["stages"][stage]["bytes"].as_u64().unwrap() == 0
    })
}

#[test]
fn split_client_and_server_runs_emit_pairable_endpoint_artifacts() {
    let addr = reserve_addr();
    let server_json = unique_path("endpoint-server");
    let client_json = unique_path("endpoint-client");
    let run_id = format!("run-endpoint-{}", std::process::id());

    let mut server = Command::new(tonic_profile_bin())
        .args([
            "--mode",
            "server",
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
            "--server-json-out",
            server_json.to_str().expect("utf8 path"),
        ])
        .spawn()
        .expect("spawn tonic-profile server");

    wait_for_port(&addr, Duration::from_secs(5));

    let client_output = Command::new(tonic_profile_bin())
        .args([
            "--mode",
            "client",
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
            "0",
            "--measure-ms",
            "20",
            "--requests",
            "1",
            "--instrumentation",
            "on",
            "--run-id",
            &run_id,
            "--json-out",
            client_json.to_str().expect("utf8 path"),
        ])
        .output()
        .expect("spawn tonic-profile client");

    assert!(
        client_output.status.success(),
        "client run failed\nstdout:\n{}\nstderr:\n{}",
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

    assert_eq!(client_report["metadata"]["mode"], "client");
    assert_eq!(client_report["metadata"]["endpoint_role"], "client");
    assert_eq!(server_report["metadata"]["mode"], "server");
    assert_eq!(server_report["metadata"]["endpoint_role"], "server");
    assert_eq!(client_report["metadata"]["run_id"], run_id);
    assert_eq!(server_report["metadata"]["run_id"], run_id);
    assert_eq!(
        client_report["metadata"]["workload_label"],
        server_report["metadata"]["workload_label"]
    );
    assert_ne!(
        client_report["metadata"]["endpoint_role"],
        server_report["metadata"]["endpoint_role"]
    );
    assert!(client_report["stages"]["enabled"].as_bool().unwrap());
    assert!(server_report["stages"]["enabled"].as_bool().unwrap());
    assert!(
        !placeholder_only(&client_report),
        "client artifact should contain real stage evidence"
    );
    assert!(
        !placeholder_only(&server_report),
        "server artifact should contain real stage evidence"
    );
}

#[test]
fn split_server_requires_an_explicit_report_path_when_auto_shutting_down() {
    let addr = reserve_addr();
    let output = Command::new(tonic_profile_bin())
        .args([
            "--mode",
            "server",
            "--bind",
            &addr,
            "--target",
            &addr,
            "--shutdown-after-requests",
            "1",
        ])
        .output()
        .expect("spawn invalid tonic-profile server");

    assert!(
        !output.status.success(),
        "server unexpectedly succeeded without a report path\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--shutdown-after-requests requires --server-json-out or --json-out"),
        "stderr should explain the missing split-run report path\nstderr:\n{stderr}"
    );
}
