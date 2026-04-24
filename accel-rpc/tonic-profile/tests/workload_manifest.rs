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
        .join("run_s01_workloads.py")
}

fn tracked_manifest() -> PathBuf {
    repo_root()
        .join("tonic-profile")
        .join("workloads")
        .join("s01_ordinary_matrix.json")
}

fn unique_dir(name: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    let path = env::temp_dir().join(format!(
        "tonic-profile-{name}-{}-{ts}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::write(path, serde_json::to_vec_pretty(value).expect("serialize json")).expect("write json");
}

fn valid_report(label: &str, instrumentation: &str) -> Value {
    json!({
        "metadata": {
            "timestamp_unix_s": 0,
            "mode": "client",
            "rpc": "unary-bytes",
            "ordinary_path": "software",
            "seam": "codec_body",
            "workload_label": label,
            "selection_policy": "echo_payload",
            "request_shape": Value::Null,
            "response_shape": Value::Null,
            "request_serialized_size": 68,
            "response_serialized_size": 66,
            "bind": "127.0.0.1:50051",
            "target": "127.0.0.1:50051",
            "payload_size": 64,
            "payload_kind": "repeated",
            "compression": "off",
            "concurrency": 1,
            "requests_target": 1,
            "warmup_ms": 10,
            "measure_ms": 20,
            "runtime": "single",
            "instrumentation": instrumentation,
            "buffer_policy": "default",
            "effective_codec_buffer_size": Value::Null,
            "effective_codec_yield_threshold": Value::Null,
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
            "encode": {"count": 0, "nanos": 0, "millis": 0.0, "bytes": 0, "avg_nanos": 0.0},
            "decode": {"count": 0, "nanos": 0, "millis": 0.0, "bytes": 0, "avg_nanos": 0.0},
            "compress": {"count": 0, "nanos": 0, "millis": 0.0, "bytes": 0, "avg_nanos": 0.0},
            "decompress": {"count": 0, "nanos": 0, "millis": 0.0, "bytes": 0, "avg_nanos": 0.0},
            "buffer_reserve": {"count": 0, "nanos": 0, "millis": 0.0, "bytes": 0, "avg_nanos": 0.0},
            "body_accum": {"count": 0, "nanos": 0, "millis": 0.0, "bytes": 0, "avg_nanos": 0.0},
            "frame_header": {"count": 0, "nanos": 0, "millis": 0.0, "bytes": 0, "avg_nanos": 0.0}
        }
    })
}

#[test]
fn tracked_manifest_covers_bytes_smoke_and_response_heavy_boundary() {
    let raw = fs::read_to_string(tracked_manifest()).expect("read tracked manifest");
    let manifest: Value = serde_json::from_str(&raw).expect("parse tracked manifest");
    let workloads = manifest["workloads"]
        .as_array()
        .expect("tracked workloads array");

    assert!(
        workloads.iter().any(|entry| {
            entry["label"] == "ordinary/unary-bytes/repeated-64"
                && entry["rpc"] == "unary-bytes"
                && entry["payload_size"] == 64
                && entry["payload_kind"] == "repeated"
                && entry["artifact_pair"].get("on").is_some()
                && entry["artifact_pair"].get("off").is_some()
        }),
        "tracked manifest should include the smallest bytes-mode smoke workload with an on/off pair"
    );

    assert!(
        workloads.iter().any(|entry| {
            entry["label"] == "ordinary/unary-proto-shape/fleet-small-to-fleet-response-heavy"
                && entry["rpc"] == "unary-proto-shape"
                && entry["proto_shape"] == "fleet-small"
                && entry["response_shape"] == "fleet-response-heavy"
                && entry["artifact_pair"].get("on").is_some()
                && entry["artifact_pair"].get("off").is_some()
        }),
        "tracked manifest should include the response-heavy proto-shape boundary workload with an on/off pair"
    );
}

#[test]
fn validate_only_rejects_manifest_entries_missing_pair_definitions() {
    let temp_dir = unique_dir("manifest-missing-pair");
    let manifest_path = temp_dir.join("broken.json");
    write_json(
        &manifest_path,
        &json!({
            "defaults": {
                "warmup_ms": 20,
                "measure_ms": 40,
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
                    "payload_kind": "repeated"
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
        stderr.contains("ordinary/unary-bytes/repeated-64") && stderr.contains("artifact_pair"),
        "stderr should name the offending workload label and missing pair field\nstderr:\n{stderr}"
    );
}

#[test]
fn verify_only_rejects_missing_instrumentation_pair_artifacts() {
    let temp_dir = unique_dir("manifest-missing-artifact");
    let manifest_path = temp_dir.join("manifest.json");
    let output_dir = temp_dir.join("artifacts");
    fs::create_dir_all(&output_dir).expect("create output dir");

    write_json(
        &manifest_path,
        &json!({
            "defaults": {
                "warmup_ms": 20,
                "measure_ms": 40,
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
                    "artifact_pair": {
                        "off": "missing.off.json",
                        "on": "present.on.json"
                    }
                }
            ]
        }),
    );
    write_json(
        &output_dir.join("present.on.json"),
        &valid_report("ordinary/unary-bytes/repeated-64", "on"),
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
            && stderr.contains("instrumentation=off")
            && stderr.contains("missing.off.json"),
        "stderr should identify the missing instrumentation pair artifact\nstderr:\n{stderr}"
    );
}
