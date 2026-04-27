use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("dsa-ffi-verifier-{name}-{nanos}"))
}

fn write_executable(path: &Path, content: &str) {
    fs::write(path, content).expect("script should be writable");
    let mut perms = fs::metadata(path)
        .expect("script metadata should exist")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("script should be executable");
}

fn verifier_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/verify_live_memmove.sh")
}

fn fake_launcher_env(capability_ok: bool) -> (PathBuf, PathBuf, String) {
    let temp_root = unique_temp_path(if capability_ok {
        "launcher-ready"
    } else {
        "launcher-missing-cap"
    });
    let shim_dir = temp_root.join("bin");
    fs::create_dir_all(&shim_dir).expect("shim dir should be creatable");

    let launcher_path = temp_root.join("dsa_launcher");
    write_executable(
        &launcher_path,
        "#!/usr/bin/env bash\nset -euo pipefail\nexec \"$@\"\n",
    );

    write_executable(
        &shim_dir.join("devenv"),
        "#!/usr/bin/env bash
set -euo pipefail
if [[ ${1:-} != shell || ${2:-} != -- || ${3:-} != launch ]]; then
  echo \"unexpected devenv invocation: $*\" >&2
  exit 90
fi
shift 3
exec \"$@\"
",
    );

    let getcap_output = if capability_ok {
        format!("{} cap_sys_rawio+eip\n", launcher_path.display())
    } else {
        format!("{} cap_net_raw+eip\n", launcher_path.display())
    };
    write_executable(
        &shim_dir.join("getcap"),
        &format!(
            "#!/usr/bin/env bash
set -euo pipefail
printf '%s' {:?}
",
            getcap_output
        ),
    );

    let mut path_entries = vec![shim_dir.display().to_string()];
    if let Some(existing) = std::env::var_os("PATH") {
        path_entries.push(existing.to_string_lossy().into_owned());
    }
    let joined_path = path_entries.join(":");

    (temp_root, launcher_path, joined_path)
}

#[test]
fn verifier_fails_preflight_when_launcher_capability_is_missing() {
    let (_temp_root, launcher_path, path_override) = fake_launcher_env(false);
    let output_dir = unique_temp_path("missing-cap-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let output = Command::new("bash")
        .arg(verifier_script())
        .env("PATH", path_override)
        .env("DSA_FFI_VERIFY_SKIP_BUILD", "1")
        .env("DSA_FFI_VERIFY_BINARY", env!("CARGO_BIN_EXE_live_memmove"))
        .env("DSA_FFI_VERIFY_DEVICE", "/dev/dsa/does-not-exist")
        .env("DSA_FFI_VERIFY_LAUNCHER_PATH", &launcher_path)
        .env("DSA_FFI_VERIFY_OUTPUT_DIR", &output_dir)
        .output()
        .expect("verifier should launch");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"));
    assert!(stdout.contains("verdict=expected_failure"));
    assert!(stdout.contains("failure_phase=preflight"));
    assert!(stdout.contains("launcher_status=missing_capability"));
    assert!(stdout.contains(&format!("launcher_path={}", launcher_path.display())));
}

#[test]
fn verifier_preserves_queue_open_failure_and_artifact_paths() {
    let (_temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let output_dir = unique_temp_path("queue-open-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let output = Command::new("bash")
        .arg(verifier_script())
        .env("PATH", path_override)
        .env("DSA_FFI_VERIFY_SKIP_BUILD", "1")
        .env("DSA_FFI_VERIFY_BINARY", env!("CARGO_BIN_EXE_live_memmove"))
        .env("DSA_FFI_VERIFY_DEVICE", "/dev/dsa/does-not-exist")
        .env("DSA_FFI_VERIFY_LAUNCHER_PATH", &launcher_path)
        .env("DSA_FFI_VERIFY_OUTPUT_DIR", &output_dir)
        .output()
        .expect("verifier should launch");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phase=done"));
    assert!(stdout.contains("verdict=expected_failure"));
    assert!(stdout.contains("failure_phase=runtime"));
    assert!(stdout.contains("launcher_status=ready"));
    assert!(stdout.contains("validation_phase=queue_open"));
    assert!(stdout.contains("validation_error_kind=queue_open"));
    assert!(stdout.contains(&format!(
        "artifact={}",
        output_dir.join("live_memmove.json").display()
    )));
    assert!(stdout.contains(&format!(
        "stdout={}",
        output_dir.join("live_memmove.stdout").display()
    )));
    assert!(stdout.contains(&format!(
        "stderr={}",
        output_dir.join("live_memmove.stderr").display()
    )));
}

#[test]
fn verifier_rejects_malformed_artifact_output() {
    let (temp_root, launcher_path, path_override) = fake_launcher_env(true);
    let output_dir = unique_temp_path("malformed-output");
    fs::create_dir_all(&output_dir).expect("output dir should be creatable");

    let fake_binary = temp_root.join("fake_live_memmove");
    write_executable(
        &fake_binary,
        "#!/usr/bin/env bash
set -euo pipefail
if [[ ${1:-} == --bytes && ${2:-} == abc ]]; then
  echo 'live_memmove: invalid value `abc` for `--bytes`; expected a positive integer' >&2
  exit 2
fi
artifact=
while [[ $# -gt 0 ]]; do
  case \"$1\" in
    --artifact)
      artifact=$2
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done
printf '{\"ok\":' > \"${artifact}\"
printf '{\"ok\":\n'
exit 1
",
    );

    let output = Command::new("bash")
        .arg(verifier_script())
        .env("PATH", path_override)
        .env("DSA_FFI_VERIFY_SKIP_BUILD", "1")
        .env("DSA_FFI_VERIFY_BINARY", &fake_binary)
        .env("DSA_FFI_VERIFY_DEVICE", "/dev/dsa/does-not-exist")
        .env("DSA_FFI_VERIFY_LAUNCHER_PATH", &launcher_path)
        .env("DSA_FFI_VERIFY_OUTPUT_DIR", &output_dir)
        .output()
        .expect("verifier should launch");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("phase=artifact_validation"));
    assert!(stderr.contains("launcher_status=ready"));
    assert!(stderr.contains(&format!(
        "artifact={}",
        output_dir.join("live_memmove.json").display()
    )));
}
