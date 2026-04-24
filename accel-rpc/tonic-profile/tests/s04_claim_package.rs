use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

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
