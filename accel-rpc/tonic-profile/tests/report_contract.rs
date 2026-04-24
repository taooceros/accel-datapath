use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
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

fn load_and_validate_report(path: &Path) -> Result<Value, String> {
    if !path.exists() {
        return Err(format!("missing report file: {}", path.display()));
    }

    let raw = fs::read_to_string(path)
        .map_err(|err| format!("failed to read report {}: {err}", path.display()))?;
    if raw.trim().is_empty() {
        return Err(format!("empty report file: {}", path.display()));
    }

    let value: Value = serde_json::from_str(&raw)
        .map_err(|err| format!("invalid report json {}: {err}", path.display()))?;
    validate_report(&value)?;
    Ok(value)
}

fn validate_report(report: &Value) -> Result<(), String> {
    require_object_field(report, "metadata")?;
    require_object_field(report, "metrics")?;
    let stages = require_object_field(report, "stages")?;

    let metadata = report
        .get("metadata")
        .and_then(Value::as_object)
        .expect("metadata object already checked");
    for field in [
        "timestamp_unix_s",
        "mode",
        "rpc",
        "bind",
        "target",
        "payload_size",
        "payload_kind",
        "compression",
        "concurrency",
        "requests_target",
        "warmup_ms",
        "measure_ms",
        "runtime",
        "instrumentation",
        "buffer_policy",
        "effective_codec_buffer_size",
        "effective_codec_yield_threshold",
        "server_core",
        "client_core",
    ] {
        if !metadata.contains_key(field) {
            return Err(format!("missing metadata.{field}"));
        }
    }

    let metrics = report
        .get("metrics")
        .and_then(Value::as_object)
        .expect("metrics object already checked");
    for field in [
        "requests_completed",
        "bytes_sent",
        "bytes_received",
        "duration_ms",
        "throughput_rps",
        "throughput_mib_s",
        "latency_us_p50",
        "latency_us_p95",
        "latency_us_p99",
        "latency_us_max",
    ] {
        if !metrics.contains_key(field) {
            return Err(format!("missing metrics.{field}"));
        }
    }

    for stage_name in [
        "encode",
        "decode",
        "compress",
        "decompress",
        "buffer_reserve",
        "body_accum",
        "frame_header",
    ] {
        let stage = stages
            .get(stage_name)
            .and_then(Value::as_object)
            .ok_or_else(|| format!("missing stages.{stage_name}"))?;
        for counter_field in ["count", "nanos", "millis", "bytes", "avg_nanos"] {
            if !stage.contains_key(counter_field) {
                return Err(format!("missing stages.{stage_name}.{counter_field}"));
            }
        }
    }

    Ok(())
}

fn require_object_field<'a>(
    value: &'a Value,
    field: &str,
) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .get(field)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("missing {field}"))
}

#[test]
fn selftest_report_exposes_required_contract_fields() {
    let addr = reserve_addr();
    let json_out = unique_path("contract");
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
            "--instrumentation",
            "on",
            "--json-out",
            json_out.to_str().expect("utf8 path"),
        ])
        .output()
        .expect("spawn tonic-profile selftest");

    assert!(
        output.status.success(),
        "selftest failed for {addr}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report = load_and_validate_report(&json_out).expect("valid selftest report");
    assert_eq!(report["metadata"]["mode"], "selftest");
    assert_eq!(report["metadata"]["rpc"], "unary");
    assert_eq!(report["metadata"]["instrumentation"], "on");
    assert_eq!(report["stages"]["enabled"], true);
}

#[test]
fn report_loader_rejects_missing_empty_and_incomplete_reports() {
    let missing = unique_path("missing");
    let err = load_and_validate_report(&missing).expect_err("missing file should fail");
    assert!(
        err.contains("missing report file"),
        "unexpected error: {err}"
    );

    let empty = unique_path("empty");
    fs::write(&empty, "\n\n").expect("write empty report");
    let err = load_and_validate_report(&empty).expect_err("empty file should fail");
    assert!(err.contains("empty report file"), "unexpected error: {err}");

    let incomplete = unique_path("incomplete");
    fs::write(&incomplete, "{\"metrics\":{},\"stages\":{}}").expect("write incomplete report");
    let err = load_and_validate_report(&incomplete).expect_err("incomplete report should fail");
    assert!(err.contains("missing metadata"), "unexpected error: {err}");
}
