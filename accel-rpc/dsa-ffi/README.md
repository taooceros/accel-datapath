# dsa-ffi

`dsa-ffi` is the crate-local proof surface for the first real Rust DSA memmove path in this repo.
It does one thing truthfully: open one Intel DSA work queue, submit one real memmove, verify the copied bytes, and report the exact failure class instead of silently falling back to software.

## Who this is for

This README is for an engineer landing cold on the crate who needs one command to answer: "did the real hardware path run, and if not, where did it fail?"

After reading, you should be able to:

1. run the hardware-backed verifier on a prepared host,
2. run the binary directly when you need a narrower repro, and
3. interpret the failure class without cross-referencing milestone notes.

## What lives here

- `DsaSession` is the only live submission path.
- `live_memmove` is the crate-local validation binary.
- `scripts/verify_live_memmove.sh` is the operational verifier that wraps the binary in the repo's `launch` capability flow and checks the machine-readable artifact it emits.

## Prerequisites

You need a host that is already prepared for user-space DSA access:

- a visible DSA work queue such as `/dev/dsa/wq0.0`,
- `devenv` on `PATH`,
- `python3` and `timeout`, and
- `tools/build/dsa_launcher` built with `cap_sys_rawio+eip`.

The repo's launcher background and capability model are documented in the launcher docs under `tools/`.

## One-command truthful proof

From the `accel-rpc` workspace root, run:

```bash
bash dsa-ffi/scripts/verify_live_memmove.sh
```

What the verifier does:

1. finds a work queue or uses `DSA_FFI_VERIFY_DEVICE`,
2. checks launcher prerequisites before attempting hardware work,
3. builds `live_memmove` unless `DSA_FFI_VERIFY_SKIP_BUILD=1`,
4. runs the binary via `devenv shell -- launch ...`,
5. writes a JSON artifact plus captured stdout/stderr into a temp output directory, and
6. rejects malformed, incomplete, or contradictory artifacts.

A successful run ends with a `phase=done` line that includes:

- `device_path`
- `requested_bytes`
- `page_fault_retries`
- `final_status`
- `launcher_status`
- `artifact`
- `stdout`
- `stderr`

Example:

```text
[verify_live_memmove] phase=done ... device_path=/dev/dsa/wq0.0 requested_bytes=64 page_fault_retries=0 final_status=0x01 validation_phase=completed verdict=pass
```

## Direct binary usage

When you already know the launcher/capability setup is correct and want a smaller repro, run the binary directly from the `accel-rpc` workspace root:

```bash
cargo run -p dsa-ffi --bin live_memmove -- \
  --device /dev/dsa/wq0.0 \
  --bytes 64 \
  --format json \
  --artifact /tmp/live_memmove.json
```

The binary always reports these fields:

- `ok`
- `device_path`
- `requested_bytes`
- `page_fault_retries`
- `final_status`
- `phase`
- `error_kind`
- `message`

On success, `message` includes copied-bytes proof in the form `verified N copied bytes via ...`.

## Failure classes

The verifier preserves two layers of failure information.

### Launcher and verifier failures

These come from the shell wrapper before the memmove result is trusted:

- `launcher_status=missing_work_queue` — no default `/dev/dsa/wq*` node was found and no explicit device was provided.
- `launcher_status=missing_devenv` — the launch wrapper cannot be entered.
- `launcher_status=missing_launcher` — `tools/build/dsa_launcher` is absent or not executable.
- `launcher_status=missing_capability` — the launcher exists but does not carry `cap_sys_rawio`.
- `phase=runtime_timeout` — the launch-wrapped validation run hung past the configured timeout.
- `phase=artifact_validation` — the binary ran, but the artifact was missing, malformed, incomplete, or inconsistent with stdout.

### Validation failure classes from `live_memmove`

These come from the Rust binary and are preserved by the verifier as `validation_error_kind=...`:

- `queue_open` — opening the work queue failed.
- `completion_timeout` — descriptor completion polling timed out.
- `malformed_completion` — hardware completion fields were internally inconsistent.
- `page_fault_retry_exhausted` — recoverable page-fault retries were exhausted.
- `completion_status` — the completion status byte reported a real failure.
- `byte_mismatch` — completion reported success, but the copied bytes did not match.

In verifier output, the Rust-side classification shows up as `validation_phase` and `validation_error_kind`, alongside the artifact/stdout/stderr paths for follow-up inspection.

## Useful overrides

The verifier is intentionally configurable so it can be used both on real hosts and in regression tests:

- `DSA_FFI_VERIFY_DEVICE` — explicit work-queue path.
- `DSA_FFI_VERIFY_BYTES` — transfer size; defaults to `64` for the minimal proof run.
- `DSA_FFI_VERIFY_OUTPUT_DIR` — keep artifacts in a known directory instead of a fresh temp dir.
- `DSA_FFI_VERIFY_PREFLIGHT_TIMEOUT` and `DSA_FFI_VERIFY_RUN_TIMEOUT` — bound stuck phases separately.
- `DSA_FFI_VERIFY_SKIP_BUILD=1` — reuse an already-built `live_memmove` binary.
- `DSA_FFI_VERIFY_BINARY` — override the binary path.
- `DSA_FFI_VERIFY_LAUNCHER_PATH` — override the launcher path.

These are inputs to the verifier itself; the verifier will fail if it depends on missing, undocumented knobs outside this list.

## Fast checks

From the repo root:

```bash
cd accel-rpc && cargo test -p dsa-ffi --test validation_cli_contract -- --nocapture
bash accel-rpc/dsa-ffi/scripts/verify_live_memmove.sh
```

The test command exercises the non-hardware CLI contract. The shell verifier is the truthful end-to-end proof command for a prepared host.
