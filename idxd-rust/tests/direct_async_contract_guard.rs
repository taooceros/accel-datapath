use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "idxd-rust-direct-async-contract-{name}-{}-{nanos}.md",
        std::process::id()
    ))
}

fn verifier_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scripts/verify_direct_async_completion_contract.sh")
}

fn write_fixture(name: &str, content: &str) -> PathBuf {
    let path = unique_temp_path(name);
    fs::write(&path, content).expect("direct async contract fixture should be writable");
    path
}

fn run_verifier(report_path: Option<&Path>) -> Output {
    let mut command = Command::new("bash");
    command.arg(verifier_script());
    if let Some(report_path) = report_path {
        command.env("IDXD_RUST_DIRECT_ASYNC_REPORT_PATH", report_path);
    }
    command.output().expect("contract verifier should launch")
}

fn output_text(output: &Output) -> String {
    format!(
        "status={:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

const BASE_VALID_CONTRACT: &str = r#"
# Direct Tokio completion-record contract fixture

This fixture describes the direct completion-driven async memmove architecture.
It is a submit-now / complete-later ENQCMD model.
The canonical request constructor is AsyncMemmoveRequest::new(source: Bytes, destination: BytesMut).
Direct Tokio v1 is ENQCMD-oriented and Attempt ENQCMD submission is part of the flow.
The implementation dynamically creates/registers one completion record for each submitted request.
PendingMemmoveOp owns an aligned DSA descriptor, an aligned DSA completion record, the source `Bytes`, and the destination `BytesMut`.
CompletionMonitor will scan pending completion records and the future resolves only when the monitor observes terminal completion state.
The monitor uses yield_now plus adaptive backoff with a bounded or adaptive policy.
Page-fault retry behavior from the synchronous path remains part of the direct async contract.
Retries keep the same logical `PendingMemmoveOp` pending until terminal success or failure.
Cancellation after acceptance records dropped-receiver lifecycle classification and dropped-receiver cleanup.
Shutdown has two distinct phases and exposes shutdown classification.
This covers R002, R003, and R008.
Expected host-free proof obligations include fake completion-record transitions, fake ENQCMD accept/reject behavior, fake retry completion snapshots, dropped-receiver cleanup, shutdown classification, and destination-length assertions.
"#;

#[test]
fn tracked_direct_async_contract_passes_verifier() {
    let output = run_verifier(None);
    let diagnostics = output_text(&output);

    assert!(
        output.status.success(),
        "tracked direct async contract should pass verifier\n{diagnostics}"
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("verdict=pass"),
        "passing verifier output should include verdict=pass\n{diagnostics}"
    );
}

#[test]
fn override_fixture_missing_required_claim_reports_missing_label() {
    let missing_enqcmd_claims = BASE_VALID_CONTRACT
        .replace("Direct Tokio v1 is ENQCMD-oriented and Attempt ENQCMD submission is part of the flow.\n", "")
        .replace(
            "The implementation dynamically creates/registers one completion record for each submitted request.\n",
            "",
        )
        .replace(
            "CompletionMonitor will scan pending completion records and the future resolves only when the monitor observes terminal completion state.\n",
            "CompletionMonitor will scan pending completion records.\n",
        );
    let fixture_path = write_fixture("missing-required-claims", &missing_enqcmd_claims);
    let output = run_verifier(Some(&fixture_path));
    let diagnostics = output_text(&output);

    assert!(
        !output.status.success(),
        "malformed contract fixture should fail verifier\n{diagnostics}"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("missing=enqcmd_direct_submission"),
        "missing ENQCMD contract should report the missing label\n{diagnostics}"
    );
}

#[test]
fn override_path_isolated_from_tracked_report() {
    let isolated_bad_fixture = write_fixture(
        "isolated-bad-fixture",
        "direct completion-driven async memmove\n",
    );
    let output = run_verifier(Some(&isolated_bad_fixture));
    let diagnostics = output_text(&output);

    assert!(
        !output.status.success(),
        "override fixture should not be rescued by the tracked report\n{diagnostics}"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("missing=submit_now_complete_later"),
        "override fixture should fail on its own missing content rather than passing via the tracked report\n{diagnostics}"
    );
}

#[test]
fn stale_architecture_claims_are_rejected() {
    let forbidden_claims = [
        (
            "movdir64-fallback",
            "Direct Tokio v1 uses MOVDIR64 fallback.",
            "forbidden=movdir64_fallback_direct_v1",
        ),
        (
            "tonic-required",
            "Tonic is required before this direct substrate can be proven.",
            "forbidden=tonic_required_for_proof",
        ),
        (
            "preallocated-registry",
            "The preallocated completion-record registry is the v1 design.",
            "forbidden=preallocated_registry_v1_design",
        ),
        (
            "blocking-resolution",
            "Blocking memmove_uninit is the future-resolution mechanism.",
            "forbidden=blocking_memmove_uninit_future_resolution",
        ),
        (
            "payload-logging",
            "Payload byte logging is acceptable for diagnostics.",
            "forbidden=payload_byte_logging",
        ),
    ];

    for (name, forbidden_claim, expected_label) in forbidden_claims {
        let fixture_path =
            write_fixture(name, &format!("{BASE_VALID_CONTRACT}\n{forbidden_claim}\n"));
        let output = run_verifier(Some(&fixture_path));
        let diagnostics = output_text(&output);

        assert!(
            !output.status.success(),
            "forbidden claim fixture {name} should fail verifier\n{diagnostics}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected_label),
            "forbidden claim fixture {name} should report {expected_label}\n{diagnostics}"
        );
    }
}
