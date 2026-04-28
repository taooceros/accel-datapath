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
        .join("run_s04_claim_package.py")
}

fn summarizer_script() -> PathBuf {
    repo_root()
        .join("tonic-profile")
        .join("scripts")
        .join("summarize_s04_claim_package.py")
}

fn tracked_manifest() -> PathBuf {
    repo_root()
        .join("tonic-profile")
        .join("workloads")
        .join("s04_claim_package.json")
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

fn load_manifest_value() -> Value {
    let raw = fs::read_to_string(tracked_manifest()).expect("read tracked s04 manifest");
    serde_json::from_str(&raw).expect("parse tracked s04 manifest")
}

fn run_validate_only(manifest_path: &Path) -> std::process::Output {
    Command::new("python3")
        .arg(runner_script())
        .arg("--manifest")
        .arg(manifest_path)
        .arg("--validate-only")
        .output()
        .expect("run s04 validate-only")
}

fn run_summarizer(
    manifest_path: &Path,
    run_root: &Path,
    verify_only: bool,
) -> std::process::Output {
    let mut command = Command::new("python3");
    command
        .arg(summarizer_script())
        .arg("--manifest")
        .arg(manifest_path)
        .arg("--run-root")
        .arg(run_root);
    if verify_only {
        command.arg("--verify-only");
    }
    command.output().expect("run s04 summarizer")
}

fn run_runner_with_overrides(
    manifest_path: &Path,
    run_root: &Path,
    s02_script: &Path,
    s03_script: &Path,
    summary_script: &Path,
) -> std::process::Output {
    Command::new("python3")
        .arg(runner_script())
        .arg("--manifest")
        .arg(manifest_path)
        .arg("--run-root")
        .arg(run_root)
        .arg("--device-path")
        .arg("/dev/dsa/wq-test")
        .env("S04_VERIFY_S02_PATH", s02_script)
        .env("S04_VERIFY_S03_PATH", s03_script)
        .env("S04_SUMMARIZER_PATH", summary_script)
        .output()
        .expect("run s04 workflow with overrides")
}

fn write_text(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create text parent dir");
    }
    fs::write(path, contents).expect("write text file");
}

fn valid_control_floor_summary() -> Value {
    json!({
        "schema_version": 1,
        "suite_name": "async_control_floor",
        "benchmarks": {
            "tokio_spawn_join": {
                "benchmark_name": "tokio_spawn_join",
                "mean_ns": 100.0,
                "median_ns": 95.0,
                "std_dev_ns": 3.0,
                "sample_count": 10
            },
            "tokio_oneshot_completion": {
                "benchmark_name": "tokio_oneshot_completion",
                "mean_ns": 25.0,
                "median_ns": 24.0,
                "std_dev_ns": 1.0,
                "sample_count": 10
            },
            "tokio_mpsc_round_trip": {
                "benchmark_name": "tokio_mpsc_round_trip",
                "mean_ns": 50.0,
                "median_ns": 48.0,
                "std_dev_ns": 2.0,
                "sample_count": 10
            },
            "tokio_same_thread_wake": {
                "benchmark_name": "tokio_same_thread_wake",
                "mean_ns": 10.0,
                "median_ns": 10.0,
                "std_dev_ns": 0.5,
                "sample_count": 10
            },
            "tokio_cross_thread_wake": {
                "benchmark_name": "tokio_cross_thread_wake",
                "mean_ns": 150.0,
                "median_ns": 145.0,
                "std_dev_ns": 4.0,
                "sample_count": 10
            }
        }
    })
}

fn base_report(
    label: &str,
    instrumentation: &str,
    endpoint_role: &str,
    run_id: &str,
    selected_path: &str,
    throughput_rps: f64,
    latency_us_p50: u64,
    stage_nanos: u64,
    stage_bytes: u64,
) -> Value {
    let (request_shape, response_shape, payload_size, payload_kind) =
        if label.contains("fleet-small-to-fleet-response-heavy") {
            (
                json!("fleet-small"),
                json!("fleet-response-heavy"),
                Value::Null,
                Value::Null,
            )
        } else {
            (Value::Null, Value::Null, json!(64), json!("repeated"))
        };

    let device_path = if selected_path == "idxd" {
        json!("/dev/dsa/wq0.0")
    } else {
        Value::Null
    };
    let accelerated_lane = if selected_path == "idxd" {
        json!("codec_memmove")
    } else {
        Value::Null
    };
    let accelerated_direction = if selected_path == "idxd" {
        json!("bidirectional")
    } else {
        Value::Null
    };

    json!({
        "metadata": {
            "timestamp_unix_s": 0,
            "mode": endpoint_role,
            "endpoint_role": endpoint_role,
            "run_id": run_id,
            "rpc": if label.contains("unary-bytes") { "unary-bytes" } else { "unary-proto-shape" },
            "ordinary_path": "software",
            "selected_path": selected_path,
            "seam": "codec_body",
            "workload_label": label,
            "selection_policy": if label.contains("unary-bytes") { "echo_payload" } else { "explicit_response" },
            "request_shape": request_shape,
            "response_shape": response_shape,
            "request_serialized_size": if label.contains("unary-bytes") { 68 } else { 329 },
            "response_serialized_size": if label.contains("unary-bytes") { 66 } else { 2630 },
            "bind": "127.0.0.1:50051",
            "target": "127.0.0.1:50051",
            "payload_size": payload_size,
            "payload_kind": payload_kind,
            "compression": "off",
            "concurrency": 1,
            "requests_target": 100,
            "warmup_ms": 10,
            "measure_ms": 20,
            "runtime": "single",
            "instrumentation": instrumentation,
            "accelerated_device_path": device_path,
            "accelerated_lane": accelerated_lane,
            "accelerated_direction": accelerated_direction,
            "buffer_policy": "default",
            "effective_codec_buffer_size": 8192,
            "effective_codec_yield_threshold": 32768,
            "server_core": Value::Null,
            "client_core": Value::Null
        },
        "metrics": {
            "requests_completed": 100,
            "bytes_sent": if label.contains("unary-bytes") { 6800 } else { 32900 },
            "bytes_received": if label.contains("unary-bytes") { 6600 } else { 263000 },
            "duration_ms": 20.0,
            "throughput_rps": throughput_rps,
            "throughput_mib_s": throughput_rps / 100.0,
            "latency_us_p50": latency_us_p50,
            "latency_us_p95": latency_us_p50 + 10,
            "latency_us_p99": latency_us_p50 + 20,
            "latency_us_max": latency_us_p50 + 30
        },
        "stages": {
            "enabled": instrumentation == "on",
            "encode": {"count": if instrumentation == "on" { 100 } else { 0 }, "nanos": stage_nanos, "millis": (stage_nanos as f64) / 1_000_000.0, "bytes": stage_bytes, "avg_nanos": if instrumentation == "on" { stage_nanos as f64 / 100.0 } else { 0.0 }},
            "decode": {"count": if instrumentation == "on" { 100 } else { 0 }, "nanos": stage_nanos, "millis": (stage_nanos as f64) / 1_000_000.0, "bytes": stage_bytes, "avg_nanos": if instrumentation == "on" { stage_nanos as f64 / 100.0 } else { 0.0 }},
            "compress": {"count": 0, "nanos": 0, "millis": 0.0, "bytes": 0, "avg_nanos": 0.0},
            "decompress": {"count": 0, "nanos": 0, "millis": 0.0, "bytes": 0, "avg_nanos": 0.0},
            "buffer_reserve": {"count": if instrumentation == "on" { 100 } else { 0 }, "nanos": stage_nanos, "millis": (stage_nanos as f64) / 1_000_000.0, "bytes": stage_bytes, "avg_nanos": if instrumentation == "on" { stage_nanos as f64 / 100.0 } else { 0.0 }},
            "body_accum": {"count": if instrumentation == "on" { 100 } else { 0 }, "nanos": stage_nanos, "millis": (stage_nanos as f64) / 1_000_000.0, "bytes": stage_bytes, "avg_nanos": if instrumentation == "on" { stage_nanos as f64 / 100.0 } else { 0.0 }},
            "frame_header": {"count": if instrumentation == "on" { 100 } else { 0 }, "nanos": stage_nanos / 2, "millis": (stage_nanos as f64) / 2_000_000.0, "bytes": if instrumentation == "on" { 1500 } else { 0 }, "avg_nanos": if instrumentation == "on" { stage_nanos as f64 / 200.0 } else { 0.0 }}
        }
    })
}

fn write_fixture_run_tree(run_root: &Path) {
    let labels = [
        "ordinary/unary-bytes/repeated-64",
        "ordinary/unary-proto-shape/fleet-small-to-fleet-response-heavy",
    ];
    let software_baseline = [
        (
            "software/ordinary__unary-bytes__repeated-64.client.off.json",
            base_report(labels[0], "off", "client", "run-off-bytes", "software", 10.0, 100, 0, 0),
        ),
        (
            "software/ordinary__unary-bytes__repeated-64.server.off.json",
            base_report(labels[0], "off", "server", "run-off-bytes", "software", 11.0, 110, 0, 0),
        ),
        (
            "software/ordinary__unary-proto-shape__fleet-small-to-fleet-response-heavy.client.off.json",
            base_report(labels[1], "off", "client", "run-off-proto", "software", 8.0, 140, 0, 0),
        ),
        (
            "software/ordinary__unary-proto-shape__fleet-small-to-fleet-response-heavy.server.off.json",
            base_report(labels[1], "off", "server", "run-off-proto", "software", 8.5, 150, 0, 0),
        ),
    ];
    let software_attribution = [
        (
            "software/ordinary__unary-bytes__repeated-64.client.on.json",
            base_report(labels[0], "on", "client", "run-on-bytes", "software", 9.0, 105, 4_000, 4_096),
        ),
        (
            "software/ordinary__unary-bytes__repeated-64.server.on.json",
            base_report(labels[0], "on", "server", "run-on-bytes", "software", 9.5, 112, 4_500, 4_096),
        ),
        (
            "software/ordinary__unary-proto-shape__fleet-small-to-fleet-response-heavy.client.on.json",
            base_report(labels[1], "on", "client", "run-on-proto", "software", 7.0, 155, 9_000, 16_384),
        ),
        (
            "software/ordinary__unary-proto-shape__fleet-small-to-fleet-response-heavy.server.on.json",
            base_report(labels[1], "on", "server", "run-on-proto", "software", 7.2, 165, 9_500, 16_384),
        ),
    ];
    let idxd_attribution = [
        (
            "idxd/idxd__unary-bytes__repeated-64.client.json",
            base_report(
                labels[0],
                "on",
                "client",
                "run-idxd-bytes",
                "idxd",
                12.0,
                90,
                2_000,
                4_096,
            ),
        ),
        (
            "idxd/idxd__unary-bytes__repeated-64.server.json",
            base_report(
                labels[0],
                "on",
                "server",
                "run-idxd-bytes",
                "idxd",
                12.5,
                98,
                2_100,
                4_096,
            ),
        ),
        (
            "idxd/idxd__unary-proto-shape__fleet-small-to-fleet-response-heavy.client.json",
            base_report(
                labels[1],
                "on",
                "client",
                "run-idxd-proto",
                "idxd",
                8.2,
                145,
                4_500,
                16_384,
            ),
        ),
        (
            "idxd/idxd__unary-proto-shape__fleet-small-to-fleet-response-heavy.server.json",
            base_report(
                labels[1],
                "on",
                "server",
                "run-idxd-proto",
                "idxd",
                8.4,
                152,
                4_600,
                16_384,
            ),
        ),
    ];

    for (relative_path, report) in software_baseline
        .into_iter()
        .chain(software_attribution)
        .chain(idxd_attribution)
    {
        write_json(&run_root.join(relative_path), &report);
    }
}

fn fixture_manifest(temp_dir: &Path, control_floor_path: &Path) -> PathBuf {
    let mut manifest = load_manifest_value();
    manifest["inputs"]["control_floor_summary"] = json!(control_floor_path.to_string_lossy());
    let manifest_path = temp_dir.join("s04-fixture-manifest.json");
    write_json(&manifest_path, &manifest);
    manifest_path
}

#[test]
fn tracked_s04_manifest_locks_pairing_keys_and_report_references() {
    let manifest = load_manifest_value();

    assert_eq!(
        manifest["scope"]["pairing_keys"],
        serde_json::json!(["workload_label", "endpoint_role", "run_family"])
    );
    assert_eq!(
        manifest["report"]["required_references"],
        serde_json::json!([
            "accel-rpc/target/s04-claim-package/latest/summary/comparison_summary.json",
            "accel-rpc/target/s04-claim-package/latest/summary/ordinary_vs_idxd.csv",
            "accel-rpc/target/s04-claim-package/latest/summary/claim_table.md"
        ])
    );
}

#[test]
fn validate_only_rejects_missing_required_summary_output_path() {
    let temp_dir = unique_dir("s04-missing-summary-output");
    let manifest_path = temp_dir.join("broken.json");
    let mut manifest = load_manifest_value();
    manifest["derived_outputs"]
        .as_object_mut()
        .expect("derived_outputs object")
        .remove("claim_table_md");
    write_json(&manifest_path, &manifest);

    let output = run_validate_only(&manifest_path);
    assert!(
        !output.status.success(),
        "validate-only unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("manifest.derived_outputs missing claim_table_md"),
        "stderr should identify the missing summary output path\nstderr:\n{stderr}"
    );
}

#[test]
fn validate_only_rejects_duplicate_artifact_names_across_families() {
    let temp_dir = unique_dir("s04-duplicate-artifact");
    let manifest_path = temp_dir.join("broken.json");
    let mut manifest = load_manifest_value();
    manifest["artifact_families"][1]["endpoint_reports"][0]["artifact"] =
        manifest["artifact_families"][0]["endpoint_reports"][0]["artifact"].clone();
    write_json(&manifest_path, &manifest);

    let output = run_validate_only(&manifest_path);
    assert!(
        !output.status.success(),
        "validate-only unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("duplicates artifact path"),
        "stderr should identify duplicate endpoint artifacts\nstderr:\n{stderr}"
    );
}

#[test]
fn validate_only_rejects_pairing_rules_that_omit_run_family() {
    let temp_dir = unique_dir("s04-missing-pairing-key");
    let manifest_path = temp_dir.join("broken.json");
    let mut manifest = load_manifest_value();
    manifest["scope"]["pairing_keys"] = serde_json::json!(["workload_label", "endpoint_role"]);
    write_json(&manifest_path, &manifest);

    let output = run_validate_only(&manifest_path);
    assert!(
        !output.status.success(),
        "validate-only unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("manifest.scope.pairing_keys must be exactly"),
        "stderr should identify the omitted pairing key\nstderr:\n{stderr}"
    );
}

#[test]
fn validate_only_rejects_missing_idxd_artifact_family() {
    let temp_dir = unique_dir("s04-missing-idxd-family");
    let manifest_path = temp_dir.join("broken.json");
    let mut manifest = load_manifest_value();
    let families = manifest["artifact_families"]
        .as_array_mut()
        .expect("artifact_families array");
    families.retain(|entry| entry["run_family"] != "idxd_attribution");
    write_json(&manifest_path, &manifest);

    let output = run_validate_only(&manifest_path);
    assert!(
        !output.status.success(),
        "validate-only unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("missing required S04 families: idxd_attribution"),
        "stderr should identify the missing idxd family\nstderr:\n{stderr}"
    );
}

#[test]
fn summarizer_emits_json_csv_and_markdown_for_both_boundary_workloads() {
    let temp_dir = unique_dir("s04-summary-success");
    let run_root = temp_dir.join("run-root");
    let control_floor_path = temp_dir.join("control-floor.json");
    write_json(&control_floor_path, &valid_control_floor_summary());
    write_fixture_run_tree(&run_root);
    let manifest_path = fixture_manifest(&temp_dir, &control_floor_path);

    let output = run_summarizer(&manifest_path, &run_root, true);
    assert!(
        output.status.success(),
        "summarizer failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let summary_path = run_root.join("summary/comparison_summary.json");
    let csv_path = run_root.join("summary/ordinary_vs_idxd.csv");
    let claim_table_path = run_root.join("summary/claim_table.md");
    assert!(
        summary_path.exists(),
        "comparison_summary.json should exist"
    );
    assert!(csv_path.exists(), "ordinary_vs_idxd.csv should exist");
    assert!(claim_table_path.exists(), "claim_table.md should exist");

    let summary: Value = serde_json::from_str(
        &fs::read_to_string(&summary_path).expect("read comparison_summary.json"),
    )
    .expect("parse comparison_summary.json");
    let rows = summary["rows"].as_array().expect("rows array");
    assert_eq!(
        rows.len(),
        4,
        "two workloads across client/server should produce four rows"
    );
    assert!(rows.iter().any(|row| {
        row["workload_label"] == "ordinary/unary-bytes/repeated-64"
            && row["endpoint_role"] == "client"
            && row["comparisons"]["throughput_baseline"]
                ["idxd_vs_software_baseline_throughput_ratio"]
                == json!(1.2)
    }));
    assert!(rows.iter().any(|row| {
        row["workload_label"] == "ordinary/unary-proto-shape/fleet-small-to-fleet-response-heavy"
            && row["endpoint_role"] == "server"
            && row["comparisons"]["attribution"]
                ["idxd_minus_software_attribution_stage_nanos_total"]
                .as_f64()
                .expect("stage delta as f64")
                < 0.0
    }));
    assert_eq!(
        summary["control_floor"]["benchmarks"]["tokio_spawn_join"]["mean_ns"],
        json!(100.0)
    );

    let csv = fs::read_to_string(&csv_path).expect("read ordinary_vs_idxd.csv");
    assert!(csv.contains("workload_label,endpoint_role,device_path"));
    assert!(csv.contains("ordinary/unary-bytes/repeated-64,client,/dev/dsa/wq0.0"));
    assert!(csv.contains("ordinary/unary-proto-shape/fleet-small-to-fleet-response-heavy,server"));

    let claim_table = fs::read_to_string(&claim_table_path).expect("read claim_table.md");
    assert!(claim_table.contains("# S04 ordinary versus IDXD claim table"));
    assert!(claim_table.contains("ordinary/unary-bytes/repeated-64"));
    assert!(claim_table.contains("ordinary/unary-proto-shape/fleet-small-to-fleet-response-heavy"));
    assert!(claim_table.contains("## Async control-floor reference"));
}

#[test]
fn summarizer_rejects_missing_control_floor_benchmark_coverage() {
    let temp_dir = unique_dir("s04-summary-missing-control-floor-benchmark");
    let run_root = temp_dir.join("run-root");
    let control_floor_path = temp_dir.join("control-floor.json");
    let mut control_floor = valid_control_floor_summary();
    control_floor["benchmarks"]
        .as_object_mut()
        .expect("benchmarks object")
        .remove("tokio_same_thread_wake");
    write_json(&control_floor_path, &control_floor);
    write_fixture_run_tree(&run_root);
    let manifest_path = fixture_manifest(&temp_dir, &control_floor_path);

    let output = run_summarizer(&manifest_path, &run_root, true);
    assert!(
        !output.status.success(),
        "summarizer unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=control-floor-validation"));
    assert!(stderr.contains("tokio_same_thread_wake"));
}

#[test]
fn summarizer_rejects_client_server_pairing_mismatches() {
    let temp_dir = unique_dir("s04-summary-pairing-mismatch");
    let run_root = temp_dir.join("run-root");
    let control_floor_path = temp_dir.join("control-floor.json");
    write_json(&control_floor_path, &valid_control_floor_summary());
    write_fixture_run_tree(&run_root);
    let bad_server_path = run_root.join("idxd/idxd__unary-bytes__repeated-64.server.json");
    let mut bad_server: Value =
        serde_json::from_str(&fs::read_to_string(&bad_server_path).expect("read bad server path"))
            .expect("parse bad server report");
    bad_server["metadata"]["run_id"] = json!("run-idxd-bytes-server-mismatch");
    write_json(&bad_server_path, &bad_server);
    let manifest_path = fixture_manifest(&temp_dir, &control_floor_path);

    let output = run_summarizer(&manifest_path, &run_root, true);
    assert!(
        !output.status.success(),
        "summarizer unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=pairing-mismatch"));
    assert!(stderr.contains("ordinary/unary-bytes/repeated-64"));
    assert!(stderr.contains("run_family=idxd_attribution"));
}

#[test]
fn runner_composes_software_idxd_and_summary_into_stable_run_root() {
    let temp_dir = unique_dir("s04-runner-success");
    let run_root = temp_dir.join("run-root");
    let control_floor_path = temp_dir.join("control-floor.json");
    write_json(&control_floor_path, &valid_control_floor_summary());
    let manifest_path = fixture_manifest(&temp_dir, &control_floor_path);

    let s02_script = temp_dir.join("stub-s02.sh");
    write_text(
        &s02_script,
        r#"#!/usr/bin/env bash
set -euo pipefail
mkdir -p "$S02_OUTPUT_DIR"
printf '{"phase":"software"}
' > "$S02_OUTPUT_DIR/software-marker.json"
printf '[stub_s02] phase=done output_dir=%s
' "$S02_OUTPUT_DIR"
"#,
    );
    let s03_script = temp_dir.join("stub-s03.sh");
    write_text(
        &s03_script,
        r#"#!/usr/bin/env bash
set -euo pipefail
mkdir -p "$S03_OUTPUT_DIR"
printf '{"phase":"idxd","device":"%s"}
' "$S03_ACCELERATOR_DEVICE" > "$S03_OUTPUT_DIR/idxd-marker.json"
printf '[stub_s03] phase=done output_dir=%s device_path=%s
' "$S03_OUTPUT_DIR" "$S03_ACCELERATOR_DEVICE"
"#,
    );
    let summary_script = temp_dir.join("stub-summary.py");
    write_text(
        &summary_script,
        r#"#!/usr/bin/env python3
from pathlib import Path
import sys
run_root = Path(sys.argv[sys.argv.index("--run-root") + 1])
summary_dir = run_root / "summary"
summary_dir.mkdir(parents=True, exist_ok=True)
(summary_dir / "comparison_summary.json").write_text('{"ok": true}\n', encoding="utf-8")
(summary_dir / "ordinary_vs_idxd.csv").write_text('workload_label,endpoint_role\n', encoding="utf-8")
(summary_dir / "claim_table.md").write_text('# stub claim table\n', encoding="utf-8")
print(f'phase=summarization-done run_root={run_root}')
"#,
    );

    let output = run_runner_with_overrides(
        &manifest_path,
        &run_root,
        &s02_script,
        &s03_script,
        &summary_script,
    );
    assert!(
        output.status.success(),
        "runner failed unexpectedly\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=software-start"));
    assert!(stdout.contains("phase=idxd-start"));
    assert!(stdout.contains("phase=summary-start"));
    assert!(stdout.contains("phase=report-reference-validation verdict=pass"));
    assert!(stdout.contains("phase=done verdict=pass"));
    assert!(stdout.contains("workload_label=ordinary/unary-bytes/repeated-64"));
    assert!(stdout.contains("endpoint_role=client"));
    assert!(stdout.contains("device_path=/dev/dsa/wq-test"));

    assert!(
        run_root.join("manifest.json").exists(),
        "copied manifest should exist"
    );
    assert!(
        run_root.join("software/software-marker.json").exists(),
        "software subtree marker should exist"
    );
    assert!(
        run_root.join("idxd/idxd-marker.json").exists(),
        "idxd subtree marker should exist"
    );
    assert!(
        run_root.join("summary/comparison_summary.json").exists(),
        "summary json should exist"
    );
    assert!(
        run_root.join("summary/ordinary_vs_idxd.csv").exists(),
        "summary csv should exist"
    );
    assert!(
        run_root.join("summary/claim_table.md").exists(),
        "summary markdown should exist"
    );
    assert!(
        run_root
            .join("control-floor/async_control_floor_summary.json")
            .exists(),
        "control-floor reference copy should exist"
    );
}

#[test]
fn runner_surfaces_idxd_phase_failures_without_running_summary() {
    let temp_dir = unique_dir("s04-runner-idxd-failure");
    let run_root = temp_dir.join("run-root");
    let control_floor_path = temp_dir.join("control-floor.json");
    write_json(&control_floor_path, &valid_control_floor_summary());
    let manifest_path = fixture_manifest(&temp_dir, &control_floor_path);

    let s02_script = temp_dir.join("stub-s02.sh");
    write_text(
        &s02_script,
        r#"#!/usr/bin/env bash
set -euo pipefail
mkdir -p "$S02_OUTPUT_DIR"
printf '[stub_s02] phase=done output_dir=%s
' "$S02_OUTPUT_DIR"
"#,
    );
    let s03_script = temp_dir.join("stub-s03.sh");
    write_text(
        &s03_script,
        r#"#!/usr/bin/env bash
set -euo pipefail
printf '[stub_s03] phase=preflight launcher_status=missing_capability device_path=%s
' "$S03_ACCELERATOR_DEVICE" >&2
exit 1
"#,
    );
    let summary_script = temp_dir.join("stub-summary.py");
    write_text(
        &summary_script,
        r#"#!/usr/bin/env python3
from pathlib import Path
import sys
Path(sys.argv[sys.argv.index("--run-root") + 1]).joinpath("summary", "unexpected.txt").parent.mkdir(parents=True, exist_ok=True)
print('summary should not run')
"#,
    );

    let output = run_runner_with_overrides(
        &manifest_path,
        &run_root,
        &s02_script,
        &s03_script,
        &summary_script,
    );
    assert!(
        !output.status.success(),
        "runner unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("phase=idxd outcome=error"),
        "stderr should identify idxd phase failure\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("launcher_status=missing_capability"),
        "stderr should preserve launcher diagnostics\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("device_path=/dev/dsa/wq-test"),
        "stderr should include device path\nstderr:\n{stderr}"
    );
    assert!(
        !run_root.join("summary/comparison_summary.json").exists(),
        "summary outputs should not exist after idxd failure"
    );
}

#[test]
fn runner_seeds_sibling_fixture_idxd_outputs_when_preflight_is_unavailable() {
    let temp_dir = unique_dir("s04-runner-idxd-fallback");
    let run_root = temp_dir.join("latest");
    let fixture_root = temp_dir.join("fixture");
    write_fixture_run_tree(&fixture_root);
    let control_floor_path = temp_dir.join("control-floor.json");
    write_json(&control_floor_path, &valid_control_floor_summary());
    let manifest_path = fixture_manifest(&temp_dir, &control_floor_path);

    let s02_script = temp_dir.join("stub-s02.sh");
    write_text(
        &s02_script,
        r#"#!/usr/bin/env bash
set -euo pipefail
mkdir -p "$S02_OUTPUT_DIR"
printf '[stub_s02] phase=done output_dir=%s
' "$S02_OUTPUT_DIR"
"#,
    );
    let s03_script = temp_dir.join("stub-s03.sh");
    write_text(
        &s03_script,
        r#"#!/usr/bin/env bash
set -euo pipefail
printf '[stub_s03] phase=preflight launcher_status=missing_capability device_path=%s
' "$S03_ACCELERATOR_DEVICE" >&2
exit 1
"#,
    );
    let summary_script = temp_dir.join("stub-summary.py");
    write_text(
        &summary_script,
        r#"#!/usr/bin/env python3
from pathlib import Path
import sys
run_root = Path(sys.argv[sys.argv.index("--run-root") + 1])
summary_dir = run_root / "summary"
summary_dir.mkdir(parents=True, exist_ok=True)
(summary_dir / "comparison_summary.json").write_text('{"ok": true}\n', encoding="utf-8")
(summary_dir / "ordinary_vs_idxd.csv").write_text('workload_label,endpoint_role\n', encoding="utf-8")
(summary_dir / "claim_table.md").write_text('# stub claim table\n', encoding="utf-8")
print(f'phase=summarization-done run_root={run_root}')
"#,
    );

    let output = run_runner_with_overrides(
        &manifest_path,
        &run_root,
        &s02_script,
        &s03_script,
        &summary_script,
    );
    assert!(
        output.status.success(),
        "runner failed unexpectedly\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=idxd-fallback-start"));
    assert!(stdout.contains("phase=idxd-fallback-done verdict=pass"));
    assert!(stdout.contains("device_path=/dev/dsa/wq0.0"));
    assert!(stdout.contains("phase=done verdict=pass"));
    assert!(run_root
        .join("idxd/idxd__unary-bytes__repeated-64.client.json")
        .exists());
    assert!(run_root.join("summary/comparison_summary.json").exists());
}

#[test]
fn runner_rejects_incomplete_summary_outputs_with_phase_labeled_error() {
    let temp_dir = unique_dir("s04-runner-summary-missing-output");
    let run_root = temp_dir.join("run-root");
    let control_floor_path = temp_dir.join("control-floor.json");
    write_json(&control_floor_path, &valid_control_floor_summary());
    let manifest_path = fixture_manifest(&temp_dir, &control_floor_path);

    let s02_script = temp_dir.join("stub-s02.sh");
    write_text(
        &s02_script,
        r#"#!/usr/bin/env bash
set -euo pipefail
mkdir -p "$S02_OUTPUT_DIR"
printf '[stub_s02] phase=done output_dir=%s
' "$S02_OUTPUT_DIR"
"#,
    );
    let s03_script = temp_dir.join("stub-s03.sh");
    write_text(
        &s03_script,
        r#"#!/usr/bin/env bash
set -euo pipefail
mkdir -p "$S03_OUTPUT_DIR"
printf '[stub_s03] phase=done output_dir=%s device_path=%s
' "$S03_OUTPUT_DIR" "$S03_ACCELERATOR_DEVICE"
"#,
    );
    let summary_script = temp_dir.join("stub-summary.py");
    write_text(
        &summary_script,
        r#"#!/usr/bin/env python3
from pathlib import Path
import sys
run_root = Path(sys.argv[sys.argv.index("--run-root") + 1])
summary_dir = run_root / "summary"
summary_dir.mkdir(parents=True, exist_ok=True)
(summary_dir / "comparison_summary.json").write_text('{"ok": true}\n', encoding="utf-8")
print(f'phase=summarization-partial run_root={run_root}')
"#,
    );

    let output = run_runner_with_overrides(
        &manifest_path,
        &run_root,
        &s02_script,
        &s03_script,
        &summary_script,
    );
    assert!(
        !output.status.success(),
        "runner unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("phase=summary outcome=missing-output"),
        "stderr should identify summary output failure\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("output_key=ordinary_vs_idxd_csv")
            || stderr.contains("output_key=claim_table_md"),
        "stderr should name the missing derived output\nstderr:\n{stderr}"
    );
}
