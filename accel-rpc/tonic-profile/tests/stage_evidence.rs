use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

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

fn run_selftest(extra_args: &[&str], artifact_name: &str) -> Value {
    let addr = reserve_addr();
    let json_out = unique_path(artifact_name);
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
    args.extend_from_slice(extra_args);

    let output = Command::new(tonic_profile_bin())
        .args(&args)
        .output()
        .expect("spawn tonic-profile selftest");

    assert!(
        output.status.success(),
        "selftest failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    parse_report(&json_out)
}

fn stage_u64(report: &Value, stage: &str, field: &str) -> u64 {
    report["stages"][stage][field]
        .as_u64()
        .unwrap_or_else(|| panic!("missing stages.{stage}.{field}"))
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
        stage_u64(report, stage, "count") == 0
            && stage_u64(report, stage, "nanos") == 0
            && stage_u64(report, stage, "bytes") == 0
    })
}

fn require_real_stage_evidence(report: &Value) -> Result<(), String> {
    if report["metadata"]["instrumentation"] == "on" && placeholder_only(report) {
        return Err(format!(
            "instrumentation-on report for workload {} endpoint {} stayed placeholder-only",
            report["metadata"]["workload_label"].as_str().unwrap_or("<unknown>"),
            report["metadata"]["endpoint_role"].as_str().unwrap_or("<unknown>")
        ));
    }
    Ok(())
}

#[test]
fn instrumentation_on_selftest_emits_real_nonzero_stage_counters() {
    let bytes_report = run_selftest(
        &[
            "--payload-size",
            "64",
            "--payload-kind",
            "repeated",
            "--instrumentation",
            "on",
        ],
        "stage-bytes",
    );
    let response_heavy_report = run_selftest(
        &[
            "--rpc",
            "unary-proto-shape",
            "--proto-shape",
            "fleet-response-heavy",
            "--response-shape",
            "fleet-response-heavy",
            "--instrumentation",
            "on",
        ],
        "stage-response-heavy",
    );

    assert_eq!(bytes_report["metadata"]["endpoint_role"], "selftest");
    assert_eq!(response_heavy_report["metadata"]["endpoint_role"], "selftest");
    require_real_stage_evidence(&bytes_report).expect("bytes report should be non-placeholder");
    require_real_stage_evidence(&response_heavy_report)
        .expect("response-heavy report should be non-placeholder");

    for report in [&bytes_report, &response_heavy_report] {
        assert!(report["stages"]["enabled"].as_bool().unwrap());
        assert!(stage_u64(report, "encode", "count") > 0);
        assert!(stage_u64(report, "decode", "count") > 0);
        assert!(stage_u64(report, "buffer_reserve", "count") > 0);
        assert!(stage_u64(report, "body_accum", "bytes") > 0);
        assert!(stage_u64(report, "frame_header", "bytes") >= 10);
    }

    assert!(
        stage_u64(&response_heavy_report, "encode", "bytes")
            > stage_u64(&bytes_report, "encode", "bytes"),
        "response-heavy workload should encode more bytes than the tiny bytes workload"
    );
    assert!(
        stage_u64(&response_heavy_report, "decode", "bytes")
            > stage_u64(&bytes_report, "decode", "bytes"),
        "response-heavy workload should decode more bytes than the tiny bytes workload"
    );
    assert!(
        stage_u64(&response_heavy_report, "body_accum", "bytes")
            > stage_u64(&bytes_report, "body_accum", "bytes"),
        "response-heavy workload should accumulate more body bytes than the tiny bytes workload"
    );
}

#[test]
fn instrumentation_off_selftest_remains_structurally_valid_baseline() {
    let report = run_selftest(
        &[
            "--payload-size",
            "64",
            "--payload-kind",
            "repeated",
            "--instrumentation",
            "off",
        ],
        "stage-baseline",
    );

    assert_eq!(report["metadata"]["instrumentation"], "off");
    assert_eq!(report["metadata"]["endpoint_role"], "selftest");
    assert_eq!(report["stages"]["enabled"], false);
    assert_eq!(report["metrics"]["requests_completed"], 1);
    assert!(report["metadata"]["run_id"].as_str().unwrap().starts_with("run-"));
    assert!(
        report["metrics"]["duration_ms"].as_f64().unwrap() >= 0.0,
        "baseline report should remain structurally valid even when instrumentation is off"
    );
}

#[test]
fn placeholder_only_instrumentation_on_artifacts_are_rejected() {
    let report = json!({
        "metadata": {
            "mode": "client",
            "endpoint_role": "client",
            "run_id": "run-stage-test",
            "workload_label": "ordinary/unary-bytes/repeated-64",
            "instrumentation": "on"
        },
        "stages": {
            "encode": {"count": 0, "nanos": 0, "bytes": 0},
            "decode": {"count": 0, "nanos": 0, "bytes": 0},
            "compress": {"count": 0, "nanos": 0, "bytes": 0},
            "decompress": {"count": 0, "nanos": 0, "bytes": 0},
            "buffer_reserve": {"count": 0, "nanos": 0, "bytes": 0},
            "body_accum": {"count": 0, "nanos": 0, "bytes": 0},
            "frame_header": {"count": 0, "nanos": 0, "bytes": 0}
        }
    });

    let err = require_real_stage_evidence(&report).expect_err("placeholder-only stages must fail");
    assert!(err.contains("ordinary/unary-bytes/repeated-64"), "unexpected error: {err}");
    assert!(err.contains("client"), "unexpected error: {err}");
}
