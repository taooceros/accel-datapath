use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn runner_script() -> PathBuf {
    repo_root()
        .join("tonic-profile")
        .join("scripts")
        .join("run_s02_evidence.py")
}

fn tracked_manifest() -> PathBuf {
    repo_root()
        .join("tonic-profile")
        .join("workloads")
        .join("s02_trustworthy_matrix.json")
}

fn unique_dir(name: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    let path = env::temp_dir().join(format!("tonic-profile-{name}-{}-{ts}", std::process::id()));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(value).expect("serialize json"),
    )
    .expect("write json");
}

fn valid_report(
    label: &str,
    instrumentation: &str,
    endpoint_role: &str,
    run_id: &str,
    encode_bytes: u64,
) -> Value {
    let has_stage_evidence = instrumentation == "on" && encode_bytes > 0;
    json!({
        "metadata": {
            "timestamp_unix_s": 0,
            "mode": endpoint_role,
            "endpoint_role": endpoint_role,
            "run_id": run_id,
            "rpc": if label.contains("unary-bytes") { "unary-bytes" } else { "unary-proto-shape" },
            "ordinary_path": "software",
            "seam": "codec_body",
            "workload_label": label,
            "selection_policy": "echo_payload",
            "request_shape": if label.contains("fleet-small-to-fleet-response-heavy") { json!("fleet-small") } else { Value::Null },
            "response_shape": if label.contains("fleet-small-to-fleet-response-heavy") { json!("fleet-response-heavy") } else { Value::Null },
            "request_serialized_size": 68,
            "response_serialized_size": 66,
            "bind": "127.0.0.1:50051",
            "target": "127.0.0.1:50051",
            "payload_size": if label.contains("unary-bytes") { json!(64) } else { Value::Null },
            "payload_kind": if label.contains("unary-bytes") { json!("repeated") } else { Value::Null },
            "compression": "off",
            "concurrency": 1,
            "requests_target": 1,
            "warmup_ms": 10,
            "measure_ms": 20,
            "runtime": "single",
            "instrumentation": instrumentation,
            "buffer_policy": "default",
            "effective_codec_buffer_size": 8192,
            "effective_codec_yield_threshold": 32768,
            "server_core": Value::Null,
            "client_core": Value::Null
        },
        "metrics": {
            "requests_completed": 1,
            "bytes_sent": 68,
            "bytes_received": 66,
            "duration_ms": 1.0,
            "throughput_rps": 1.0,
            "throughput_mib_s": 1.0,
            "latency_us_p50": 1,
            "latency_us_p95": 1,
            "latency_us_p99": 1,
            "latency_us_max": 1
        },
        "stages": {
            "enabled": instrumentation == "on",
            "encode": {"count": if has_stage_evidence { 1 } else { 0 }, "nanos": if has_stage_evidence { 10 } else { 0 }, "millis": 0.0, "bytes": if has_stage_evidence { encode_bytes } else { 0 }, "avg_nanos": 0.0},
            "decode": {"count": if has_stage_evidence { 1 } else { 0 }, "nanos": if has_stage_evidence { 10 } else { 0 }, "millis": 0.0, "bytes": if has_stage_evidence { encode_bytes } else { 0 }, "avg_nanos": 0.0},
            "compress": {"count": 0, "nanos": 0, "millis": 0.0, "bytes": 0, "avg_nanos": 0.0},
            "decompress": {"count": 0, "nanos": 0, "millis": 0.0, "bytes": 0, "avg_nanos": 0.0},
            "buffer_reserve": {"count": if has_stage_evidence { 1 } else { 0 }, "nanos": if has_stage_evidence { 10 } else { 0 }, "millis": 0.0, "bytes": if has_stage_evidence { encode_bytes } else { 0 }, "avg_nanos": 0.0},
            "body_accum": {"count": if has_stage_evidence { 1 } else { 0 }, "nanos": if has_stage_evidence { 10 } else { 0 }, "millis": 0.0, "bytes": if has_stage_evidence { encode_bytes } else { 0 }, "avg_nanos": 0.0},
            "frame_header": {"count": if has_stage_evidence { 1 } else { 0 }, "nanos": if has_stage_evidence { 10 } else { 0 }, "millis": 0.0, "bytes": if has_stage_evidence { 15 } else { 0 }, "avg_nanos": 0.0}
        }
    })
}

#[test]
fn tracked_manifest_covers_boundary_workloads_and_expected_benchmarks() {
    let raw = fs::read_to_string(tracked_manifest()).expect("read tracked manifest");
    let manifest: Value = serde_json::from_str(&raw).expect("parse tracked manifest");

    let expected_benchmarks = manifest["expected_benchmarks"]
        .as_array()
        .expect("expected_benchmarks array");
    for benchmark in [
        "tokio_spawn_join",
        "tokio_oneshot_completion",
        "tokio_mpsc_round_trip",
        "tokio_same_thread_wake",
        "tokio_cross_thread_wake",
    ] {
        assert!(
            expected_benchmarks.iter().any(|entry| entry == benchmark),
            "tracked manifest should include benchmark {benchmark}"
        );
    }

    let workloads = manifest["workloads"]
        .as_array()
        .expect("tracked workloads array");

    assert!(
        workloads.iter().any(|entry| {
            entry["label"] == "ordinary/unary-bytes/repeated-64"
                && entry["rpc"] == "unary-bytes"
                && entry["payload_size"] == 64
                && entry["payload_kind"] == "repeated"
                && entry["endpoint_artifacts"]["on"]["client"].is_string()
                && entry["endpoint_artifacts"]["on"]["server"].is_string()
                && entry["endpoint_artifacts"]["off"]["client"].is_string()
                && entry["endpoint_artifacts"]["off"]["server"].is_string()
        }),
        "tracked manifest should include the tiny bytes workload with explicit client/server artifacts"
    );

    assert!(
        workloads.iter().any(|entry| {
            entry["label"] == "ordinary/unary-proto-shape/fleet-small-to-fleet-response-heavy"
                && entry["rpc"] == "unary-proto-shape"
                && entry["proto_shape"] == "fleet-small"
                && entry["response_shape"] == "fleet-response-heavy"
                && entry["endpoint_artifacts"]["on"]["client"].is_string()
                && entry["endpoint_artifacts"]["on"]["server"].is_string()
                && entry["endpoint_artifacts"]["off"]["client"].is_string()
                && entry["endpoint_artifacts"]["off"]["server"].is_string()
        }),
        "tracked manifest should include the response-heavy boundary workload with explicit client/server artifacts"
    );
}

#[test]
fn validate_only_rejects_missing_expected_benchmark_ids() {
    let temp_dir = unique_dir("manifest-missing-benchmarks");
    let manifest_path = temp_dir.join("broken.json");
    write_json(
        &manifest_path,
        &json!({
            "defaults": {
                "warmup_ms": 10,
                "measure_ms": 20,
                "requests": 1,
                "concurrency": 1,
                "runtime": "single",
                "compression": "off",
                "buffer_policy": "default"
            },
            "workloads": [
                {
                    "label": "ordinary/unary-bytes/repeated-64",
                    "rpc": "unary-bytes",
                    "payload_size": 64,
                    "payload_kind": "repeated",
                    "endpoint_artifacts": {
                        "off": {"client": "a.off.client.json", "server": "a.off.server.json"},
                        "on": {"client": "a.on.client.json", "server": "a.on.server.json"}
                    }
                },
                {
                    "label": "ordinary/unary-proto-shape/fleet-small-to-fleet-response-heavy",
                    "rpc": "unary-proto-shape",
                    "proto_shape": "fleet-small",
                    "response_shape": "fleet-response-heavy",
                    "endpoint_artifacts": {
                        "off": {"client": "b.off.client.json", "server": "b.off.server.json"},
                        "on": {"client": "b.on.client.json", "server": "b.on.server.json"}
                    }
                }
            ]
        }),
    );

    let output = Command::new("python3")
        .arg(runner_script())
        .arg("--manifest")
        .arg(&manifest_path)
        .arg("--validate-only")
        .output()
        .expect("run workload runner validate-only");

    assert!(
        !output.status.success(),
        "validate-only unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("expected_benchmarks"),
        "stderr should identify the missing expected benchmark list\nstderr:\n{stderr}"
    );
}

#[test]
fn validate_only_rejects_manifest_entries_missing_server_artifacts() {
    let temp_dir = unique_dir("manifest-missing-server-artifact");
    let manifest_path = temp_dir.join("broken.json");
    write_json(
        &manifest_path,
        &json!({
            "expected_benchmarks": [
                "tokio_spawn_join",
                "tokio_oneshot_completion",
                "tokio_mpsc_round_trip",
                "tokio_same_thread_wake",
                "tokio_cross_thread_wake"
            ],
            "defaults": {
                "warmup_ms": 10,
                "measure_ms": 20,
                "requests": 1,
                "concurrency": 1,
                "runtime": "single",
                "compression": "off",
                "buffer_policy": "default"
            },
            "workloads": [
                {
                    "label": "ordinary/unary-bytes/repeated-64",
                    "rpc": "unary-bytes",
                    "payload_size": 64,
                    "payload_kind": "repeated",
                    "endpoint_artifacts": {
                        "off": {"client": "a.off.client.json", "server": "a.off.server.json"},
                        "on": {"client": "a.on.client.json"}
                    }
                },
                {
                    "label": "ordinary/unary-proto-shape/fleet-small-to-fleet-response-heavy",
                    "rpc": "unary-proto-shape",
                    "proto_shape": "fleet-small",
                    "response_shape": "fleet-response-heavy",
                    "endpoint_artifacts": {
                        "off": {"client": "b.off.client.json", "server": "b.off.server.json"},
                        "on": {"client": "b.on.client.json", "server": "b.on.server.json"}
                    }
                }
            ]
        }),
    );

    let output = Command::new("python3")
        .arg(runner_script())
        .arg("--manifest")
        .arg(&manifest_path)
        .arg("--validate-only")
        .output()
        .expect("run workload runner validate-only");

    assert!(
        !output.status.success(),
        "validate-only unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ordinary/unary-bytes/repeated-64")
            && stderr.contains("endpoint_artifacts")
            && stderr.contains("server"),
        "stderr should name the offending workload label and missing server artifact\nstderr:\n{stderr}"
    );
}

#[test]
fn verify_only_rejects_placeholder_only_instrumentation_on_artifacts() {
    let temp_dir = unique_dir("manifest-placeholder-only");
    let manifest_path = temp_dir.join("manifest.json");
    let output_dir = temp_dir.join("artifacts");
    fs::create_dir_all(&output_dir).expect("create output dir");

    write_json(
        &manifest_path,
        &json!({
            "expected_benchmarks": [
                "tokio_spawn_join",
                "tokio_oneshot_completion",
                "tokio_mpsc_round_trip",
                "tokio_same_thread_wake",
                "tokio_cross_thread_wake"
            ],
            "defaults": {
                "warmup_ms": 10,
                "measure_ms": 20,
                "requests": 1,
                "concurrency": 1,
                "runtime": "single",
                "compression": "off",
                "buffer_policy": "default"
            },
            "workloads": [
                {
                    "label": "ordinary/unary-bytes/repeated-64",
                    "rpc": "unary-bytes",
                    "payload_size": 64,
                    "payload_kind": "repeated",
                    "endpoint_artifacts": {
                        "off": {
                            "client": "ordinary__unary-bytes__repeated-64.client.off.json",
                            "server": "ordinary__unary-bytes__repeated-64.server.off.json"
                        },
                        "on": {
                            "client": "ordinary__unary-bytes__repeated-64.client.on.json",
                            "server": "ordinary__unary-bytes__repeated-64.server.on.json"
                        }
                    }
                },
                {
                    "label": "ordinary/unary-proto-shape/fleet-small-to-fleet-response-heavy",
                    "rpc": "unary-proto-shape",
                    "proto_shape": "fleet-small",
                    "response_shape": "fleet-response-heavy",
                    "endpoint_artifacts": {
                        "off": {
                            "client": "ordinary__unary-proto-shape__fleet-small-to-fleet-response-heavy.client.off.json",
                            "server": "ordinary__unary-proto-shape__fleet-small-to-fleet-response-heavy.server.off.json"
                        },
                        "on": {
                            "client": "ordinary__unary-proto-shape__fleet-small-to-fleet-response-heavy.client.on.json",
                            "server": "ordinary__unary-proto-shape__fleet-small-to-fleet-response-heavy.server.on.json"
                        }
                    }
                }
            ]
        }),
    );

    write_json(
        &output_dir.join("ordinary__unary-bytes__repeated-64.client.off.json"),
        &valid_report(
            "ordinary/unary-bytes/repeated-64",
            "off",
            "client",
            "run-off-bytes",
            0,
        ),
    );
    write_json(
        &output_dir.join("ordinary__unary-bytes__repeated-64.server.off.json"),
        &valid_report(
            "ordinary/unary-bytes/repeated-64",
            "off",
            "server",
            "run-off-bytes",
            0,
        ),
    );
    write_json(
        &output_dir.join("ordinary__unary-bytes__repeated-64.client.on.json"),
        &valid_report(
            "ordinary/unary-bytes/repeated-64",
            "on",
            "client",
            "run-on-bytes",
            0,
        ),
    );
    write_json(
        &output_dir.join("ordinary__unary-bytes__repeated-64.server.on.json"),
        &valid_report(
            "ordinary/unary-bytes/repeated-64",
            "on",
            "server",
            "run-on-bytes",
            0,
        ),
    );
    write_json(
        &output_dir.join(
            "ordinary__unary-proto-shape__fleet-small-to-fleet-response-heavy.client.off.json",
        ),
        &valid_report(
            "ordinary/unary-proto-shape/fleet-small-to-fleet-response-heavy",
            "off",
            "client",
            "run-off-proto",
            0,
        ),
    );
    write_json(
        &output_dir.join(
            "ordinary__unary-proto-shape__fleet-small-to-fleet-response-heavy.server.off.json",
        ),
        &valid_report(
            "ordinary/unary-proto-shape/fleet-small-to-fleet-response-heavy",
            "off",
            "server",
            "run-off-proto",
            0,
        ),
    );
    write_json(
        &output_dir.join(
            "ordinary__unary-proto-shape__fleet-small-to-fleet-response-heavy.client.on.json",
        ),
        &valid_report(
            "ordinary/unary-proto-shape/fleet-small-to-fleet-response-heavy",
            "on",
            "client",
            "run-on-proto",
            200,
        ),
    );
    write_json(
        &output_dir.join(
            "ordinary__unary-proto-shape__fleet-small-to-fleet-response-heavy.server.on.json",
        ),
        &valid_report(
            "ordinary/unary-proto-shape/fleet-small-to-fleet-response-heavy",
            "on",
            "server",
            "run-on-proto",
            220,
        ),
    );

    let output = Command::new("python3")
        .arg(runner_script())
        .arg("--manifest")
        .arg(&manifest_path)
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--verify-only")
        .output()
        .expect("run workload runner verify-only");

    assert!(
        !output.status.success(),
        "verify-only unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ordinary/unary-bytes/repeated-64")
            && stderr.contains("instrumentation=on")
            && stderr.contains("endpoint_role=client")
            && stderr.contains("placeholder-only"),
        "stderr should identify the placeholder-only instrumentation-on artifact\nstderr:\n{stderr}"
    );
}
