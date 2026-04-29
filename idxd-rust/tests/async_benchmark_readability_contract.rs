use std::path::PathBuf;
use std::process::{Command, Output};

fn guard_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scripts/check_m009_s03_tokio_benchmark_readability.sh")
}

fn run_guard() -> Output {
    Command::new("bash")
        .arg(guard_script())
        .output()
        .expect("S03 Tokio benchmark readability guard should launch")
}

fn output_text(output: &Output) -> String {
    format!(
        "status={:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn tracked_s03_tokio_benchmark_readability_guard_passes() {
    let output = run_guard();
    let diagnostics = output_text(&output);

    assert!(
        output.status.success(),
        "tracked S03 Tokio benchmark readability guard should pass\n{diagnostics}"
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("verdict=pass"),
        "passing guard output should include verdict=pass\n{diagnostics}"
    );
}
