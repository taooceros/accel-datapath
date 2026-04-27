# dsa-ffi

`dsa-ffi` is the crate-local proof surface for the first real Rust DSA memmove path in this repo.
It does one thing truthfully: open one Intel DSA work queue, submit one real memmove, verify the copied bytes, and report the exact failure class instead of silently falling back to software.

## Who this is for

This README is for an engineer landing cold on the crate who needs one command to answer: "did the real hardware path run, and if not, where did it fail?"

After reading, you should be able to:

1. run the hardware-backed verifier on a prepared host,
2. run the proof binaries directly when you need a narrower repro, and
3. interpret the shared-handle lifecycle, worker, and validation failure classes without cross-referencing milestone notes.

## What lives here

- `DsaSession` is the only live submission path.
- `AsyncDsaSession` is the explicit lifecycle owner for the async path.
- `AsyncDsaHandle` is the only cloneable Tokio-facing surface. Cloning it shares one worker-owned `DsaSession`; it never duplicates hardware ownership.
- `live_memmove` is the crate-local synchronous validation binary.
- `await_memmove` is the crate-local async validation binary that exercises the public owner-plus-handle contract.
- `verify_live_memmove.sh` and `verify_async_memmove.sh` are the operational verifiers that wrap the binaries in the repo's `launch` capability flow and check the machine-readable artifacts they emit.

## Prerequisites

You need a host that is already prepared for user-space DSA access:

- a visible DSA work queue such as `/dev/dsa/wq0.0`,
- `devenv` on `PATH`,
- `python3` and `timeout`, and
- `tools/build/dsa_launcher` built with `cap_sys_rawio+eip`.

The repo's launcher background and capability model are documented in the launcher docs under `tools/`.

## Choose the proof path

Use the synchronous proof path when you are isolating the raw crate-owned DSA memmove contract:

```bash
bash dsa-ffi/scripts/verify_live_memmove.sh
```

Use the async proof path when you need to prove that ordinary Tokio callers can clone a handle, await real work, and still distinguish owner shutdown, worker failure, and wrapped validation errors:

```bash
bash dsa-ffi/scripts/verify_async_memmove.sh
```

In short:

- **`live_memmove`** answers "did the direct `DsaSession` path behave truthfully?"
- **`await_memmove`** answers "did the public async owner-plus-handle surface preserve truthful lifecycle-vs-worker-vs-validation failures?"

## Async ownership model

The async surface is intentionally split in two.

- `AsyncDsaSession` owns the worker thread and therefore owns shutdown.
- `AsyncDsaHandle` is what Tokio tasks clone and await.
- Ordinary Tokio composition such as `tokio::join!` or spawned tasks still uses that same cloneable handle surface; cloned handles do not create extra sessions or extra hardware owners.
- All submissions funnel through one worker-owned `DsaSession`, so overlapped requests queue FIFO and execute one at a time even when multiple Tokio tasks are awaiting them concurrently.
- Once a request has crossed that enqueue boundary, aborting or dropping the awaiting Tokio task does not cancel the worker-side memmove. The worker still finishes the request, and later submissions can continue using the shared handle.
- Shutdown is drain-then-stop: work that was already queued drains before the worker thread exits, and submissions attempted after shutdown are rejected with `owner_shutdown`.
- The worker thread owns the real `DsaSession`, so all hardware access still crosses one explicit boundary as owned requests and owned replies.

That split matters operationally because it makes failure interpretation honest:

- if the owner shuts down before a reply exists, the async proof surface reports `error_kind=lifecycle_failure` with `lifecycle_failure_kind=owner_shutdown`,
- if the worker path breaks before a reply exists, it reports `error_kind=worker_failure` with a `worker_failure_kind`, and
- if the worker successfully propagates a real memmove problem, it reports `error_kind=validation_failure` plus the underlying validation phase and error kind.

This is why the async verifier is the main operator entrypoint for the shared Tokio handle proof path rather than just another wrapper around the synchronous binary.

## One-command truthful proof

From the `accel-rpc` workspace root, run either verifier:

```bash
bash dsa-ffi/scripts/verify_live_memmove.sh
bash dsa-ffi/scripts/verify_async_memmove.sh
```

From the repo root, equivalent wrapper entrypoints are also available:

```bash
bash accel-rpc/dsa-ffi/scripts/verify_live_memmove.sh
bash accel-rpc/dsa-ffi/scripts/verify_async_memmove.sh
```

What both verifiers do:

1. find a work queue or use `DSA_FFI_VERIFY_DEVICE`,
2. check launcher prerequisites before attempting hardware work,
3. build the selected proof binary unless `DSA_FFI_VERIFY_SKIP_BUILD=1`,
4. run the binary via `devenv shell -- launch ...`,
5. write a JSON artifact plus captured stdout/stderr into a temp output directory, and
6. reject malformed, incomplete, or contradictory artifacts.

A successful verifier execution always ends with a `phase=done` line. When hardware execution succeeds it includes `verdict=pass`; when the host or queue is not ready but the failure was classified truthfully it includes `verdict=expected_failure` plus the preserved failure metadata.

The synchronous verifier final line includes:

- `device_path`
- `requested_bytes`
- `page_fault_retries`
- `final_status`
- `launcher_status`
- `artifact`
- `stdout`
- `stderr`

The async verifier final line includes those same fields plus:

- `error_kind`
- `async_lifecycle_failure_kind`
- `async_worker_failure_kind`
- `validation_phase`
- `validation_error_kind`

Examples:

```text
[verify_live_memmove] phase=done ... device_path=/dev/dsa/wq0.0 requested_bytes=64 page_fault_retries=0 final_status=0x01 validation_phase=completed verdict=pass
[verify_async_memmove] phase=done ... device_path=/dev/dsa/wq0.0 requested_bytes=64 page_fault_retries=0 final_status=0x01 error_kind=null async_lifecycle_failure_kind=null async_worker_failure_kind=null validation_phase=completed validation_error_kind=null verdict=pass
[verify_async_memmove] phase=done ... device_path=/dev/dsa/wq0.0 requested_bytes=64 error_kind=lifecycle_failure async_lifecycle_failure_kind=owner_shutdown async_worker_failure_kind=null validation_phase=null validation_error_kind=null verdict=expected_failure
```

On an unprepared host, the verifier still exits successfully when it can classify the failure honestly. For example, a launcher without `cap_sys_rawio+eip` ends with:

```text
[verify_async_memmove] phase=done ... verdict=expected_failure failure_phase=preflight launcher_status=missing_capability launcher_path=/path/to/dsa_launcher
```

## Direct binary usage

When you already know the launcher/capability setup is correct and want a smaller repro, run the binaries directly from the `accel-rpc` workspace root.

### Synchronous proof binary

```bash
cargo run -p dsa-ffi --bin live_memmove -- \
  --device /dev/dsa/wq0.0 \
  --bytes 64 \
  --format json \
  --artifact /tmp/live_memmove.json
```

The synchronous binary always reports these fields:

- `ok`
- `device_path`
- `requested_bytes`
- `page_fault_retries`
- `final_status`
- `phase`
- `error_kind`
- `message`

### Async proof binary

```bash
cargo run -p dsa-ffi --bin await_memmove -- \
  --device /dev/dsa/wq0.0 \
  --bytes 64 \
  --format json \
  --artifact /tmp/await_memmove.json
```

The async binary always reports these fields:

- `ok`
- `device_path`
- `requested_bytes`
- `page_fault_retries`
- `final_status`
- `phase`
- `error_kind`
- `lifecycle_failure_kind`
- `worker_failure_kind`
- `validation_phase`
- `validation_error_kind`
- `message`

On success, `message` includes copied-bytes proof in the form `verified N copied bytes via async wrapper on ...`.

## Failure classes

The verifiers preserve two layers of failure information.

### Launcher and verifier failures

These come from the shell wrapper before the memmove result is trusted:

- `launcher_status=missing_work_queue` — no default `/dev/dsa/wq*` node was found and no explicit device was provided.
- `launcher_status=missing_devenv` — the launch wrapper cannot be entered.
- `launcher_status=missing_launcher` — `tools/build/dsa_launcher` is absent or not executable.
- `launcher_status=missing_capability` — the launcher exists but does not carry `cap_sys_rawio`.
- `launcher_status=contradictory_overrides` — a binary override was supplied without `DSA_FFI_VERIFY_SKIP_BUILD=1`, which would otherwise build one binary and execute another.
- `phase=preflight` or `phase=runtime` with a timeout message — the launch-wrapped validation run exceeded the configured timeout while still preserving the output paths and launcher state.
- `phase=artifact_validation` — the binary ran, but the artifact was missing, malformed, incomplete, inconsistent with stdout, or internally contradictory.

### Validation failure classes from the Rust binaries

These come from the Rust binaries and are preserved by the verifiers as validation metadata:

- `invalid_device_path` — the requested device path was empty or malformed before queue-open.
- `queue_open` — opening the work queue failed.
- `completion_timeout` — descriptor completion polling timed out.
- `malformed_completion` — hardware completion fields were internally inconsistent.
- `page_fault_retry_exhausted` — recoverable page-fault retries were exhausted.
- `completion_status` — the completion status byte reported a real failure.
- `byte_mismatch` — completion reported success, but the copied bytes did not match.

In async verifier output, wrapper-only failures stay separate:

- `error_kind=lifecycle_failure` with `async_lifecycle_failure_kind=owner_shutdown` means the explicit owner closed the shared handle before a trustworthy validation result existed.
- `error_kind=worker_failure` with `async_worker_failure_kind=worker_init_closed|request_channel_closed|response_channel_closed|worker_panicked` means the async shell failed before a trustworthy validation result existed.
- `error_kind=validation_failure` means the wrapper successfully propagated the underlying `MemmoveError`, which is preserved as `validation_phase` and `validation_error_kind`.

If you need the exact machine-readable payload, inspect the JSON artifact next to the captured stdout/stderr files. The verifier treats any disagreement between stdout and the artifact as a hard `phase=artifact_validation` failure.

## Useful overrides

The verifiers are intentionally configurable so they can be used both on real hosts and in regression tests:

- `DSA_FFI_VERIFY_DEVICE` — explicit work-queue path.
- `DSA_FFI_VERIFY_BYTES` — transfer size; defaults to `64` for the minimal proof run.
- `DSA_FFI_VERIFY_OUTPUT_DIR` — keep artifacts in a known directory instead of a fresh temp dir.
- `DSA_FFI_VERIFY_PREFLIGHT_TIMEOUT` and `DSA_FFI_VERIFY_RUN_TIMEOUT` — bound stuck phases separately.
- `DSA_FFI_VERIFY_SKIP_BUILD=1` — reuse an already-built proof binary.
- `DSA_FFI_VERIFY_BINARY` — override the proof binary path. Pair this with `DSA_FFI_VERIFY_SKIP_BUILD=1`.
- `DSA_FFI_VERIFY_LAUNCHER_PATH` — override the launcher path.

These are inputs to the verifiers themselves; the verifiers will fail if they depend on missing or contradictory knobs outside this list.

## Fast checks

From the repo root:

```bash
cd accel-rpc && cargo test -p dsa-ffi --test validation_cli_contract -- --nocapture
cd accel-rpc && cargo test -p dsa-ffi --test tokio_handle_contract --test async_validation_cli_contract --test async_verifier_contract -- --nocapture
bash accel-rpc/dsa-ffi/scripts/verify_live_memmove.sh
bash accel-rpc/dsa-ffi/scripts/verify_async_memmove.sh
```

The Tokio-handle and CLI contract tests exercise the non-hardware schemas for the public async surface. The shell verifiers are the truthful end-to-end proof commands for prepared hosts and the expected-failure proof commands for unprepared ones.
