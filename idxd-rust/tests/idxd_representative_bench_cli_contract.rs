use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

const TOP_LEVEL_FIELDS: &[&str] = &[
    "schema_version",
    "ok",
    "verdict",
    "claim_eligible",
    "suite",
    "profile",
    "requested_bytes",
    "iterations",
    "warmup_iterations",
    "clock",
    "failure_phase",
    "error_kind",
    "failure_target",
    "failure_accelerator",
    "targets",
];

const TARGET_FIELDS: &[&str] = &[
    "target",
    "operation",
    "family",
    "device_path",
    "work_queue_mode",
    "target_role",
    "requested_bytes",
    "iterations",
    "warmup_iterations",
    "ok",
    "verdict",
    "claim_eligible",
    "completed_operations",
    "failed_operations",
    "elapsed_ns",
    "min_latency_ns",
    "mean_latency_ns",
    "max_latency_ns",
    "ops_per_sec",
    "bytes_per_sec",
    "total_page_fault_retries",
    "last_page_fault_retries",
    "final_status",
    "completion_error_code",
    "invalid_flags",
    "fault_addr",
    "crc64",
    "expected_crc64",
    "crc64_verified",
    "failure_phase",
    "error_kind",
    "message",
];

fn bench_bin() -> &'static str {
    env!("CARGO_BIN_EXE_idxd_representative_bench")
}

fn crate_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn unique_temp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("idxd-rust-representative-bench-{name}-{nanos}"))
}

fn run(args: &[&str]) -> Output {
    Command::new(bench_bin())
        .args(args)
        .output()
        .expect("benchmark binary should launch")
}

fn stdout_json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("stdout should be valid json")
}

fn targets(value: &Value) -> &[Value] {
    value["targets"]
        .as_array()
        .expect("targets should be a json array")
}

fn assert_required_schema(value: &Value) {
    let object = value.as_object().expect("artifact should be a json object");
    for field in TOP_LEVEL_FIELDS {
        assert!(
            object.contains_key(*field),
            "missing top-level benchmark schema field `{field}` in {value}"
        );
    }

    for target in targets(value) {
        let object = target
            .as_object()
            .expect("target row should be a json object");
        for field in TARGET_FIELDS {
            assert!(
                object.contains_key(*field),
                "missing target benchmark schema field `{field}` in {target}"
            );
        }
    }
}

fn assert_no_payload_dump_fields(value: &Value) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                assert!(
                    !matches!(
                        key.as_str(),
                        "payload"
                            | "payload_bytes"
                            | "source"
                            | "source_bytes"
                            | "source_payload"
                            | "destination"
                            | "destination_bytes"
                            | "destination_payload"
                            | "src"
                            | "dst"
                    ),
                    "unexpected payload dump field `{key}` in {value}"
                );
                assert_no_payload_dump_fields(child);
            }
        }
        Value::Array(items) => {
            for child in items {
                assert_no_payload_dump_fields(child);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn assert_failure_rows_do_not_claim_positive_metrics(value: &Value) {
    for row in targets(value) {
        if row["ok"].as_bool() == Some(true) {
            for field in [
                "elapsed_ns",
                "min_latency_ns",
                "mean_latency_ns",
                "max_latency_ns",
                "ops_per_sec",
                "bytes_per_sec",
            ] {
                assert!(
                    row[field].as_f64().expect("pass metric should be numeric") > 0.0,
                    "pass row metric `{field}` must be positive in {row}"
                );
            }
        } else {
            assert_eq!(row["claim_eligible"], false);
            for field in [
                "elapsed_ns",
                "min_latency_ns",
                "mean_latency_ns",
                "max_latency_ns",
                "ops_per_sec",
                "bytes_per_sec",
            ] {
                assert!(
                    row[field].is_null(),
                    "failure row should not emit positive metric claim `{field}` in {row}"
                );
            }
        }
    }
}

#[test]
fn benchmark_binary_uses_generic_session_operations_not_raw_surfaces() {
    let source = fs::read_to_string(crate_path("src/bin/idxd_representative_bench.rs"))
        .expect("benchmark source should be tracked");

    for required in [
        "IdxdSession::<Dsa>::open",
        "IdxdSession::<Iax>::open",
        "session.memmove(&mut dst, &src)",
        "session.crc64(&src)",
        "crc64_t10dif_field",
        "profile()",
        "\"release\"",
    ] {
        assert!(
            source.contains(required),
            "benchmark should exercise and report the generic session path: missing {required:?}"
        );
    }

    for forbidden in [
        "DsaSession",
        "WqPortal",
        "submit_movdir64b",
        "submit_enqcmd",
        "run_direct_memmove",
        "run_iax_crc64",
        "portal.submit(",
        "hw-eval",
        "pub trait",
        "unsafe",
    ] {
        assert!(
            !source.contains(forbidden),
            "benchmark must not bypass the generic session/lifecycle boundary: found {forbidden:?}"
        );
    }
}

#[test]
fn prints_help_without_touching_hardware() {
    let output = run(&["--help"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("idxd_representative_bench"));
    assert!(stdout.contains("--dsa-device PATH"));
    assert!(stdout.contains("--iax-device PATH"));
    assert!(stdout.contains("IdxdSession<Dsa>::memmove"));
    assert!(stdout.contains("IdxdSession<Iax>::crc64"));
}

#[test]
fn rejects_missing_required_devices_before_touching_hardware() {
    let missing_both = run(&[]);
    assert_eq!(missing_both.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&missing_both.stdout).is_empty());
    assert!(
        String::from_utf8_lossy(&missing_both.stderr).contains("missing required `--dsa-device`")
    );

    let missing_iax = run(&["--dsa-device", "/dev/dsa/does-not-exist"]);
    assert_eq!(missing_iax.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&missing_iax.stdout).is_empty());
    assert!(
        String::from_utf8_lossy(&missing_iax.stderr).contains("missing required `--iax-device`")
    );
}

#[test]
fn rejects_malformed_cli_inputs_before_touching_hardware() {
    for args in [
        [
            "--dsa-device",
            "/dev/dsa/nope",
            "--iax-device",
            "/dev/iax/nope",
            "--bytes",
            "abc",
        ]
        .as_slice(),
        [
            "--dsa-device",
            "/dev/dsa/nope",
            "--iax-device",
            "/dev/iax/nope",
            "--bytes",
            "0",
        ]
        .as_slice(),
        [
            "--dsa-device",
            "/dev/dsa/nope",
            "--iax-device",
            "/dev/iax/nope",
            "--iterations",
            "abc",
        ]
        .as_slice(),
        [
            "--dsa-device",
            "/dev/dsa/nope",
            "--iax-device",
            "/dev/iax/nope",
            "--iterations",
            "0",
        ]
        .as_slice(),
        ["--dsa-device", "/dev/dsa/nope", "--iax-device", ""].as_slice(),
        [
            "--dsa-device",
            "/dev/dsa/nope",
            "--iax-device",
            "/dev/iax/nope",
            "--format",
            "xml",
        ]
        .as_slice(),
        [
            "--dsa-device",
            "/dev/dsa/nope",
            "--iax-device",
            "/dev/iax/nope",
            "--dsa-shared-device",
            "/dev/dsa/nope",
        ]
        .as_slice(),
    ] {
        let output = run(args);
        assert_eq!(output.status.code(), Some(2), "args={args:?}");
        assert!(String::from_utf8_lossy(&output.stdout).is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).is_empty());
    }
}

#[test]
fn rejects_bad_artifact_paths_before_touching_hardware() {
    let missing_value = run(&[
        "--dsa-device",
        "/dev/dsa/does-not-exist",
        "--iax-device",
        "/dev/iax/does-not-exist",
        "--artifact",
    ]);
    assert_eq!(missing_value.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&missing_value.stdout).is_empty());
    assert!(
        String::from_utf8_lossy(&missing_value.stderr).contains("missing value for `--artifact`")
    );

    let temp_dir = unique_temp_path("artifact-dir");
    fs::create_dir_all(&temp_dir).expect("temp dir should be creatable");

    let directory_artifact = run(&[
        "--dsa-device",
        "/dev/dsa/does-not-exist",
        "--iax-device",
        "/dev/iax/does-not-exist",
        "--artifact",
        temp_dir.to_str().expect("temp dir should be utf-8"),
    ]);
    assert_eq!(directory_artifact.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&directory_artifact.stdout).is_empty());
    assert!(
        String::from_utf8_lossy(&directory_artifact.stderr)
            .contains("expected a writable file path")
    );

    let missing_parent = unique_temp_path("missing-parent").join("artifact.json");
    let missing_parent_artifact = run(&[
        "--dsa-device",
        "/dev/dsa/does-not-exist",
        "--iax-device",
        "/dev/iax/does-not-exist",
        "--artifact",
        missing_parent.to_str().expect("temp path should be utf-8"),
    ]);
    assert_eq!(missing_parent_artifact.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&missing_parent_artifact.stdout).is_empty());
    assert!(String::from_utf8_lossy(&missing_parent_artifact.stderr).contains("writable parent"));

    fs::remove_dir_all(&temp_dir).expect("temp dir cleanup should succeed");
}

#[test]
fn nonexistent_devices_emit_stable_no_payload_failure_json() {
    let output = run(&[
        "--dsa-device",
        "/dev/dsa/does-not-exist",
        "--iax-device",
        "/dev/iax/does-not-exist",
        "--bytes",
        "64",
        "--iterations",
        "2",
        "--format",
        "json",
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());

    let artifact = stdout_json(&output);
    assert_required_schema(&artifact);
    assert_eq!(artifact["schema_version"], 1);
    assert_eq!(artifact["ok"], false);
    assert_eq!(artifact["verdict"], "expected_failure");
    assert_eq!(artifact["claim_eligible"], false);
    assert_eq!(artifact["suite"], "idxd_representative_bench");
    assert_eq!(artifact["requested_bytes"], 64);
    assert_eq!(artifact["iterations"], 2);
    assert_eq!(artifact["warmup_iterations"], 1);
    assert_eq!(artifact["clock"], "std::time::Instant");
    assert_eq!(artifact["failure_phase"], "queue_open");
    assert_eq!(artifact["error_kind"], "queue_open");
    assert_eq!(artifact["failure_target"], "dsa-memmove");
    assert_eq!(artifact["failure_accelerator"], "dsa");

    let rows = targets(&artifact);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["target"], "dsa-memmove");
    assert_eq!(rows[0]["operation"], "memmove");
    assert_eq!(rows[0]["family"], "dsa");
    assert_eq!(rows[0]["device_path"], "/dev/dsa/does-not-exist");
    assert_eq!(rows[0]["work_queue_mode"], Value::Null);
    assert_eq!(rows[0]["completed_operations"], 0);
    assert_eq!(rows[0]["failed_operations"], 0);
    assert_eq!(rows[0]["failure_phase"], "queue_open");
    assert_eq!(rows[0]["error_kind"], "queue_open");

    assert_eq!(rows[1]["target"], "iax-crc64");
    assert_eq!(rows[1]["operation"], "crc64");
    assert_eq!(rows[1]["family"], "iax");
    assert_eq!(rows[1]["device_path"], "/dev/iax/does-not-exist");
    assert_eq!(rows[1]["failure_phase"], "queue_open");
    assert_eq!(rows[1]["error_kind"], "queue_open");
    assert!(rows[1]["crc64"].is_null());
    assert!(rows[1]["expected_crc64"].is_null());
    assert!(rows[1]["crc64_verified"].is_null());

    assert_failure_rows_do_not_claim_positive_metrics(&artifact);
    assert_no_payload_dump_fields(&artifact);
}

#[test]
fn optional_shared_dsa_target_is_reported_only_when_configured() {
    let output = run(&[
        "--dsa-device",
        "/dev/dsa/does-not-exist",
        "--iax-device",
        "/dev/iax/does-not-exist",
        "--dsa-shared-device",
        "/dev/dsa/shared-does-not-exist",
        "--bytes",
        "64",
        "--iterations",
        "2",
        "--format",
        "json",
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());

    let artifact = stdout_json(&output);
    let rows = targets(&artifact);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[2]["target"], "dsa-shared-memmove");
    assert_eq!(rows[2]["target_role"], "optional-shared");
    assert_eq!(rows[2]["family"], "dsa");
    assert_eq!(rows[2]["device_path"], "/dev/dsa/shared-does-not-exist");
    assert_eq!(rows[2]["failure_phase"], "queue_open");
    assert_no_payload_dump_fields(&artifact);
}

#[test]
fn text_output_surfaces_top_level_and_per_target_observability_fields() {
    let output = run(&[
        "--dsa-device",
        "/dev/dsa/does-not-exist",
        "--iax-device",
        "/dev/iax/does-not-exist",
        "--bytes",
        "64",
        "--iterations",
        "2",
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());

    let stdout = String::from_utf8_lossy(&output.stdout);
    for field in [
        "schema_version=",
        "ok=false",
        "verdict=expected_failure",
        "claim_eligible=false",
        "suite=idxd_representative_bench",
        "profile=",
        "requested_bytes=64",
        "iterations=2",
        "warmup_iterations=1",
        "clock=std::time::Instant",
        "failure_phase=queue_open",
        "error_kind=queue_open",
        "dsa-memmove.operation=memmove",
        "dsa-memmove.elapsed_ns=null",
        "dsa-memmove.ops_per_sec=null",
        "iax-crc64.operation=crc64",
        "iax-crc64.crc64_verified=null",
    ] {
        assert!(
            stdout.contains(field),
            "text output missing `{field}`: {stdout}"
        );
    }
}

#[test]
fn writes_artifact_matching_stdout_json_for_failure_schema() {
    let artifact_path = unique_temp_path("artifact.json");

    let output = run(&[
        "--dsa-device",
        "/dev/dsa/does-not-exist",
        "--iax-device",
        "/dev/iax/does-not-exist",
        "--bytes",
        "4096",
        "--iterations",
        "3",
        "--format",
        "json",
        "--artifact",
        artifact_path
            .to_str()
            .expect("artifact path should be valid utf-8"),
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());

    let stdout: Value =
        serde_json::from_slice(&output.stdout).expect("stdout should parse as json");
    let artifact_text = fs::read_to_string(&artifact_path).expect("artifact should be written");
    let artifact: Value =
        serde_json::from_str(&artifact_text).expect("artifact should parse as json");
    assert_eq!(artifact, stdout);
    assert_required_schema(&artifact);
    assert_eq!(artifact["requested_bytes"], 4096);
    assert_eq!(artifact["iterations"], 3);
    assert_eq!(artifact["verdict"], "expected_failure");
    assert_eq!(artifact["claim_eligible"], false);
    assert_failure_rows_do_not_claim_positive_metrics(&artifact);
    assert_no_payload_dump_fields(&artifact);

    fs::remove_file(&artifact_path).expect("artifact cleanup should succeed");
}
