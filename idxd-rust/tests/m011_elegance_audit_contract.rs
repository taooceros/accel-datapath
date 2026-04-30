use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn crate_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn repo_root() -> PathBuf {
    crate_path("..")
        .canonicalize()
        .expect("idxd-rust parent should be the repository root")
}

fn guard_script() -> PathBuf {
    crate_path("scripts/check_m011_s05_elegance_audit.sh")
}

fn report_path() -> PathBuf {
    repo_root().join("docs/report/architecture/017.generic_idxd_elegance_audit.md")
}

fn run_guard(report: Option<&Path>) -> Output {
    let mut command = Command::new("bash");
    command.arg(guard_script()).current_dir(repo_root());
    if let Some(report) = report {
        command.arg(report);
    }
    command
        .output()
        .expect("S05 elegance audit guard should launch")
}

fn output_text(output: &Output) -> String {
    format!(
        "status={:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn unique_temp_report(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX_EPOCH")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "m011_s05_elegance_audit_{name}_{}_{}.md",
        std::process::id(),
        nanos
    ))
}

fn write_temp_report(name: &str, content: &str) -> PathBuf {
    let path = unique_temp_report(name);
    fs::write(&path, content).expect("temporary malformed report should be writable");
    path
}

fn remove_temp_report(path: &Path) {
    let _ = fs::remove_file(path);
}

#[test]
fn tracked_s05_elegance_audit_guard_passes() {
    let output = run_guard(Some(&report_path()));
    let diagnostics = output_text(&output);

    assert!(
        output.status.success(),
        "tracked S05 elegance audit guard should pass\n{diagnostics}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("verdict=pass"),
        "passing guard output should include verdict=pass\n{diagnostics}"
    );
    assert!(
        stdout.contains("source_checks=pass"),
        "passing guard output should include source drift coverage\n{diagnostics}"
    );
    assert!(
        stdout.contains("proof_boundary=generic_sessions_only"),
        "passing guard output should name the generic-session proof boundary\n{diagnostics}"
    );
}

#[test]
fn guard_fails_closed_for_missing_or_empty_report() {
    let missing = unique_temp_report("missing");
    remove_temp_report(&missing);
    let missing_output = run_guard(Some(&missing));
    let missing_diagnostics = output_text(&missing_output);
    assert!(
        !missing_output.status.success(),
        "missing report path should fail closed\n{missing_diagnostics}"
    );
    assert!(
        missing_diagnostics.contains("missing report"),
        "missing report failure should be actionable\n{missing_diagnostics}"
    );

    let empty = write_temp_report("empty", "");
    let empty_output = run_guard(Some(&empty));
    let empty_diagnostics = output_text(&empty_output);
    remove_temp_report(&empty);
    assert!(
        !empty_output.status.success(),
        "empty report should fail closed\n{empty_diagnostics}"
    );
    assert!(
        empty_diagnostics.contains("empty report"),
        "empty report failure should be actionable\n{empty_diagnostics}"
    );
}

#[test]
fn guard_rejects_stale_overclaims_and_payload_logging_language() {
    let tracked_report = fs::read_to_string(report_path())
        .expect("tracked S05 report should be readable for negative fixture");
    let stale_report = format!(
        "{tracked_report}\n\nM011 delivered a full DSA surface, implemented a production-grade scheduler, and shipped a benchmark matrix/framework. Raw buffer bytes are logged as payload byte examples.\n"
    );
    let stale_path = write_temp_report("stale_overclaim", &stale_report);
    let output = run_guard(Some(&stale_path));
    let diagnostics = output_text(&output);
    remove_temp_report(&stale_path);

    assert!(
        !output.status.success(),
        "stale overclaim report should fail closed\n{diagnostics}"
    );
    assert!(
        diagnostics.contains("stale overclaim") || diagnostics.contains("payload_examples"),
        "stale overclaim failure should name the report-language problem\n{diagnostics}"
    );
}
