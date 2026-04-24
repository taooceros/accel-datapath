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

fn s03_runner_script() -> PathBuf {
    repo_root()
        .join("tonic-profile")
        .join("scripts")
        .join("run_s03_idxd_evidence.py")
}

fn s03_tracked_manifest() -> PathBuf {
    repo_root()
        .join("tonic-profile")
        .join("workloads")
        .join("s03_idxd_matrix.json")
}

fn valid_idxd_report(label: &str, endpoint_role: &str, run_id: &str, device_path: &str, encode_bytes: u64) -> Value {
    json!({
        "metadata": {
            "timestamp_unix_s": 0,
            "mode": endpoint_role,
            "endpoint_role": endpoint_role,
            "run_id": run_id,
            "rpc": if label.contains("unary-bytes") { "unary-bytes" } else { "unary-proto-shape" },
            "ordinary_path": "software",
            "selected_path": "idxd",
            "seam": "codec_body",
            "workload_label": label,
            "selection_policy": if label.contains("unary-bytes") { "echo_payload" } else { "explicit_response" },
            "request_shape": if label.contains("fleet-small-to-fleet-response-heavy") { json!("fleet-small") } else { Value::Null },
            "response_shape": if label.contains("fleet-small-to-fleet-response-heavy") { json!("fleet-response-heavy") } else { Value::Null },
            "request_serialized_size": if label.contains("unary-bytes") { 69 } else { 329 },
            "response_serialized_size": if label.contains("unary-bytes") { 69 } else { 2630 },
            "bind": "127.0.0.1:50051",
            "target": "127.0.0.1:50051",
            "payload_size": if label.contains("unary-bytes") { json!(64) } else { Value::Null },
            "payload_kind": if label.contains("unary-bytes") { json!("repeated") } else { Value::Null },
            "compression": "off",
            "concurrency": 1,
            "requests_target": 1,
            "warmup_ms": 0,
            "measure_ms": 20,
            "runtime": "single",
            "instrumentation": "on",
            "accelerated_device_path": device_path,
            "accelerated_lane": "codec_memmove",
            "accelerated_direction": "bidirectional",
            "buffer_policy": "default",
            "effective_codec_buffer_size": 8192,
            "effective_codec_yield_threshold": 32768,
            "server_core": Value::Null,
            "client_core": Value::Null
        },
        "metrics": {
            "requests_completed": 1,
            "bytes_sent": encode_bytes,
            "bytes_received": encode_bytes,
            "duration_ms": 1.0,
            "throughput_rps": 1.0,
            "throughput_mib_s": 1.0,
            "latency_us_p50": 1,
            "latency_us_p95": 1,
            "latency_us_p99": 1,
            "latency_us_max": 1
        },
        "stages": {
            "enabled": true,
            "encode": {"count": 1, "nanos": 10, "millis": 0.0, "bytes": encode_bytes, "avg_nanos": 10.0},
            "decode": {"count": 1, "nanos": 10, "millis": 0.0, "bytes": encode_bytes, "avg_nanos": 10.0},
            "compress": {"count": 0, "nanos": 0, "millis": 0.0, "bytes": 0, "avg_nanos": 0.0},
            "decompress": {"count": 0, "nanos": 0, "millis": 0.0, "bytes": 0, "avg_nanos": 0.0},
            "buffer_reserve": {"count": 1, "nanos": 10, "millis": 0.0, "bytes": encode_bytes, "avg_nanos": 10.0},
            "body_accum": {"count": 1, "nanos": 10, "millis": 0.0, "bytes": encode_bytes, "avg_nanos": 10.0},
            "frame_header": {"count": 1, "nanos": 10, "millis": 0.0, "bytes": 15, "avg_nanos": 10.0}
        }
    })
}

#[test]
fn tracked_s03_manifest_covers_boundary_workloads_and_accelerated_expectations() {
    let raw = fs::read_to_string(s03_tracked_manifest()).expect("read tracked s03 manifest");
    let manifest: Value = serde_json::from_str(&raw).expect("parse tracked s03 manifest");

    assert_eq!(manifest["expected_metadata"]["ordinary_path"], "software");
    assert_eq!(manifest["expected_metadata"]["selected_path"], "idxd");
    assert_eq!(manifest["expected_metadata"]["accelerated_lane"], "codec_memmove");
    assert_eq!(manifest["expected_metadata"]["accelerated_direction"], "bidirectional");
    assert_eq!(manifest["expected_metadata"]["require_device_path"], true);

    let workloads = manifest["workloads"]
        .as_array()
        .expect("tracked workloads array");

    assert!(
        workloads.iter().any(|entry| {
            entry["label"] == "ordinary/unary-bytes/repeated-64"
                && entry["rpc"] == "unary-bytes"
                && entry["payload_size"] == 64
                && entry["payload_kind"] == "repeated"
                && entry["endpoint_artifacts"]["client"] == "idxd__unary-bytes__repeated-64.client.json"
                && entry["endpoint_artifacts"]["server"] == "idxd__unary-bytes__repeated-64.server.json"
        }),
        "tracked s03 manifest should include the tiny bytes workload with explicit client/server artifacts"
    );

    assert!(
        workloads.iter().any(|entry| {
            entry["label"] == "ordinary/unary-proto-shape/fleet-small-to-fleet-response-heavy"
                && entry["rpc"] == "unary-proto-shape"
                && entry["proto_shape"] == "fleet-small"
                && entry["response_shape"] == "fleet-response-heavy"
                && entry["endpoint_artifacts"]["client"]
                    == "idxd__unary-proto-shape__fleet-small-to-fleet-response-heavy.client.json"
                && entry["endpoint_artifacts"]["server"]
                    == "idxd__unary-proto-shape__fleet-small-to-fleet-response-heavy.server.json"
        }),
        "tracked s03 manifest should include the response-heavy boundary workload with explicit client/server artifacts"
    );
}

#[test]
fn s03_validate_only_rejects_missing_expected_accelerated_metadata() {
    let temp_dir = unique_dir("s03-missing-expected-metadata");
    let manifest_path = temp_dir.join("broken.json");
    write_json(
        &manifest_path,
        &json!({
            "defaults": {
                "warmup_ms": 0,
                "measure_ms": 20,
                "requests": 1,
                "concurrency": 1,
                "runtime": "single",
                "compression": "off",
                "buffer_policy": "default",
                "instrumentation": "on",
                "accelerated_path": "idxd"
            },
            "expected_metadata": {
                "ordinary_path": "software",
                "selected_path": "idxd",
                "seam": "codec_body",
                "accelerated_lane": "codec_memmove"
            },
            "workloads": [
                {
                    "label": "ordinary/unary-bytes/repeated-64",
                    "rpc": "unary-bytes",
                    "payload_size": 64,
                    "payload_kind": "repeated",
                    "endpoint_artifacts": {
                        "client": "idxd__unary-bytes__repeated-64.client.json",
                        "server": "idxd__unary-bytes__repeated-64.server.json"
                    }
                },
                {
                    "label": "ordinary/unary-proto-shape/fleet-small-to-fleet-response-heavy",
                    "rpc": "unary-proto-shape",
                    "proto_shape": "fleet-small",
                    "response_shape": "fleet-response-heavy",
                    "endpoint_artifacts": {
                        "client": "idxd__unary-proto-shape__fleet-small-to-fleet-response-heavy.client.json",
                        "server": "idxd__unary-proto-shape__fleet-small-to-fleet-response-heavy.server.json"
                    }
                }
            ]
        }),
    );

    let output = Command::new("python3")
        .arg(s03_runner_script())
        .arg("--manifest")
        .arg(&manifest_path)
        .arg("--validate-only")
        .output()
        .expect("run s03 workload runner validate-only");

    assert!(!output.status.success(), "validate-only unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("expected_metadata")
            && (stderr.contains("accelerated_direction") || stderr.contains("require_device_path")),
        "stderr should identify the missing accelerated metadata expectation\nstderr:\n{stderr}"
    );
}

#[test]
fn s03_validate_only_rejects_manifest_entries_missing_server_artifacts() {
    let temp_dir = unique_dir("s03-missing-server-artifact");
    let manifest_path = temp_dir.join("broken.json");
    write_json(
        &manifest_path,
        &json!({
            "defaults": {
                "warmup_ms": 0,
                "measure_ms": 20,
                "requests": 1,
                "concurrency": 1,
                "runtime": "single",
                "compression": "off",
                "buffer_policy": "default",
                "instrumentation": "on",
                "accelerated_path": "idxd"
            },
            "expected_metadata": {
                "ordinary_path": "software",
                "selected_path": "idxd",
                "seam": "codec_body",
                "accelerated_lane": "codec_memmove",
                "accelerated_direction": "bidirectional",
                "require_device_path": true
            },
            "workloads": [
                {
                    "label": "ordinary/unary-bytes/repeated-64",
                    "rpc": "unary-bytes",
                    "payload_size": 64,
                    "payload_kind": "repeated",
                    "endpoint_artifacts": {
                        "client": "idxd__unary-bytes__repeated-64.client.json"
                    }
                },
                {
                    "label": "ordinary/unary-proto-shape/fleet-small-to-fleet-response-heavy",
                    "rpc": "unary-proto-shape",
                    "proto_shape": "fleet-small",
                    "response_shape": "fleet-response-heavy",
                    "endpoint_artifacts": {
                        "client": "idxd__unary-proto-shape__fleet-small-to-fleet-response-heavy.client.json",
                        "server": "idxd__unary-proto-shape__fleet-small-to-fleet-response-heavy.server.json"
                    }
                }
            ]
        }),
    );

    let output = Command::new("python3")
        .arg(s03_runner_script())
        .arg("--manifest")
        .arg(&manifest_path)
        .arg("--validate-only")
        .output()
        .expect("run s03 workload runner validate-only");

    assert!(!output.status.success(), "validate-only unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ordinary/unary-bytes/repeated-64")
            && stderr.contains("endpoint_artifacts")
            && stderr.contains("server"),
        "stderr should name the offending workload label and missing server artifact\nstderr:\n{stderr}"
    );
}

#[test]
fn s03_verify_only_rejects_software_looking_accelerated_artifacts() {
    let temp_dir = unique_dir("s03-software-looking-artifacts");
    let manifest_path = temp_dir.join("manifest.json");
    let output_dir = temp_dir.join("artifacts");
    fs::create_dir_all(&output_dir).expect("create output dir");
    let device_path = "/dev/dsa/wq0.0";

    write_json(
        &manifest_path,
        &json!({
            "defaults": {
                "warmup_ms": 0,
                "measure_ms": 20,
                "requests": 1,
                "concurrency": 1,
                "runtime": "single",
                "compression": "off",
                "buffer_policy": "default",
                "instrumentation": "on",
                "accelerated_path": "idxd"
            },
            "expected_metadata": {
                "ordinary_path": "software",
                "selected_path": "idxd",
                "seam": "codec_body",
                "accelerated_lane": "codec_memmove",
                "accelerated_direction": "bidirectional",
                "require_device_path": true
            },
            "workloads": [
                {
                    "label": "ordinary/unary-bytes/repeated-64",
                    "rpc": "unary-bytes",
                    "payload_size": 64,
                    "payload_kind": "repeated",
                    "endpoint_artifacts": {
                        "client": "idxd__unary-bytes__repeated-64.client.json",
                        "server": "idxd__unary-bytes__repeated-64.server.json"
                    }
                },
                {
                    "label": "ordinary/unary-proto-shape/fleet-small-to-fleet-response-heavy",
                    "rpc": "unary-proto-shape",
                    "proto_shape": "fleet-small",
                    "response_shape": "fleet-response-heavy",
                    "endpoint_artifacts": {
                        "client": "idxd__unary-proto-shape__fleet-small-to-fleet-response-heavy.client.json",
                        "server": "idxd__unary-proto-shape__fleet-small-to-fleet-response-heavy.server.json"
                    }
                }
            ]
        }),
    );

    let mut bad_client = valid_idxd_report(
        "ordinary/unary-bytes/repeated-64",
        "client",
        "run-on-bytes",
        device_path,
        64,
    );
    bad_client["metadata"]["selected_path"] = json!("software");
    write_json(
        &output_dir.join("idxd__unary-bytes__repeated-64.client.json"),
        &bad_client,
    );
    write_json(
        &output_dir.join("idxd__unary-bytes__repeated-64.server.json"),
        &valid_idxd_report(
            "ordinary/unary-bytes/repeated-64",
            "server",
            "run-on-bytes",
            device_path,
            64,
        ),
    );
    write_json(
        &output_dir.join("idxd__unary-proto-shape__fleet-small-to-fleet-response-heavy.client.json"),
        &valid_idxd_report(
            "ordinary/unary-proto-shape/fleet-small-to-fleet-response-heavy",
            "client",
            "run-on-proto",
            device_path,
            240,
        ),
    );
    write_json(
        &output_dir.join("idxd__unary-proto-shape__fleet-small-to-fleet-response-heavy.server.json"),
        &valid_idxd_report(
            "ordinary/unary-proto-shape/fleet-small-to-fleet-response-heavy",
            "server",
            "run-on-proto",
            device_path,
            260,
        ),
    );

    let output = Command::new("python3")
        .arg(s03_runner_script())
        .arg("--manifest")
        .arg(&manifest_path)
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--accelerator-device")
        .arg(device_path)
        .arg("--verify-only")
        .output()
        .expect("run s03 workload runner verify-only");

    assert!(!output.status.success(), "verify-only unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ordinary/unary-bytes/repeated-64")
            && stderr.contains("selected_path")
            && stderr.contains("software"),
        "stderr should identify the software-looking accelerated artifact\nstderr:\n{stderr}"
    );
}

fn s04_runner_script() -> PathBuf {
    repo_root()
        .join("tonic-profile")
        .join("scripts")
        .join("run_s04_claim_package.py")
}

fn summarizer_script() -> PathBuf {
    repo_root()
        .join("tonic-profile")
        .join("scripts")
        .join("summarize_s04_claim_package.py")
}

fn proof_runner_common_script() -> PathBuf {
    repo_root()
        .join("tonic-profile")
        .join("scripts")
        .join("proof_runner_common.py")
}

fn claim_package_contract_script() -> PathBuf {
    repo_root()
        .join("tonic-profile")
        .join("scripts")
        .join("claim_package_contract.py")
}

fn s04_tracked_manifest() -> PathBuf {
    repo_root()
        .join("tonic-profile")
        .join("workloads")
        .join("s04_claim_package.json")
}

#[test]
fn tracked_s04_manifest_covers_required_families_outputs_and_report_references() {
    let raw = fs::read_to_string(s04_tracked_manifest()).expect("read tracked s04 manifest");
    let manifest: Value = serde_json::from_str(&raw).expect("parse tracked s04 manifest");

    assert_eq!(manifest["run_root"], "accel-rpc/target/s04-claim-package/latest");
    assert_eq!(
        manifest["scope"]["pairing_keys"],
        json!(["workload_label", "endpoint_role", "run_family"])
    );
    assert_eq!(
        manifest["scope"]["workload_labels"],
        json!([
            "ordinary/unary-bytes/repeated-64",
            "ordinary/unary-proto-shape/fleet-small-to-fleet-response-heavy"
        ])
    );
    assert_eq!(
        manifest["inputs"]["software_manifest"],
        "accel-rpc/tonic-profile/workloads/s02_trustworthy_matrix.json"
    );
    assert_eq!(
        manifest["inputs"]["idxd_manifest"],
        "accel-rpc/tonic-profile/workloads/s03_idxd_matrix.json"
    );
    assert_eq!(
        manifest["inputs"]["control_floor_summary"],
        "accel-rpc/target/control-floor/async_control_floor_summary.json"
    );
    assert_eq!(
        manifest["inputs"]["report_contract"],
        "accel-rpc/tonic-profile/tests/report_contract.rs"
    );

    let families = manifest["artifact_families"]
        .as_array()
        .expect("tracked s04 artifact_families array");
    assert_eq!(families.len(), 3, "s04 should freeze three comparison families");

    for (run_family, instrumentation, selected_path, source_manifest, expected_prefix) in [
        (
            "software_baseline",
            "off",
            "software",
            "accel-rpc/tonic-profile/workloads/s02_trustworthy_matrix.json",
            "software/",
        ),
        (
            "software_attribution",
            "on",
            "software",
            "accel-rpc/tonic-profile/workloads/s02_trustworthy_matrix.json",
            "software/",
        ),
        (
            "idxd_attribution",
            "on",
            "idxd",
            "accel-rpc/tonic-profile/workloads/s03_idxd_matrix.json",
            "idxd/",
        ),
    ] {
        let family = families
            .iter()
            .find(|entry| entry["run_family"] == run_family)
            .unwrap_or_else(|| panic!("missing run_family {run_family}"));
        assert_eq!(family["instrumentation"], instrumentation);
        assert_eq!(family["selected_path"], selected_path);
        assert_eq!(family["source_manifest"], source_manifest);

        let reports = family["endpoint_reports"]
            .as_array()
            .expect("endpoint_reports array");
        assert_eq!(reports.len(), 4, "{run_family} should enumerate client/server artifacts for both workloads");
        for label in [
            "ordinary/unary-bytes/repeated-64",
            "ordinary/unary-proto-shape/fleet-small-to-fleet-response-heavy",
        ] {
            for endpoint_role in ["client", "server"] {
                let report = reports
                    .iter()
                    .find(|entry| {
                        entry["workload_label"] == label && entry["endpoint_role"] == endpoint_role
                    })
                    .unwrap_or_else(|| {
                        panic!("missing {run_family} artifact for {label} {endpoint_role}")
                    });
                let artifact = report["artifact"].as_str().expect("artifact string");
                assert!(
                    artifact.starts_with(expected_prefix),
                    "{run_family} artifact should live under {expected_prefix}: {artifact}"
                );
            }
        }
    }

    assert_eq!(
        manifest["derived_outputs"],
        json!({
            "comparison_summary_json": "summary/comparison_summary.json",
            "ordinary_vs_idxd_csv": "summary/ordinary_vs_idxd.csv",
            "claim_table_md": "summary/claim_table.md"
        })
    );
    assert_eq!(
        manifest["report"]["path"],
        "docs/report/benchmarking/014.idxd_tonic_same_repo_claim_package.md"
    );
    assert_eq!(
        manifest["report"]["required_references"],
        json!([
            "accel-rpc/target/s04-claim-package/latest/summary/comparison_summary.json",
            "accel-rpc/target/s04-claim-package/latest/summary/ordinary_vs_idxd.csv",
            "accel-rpc/target/s04-claim-package/latest/summary/claim_table.md"
        ])
    );
}

#[test]
fn extracted_helper_modules_exist_and_summarizer_imports_only_contract_helpers() {
    let proof_runner_common = proof_runner_common_script();
    let claim_package_contract = claim_package_contract_script();
    let summarizer = summarizer_script();

    assert!(proof_runner_common.exists(), "missing shared proof runner helper: {:?}", proof_runner_common);
    assert!(claim_package_contract.exists(), "missing claim package contract helper: {:?}", claim_package_contract);

    let summarizer_text = fs::read_to_string(&summarizer).expect("read s04 summarizer");
    assert!(
        summarizer_text.contains("import claim_package_contract as claim_contract"),
        "summarizer should import the extracted claim package contract helper"
    );
    assert!(
        !summarizer_text.contains("import run_s04_claim_package"),
        "summarizer should not import the s04 orchestration runner directly"
    );
}

#[test]
fn s04_validate_only_accepts_the_tracked_manifest() {
    let output = Command::new("python3")
        .arg(s04_runner_script())
        .arg("--manifest")
        .arg(s04_tracked_manifest())
        .arg("--validate-only")
        .output()
        .expect("run s04 claim-package validate-only");

    assert!(
        output.status.success(),
        "validate-only failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("phase=manifest-parse")
            && stdout.contains("run_root=")
            && stdout.contains("accel-rpc/target/s04-claim-package/latest")
            && stdout.contains("summary_path=")
            && stdout.contains(
                "accel-rpc/target/s04-claim-package/latest/summary/comparison_summary.json"
            ),
        "validate-only should print the tracked run-root and summary contract\nstdout:\n{stdout}"
    );
}
