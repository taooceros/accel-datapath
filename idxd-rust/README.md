# idxd-rust

`idxd-rust` is the crate-local proof surface for real Rust Intel IDXD data paths in this repo.
It truthfully opens work queues, submits representative operations, verifies or reports the exact failure class, and avoids silently falling back to software.

The mature path is DSA memmove. The generic `IdxdSession<Accel>` seam now also exposes representative operations for DSA memmove and IAX/IAA crc64 so downstream hardware proof can exercise both accelerator families without adopting a broad operation framework.

## Who this is for

This README is for an engineer landing cold on the crate who needs one command to answer: "did the real hardware path run, and if not, where did it fail?"

After reading, you should be able to:

1. run the hardware-backed verifier on a prepared host,
2. run the proof binaries directly when you need a narrower repro, and
3. interpret the shared-handle lifecycle, direct-runtime, legacy worker-fixture, and validation failure classes without cross-referencing milestone notes.

## What lives here

- `DsaSession` is the established DSA memmove submission path. It remains a separate public type and is not an alias for the generic session seam.
- `IdxdSession<Dsa>` and `IdxdSession<Iax>` are the concrete marker-family uses of the lean `IdxdSession<Accel>` operation seam. `IdxdSession<Dsa>::memmove` uses the same blocking DSA lifecycle as `DsaSession`; `IdxdSession<Iax>::crc64` and the `Iaa` spelling use the representative IAX/IAA crc64 lifecycle. This is intentionally narrow coverage, not a public operation hierarchy or a full accelerator runtime.
- `AsyncDsaSession` is the explicit lifecycle owner for the async path.
- `AsyncDsaHandle` is the only cloneable Tokio-facing surface. Cloning it shares one direct async runtime with one mapped work-queue portal and completion monitor; it never duplicates hardware ownership.
- `live_memmove` is the crate-local synchronous validation binary for the legacy direct `DsaSession` proof path.
- `live_idxd_op` is the narrow S03 representative proof binary for generic `IdxdSession<Dsa>` memmove and `IdxdSession<Iax>` crc64 runs.
- `await_memmove` is the crate-local async validation binary that exercises the public owner-plus-handle contract.
- `tokio_memmove_bench` is the standalone Tokio-only direct async benchmark/proof binary. It emits JSON-first evidence for single latency, concurrent submissions, and fixed-duration throughput.
- `verify_live_memmove.sh`, `verify_async_memmove.sh`, `verify_tokio_memmove_bench.sh`, and `verify_idxd_representative_ops.sh` are the operational verifiers that wrap hardware proof binaries in the repo's `launch` capability flow and check the machine-readable artifacts they emit.

## Prerequisites

You need a host that is already prepared for user-space DSA access:

- a visible DSA work queue such as `/dev/dsa/wq0.0`,
- `devenv` on `PATH`,
- `python3` and `timeout`, and
- `tools/build/dsa_launcher` built with `cap_sys_rawio+eip`.

The repo's launcher background and capability model are documented in the launcher docs under `tools/`.

## S03 representative hardware proof

Use the representative proof when you need to show that the generic session seam runs one DSA operation and one IAX/IAA operation on real work queues. It is not a benchmark: it records whether representative hardware work ran and how failures were classified, not latency or throughput numbers. `hw-eval` remains diagnostic and benchmark prior art, and `live_memmove` remains the legacy direct `DsaSession` proof. S03 closure evidence must come from `live_idxd_op` or `verify_idxd_representative_ops.sh` exercising `IdxdSession<Accel>`.

The verifier is the preferred operator entrypoint because it builds the proof binary, runs it through the launcher flow, validates artifacts, and prints machine-readable phase lines:

```bash
IDXD_RUST_VERIFY_DSA_DEVICE=/dev/dsa/wq0.0 \
IDXD_RUST_VERIFY_IAX_DEVICE=/dev/iax/wq1.0 \
IDXD_RUST_VERIFY_BYTES=64 \
bash idxd-rust/scripts/verify_idxd_representative_ops.sh
```

The required target roles are `dsa-memmove` on a DSA work queue and `iax-crc64` on an IAX/IAA work queue. Set `IDXD_RUST_VERIFY_DSA_SHARED_DEVICE=/dev/dsa/wq0.1` when you also want the optional shared-DSA target; if unset, the verifier uses a second discovered DSA work queue when one is visible.

Useful verifier knobs:

- `IDXD_RUST_VERIFY_DSA_DEVICE` — required DSA target override when discovery should not pick the first `/dev/dsa/wq*`.
- `IDXD_RUST_VERIFY_IAX_DEVICE` — required IAX/IAA target override when discovery should not pick the first `/dev/iax/wq*`.
- `IDXD_RUST_VERIFY_DSA_SHARED_DEVICE` — optional second DSA target for the shared-WQ representative check.
- `IDXD_RUST_VERIFY_BYTES` — requested bytes per operation; the verifier default is the small proof size `64`.
- `IDXD_RUST_VERIFY_OUTPUT_DIR` — stable directory for JSON artifacts plus captured stdout/stderr.
- `IDXD_RUST_VERIFY_PREFLIGHT_TIMEOUT` and `IDXD_RUST_VERIFY_RUN_TIMEOUT` — separate bounds for launcher preflight and runtime operation phases.
- `IDXD_RUST_VERIFY_PROFILE` — Cargo profile used to build `live_idxd_op` before execution.
- `IDXD_RUST_VERIFY_SKIP_BUILD=1`, `IDXD_RUST_VERIFY_BINARY`, and `IDXD_RUST_VERIFY_LAUNCHER_PATH` — reuse an existing proof binary or launcher. Pair a binary override with `IDXD_RUST_VERIFY_SKIP_BUILD=1` so the verifier does not build one binary and execute another.

Each `live_idxd_op` JSON artifact uses the same no-payload schema for success and failure:

- `ok`
- `operation`
- `accelerator`
- `device_path`
- `requested_bytes`
- `page_fault_retries`
- `final_status`
- `phase`
- `error_kind`
- `completion_error_code`
- `invalid_flags`
- `fault_addr`
- `crc64`
- `expected_crc64`
- `crc64_verified`
- `message`

The verifier also prints phase lines with `launcher_status`, `targets`, `artifact_paths`, `stdout_paths`, and `stderr_paths`. A final `verdict=pass` line means the required representative targets completed and the artifacts matched stdout. A final `verdict=expected_failure` line means the host, launcher, queue, timeout, or operation failure was classified truthfully; that is useful diagnostic output on an unprepared host, but it is not S03 closure evidence and does not satisfy the prepared-host representative proof requirement.

The no-payload rule is strict. Reports may include operation metadata, status values, retry counts, CRC scalars, and artifact paths; they must not dump raw buffers. The verifier rejects malformed JSON, stdout/artifact disagreement, contradictory exit statuses, and payload dump fields before downstream evidence can consume the output.

For a narrower repro, run the proof binary directly against one target:

```bash
cargo run -p idxd-rust --bin live_idxd_op -- \
  --op dsa-memmove \
  --device /dev/dsa/wq0.0 \
  --bytes 64 \
  --format json \
  --artifact /tmp/live_idxd_op-dsa.json

cargo run -p idxd-rust --bin live_idxd_op -- \
  --op iax-crc64 \
  --device /dev/iax/wq1.0 \
  --bytes 64 \
  --format json \
  --artifact /tmp/live_idxd_op-iax.json
```

Direct binary runs are useful for debugging one target. The verifier remains the stable S03/S04 handoff contract because it records all target roles and artifact/stdout/stderr paths in one validated output stream.

## Choose the proof path

Use the synchronous proof path when you are isolating the raw crate-owned DSA memmove contract:

```bash
bash idxd-rust/scripts/verify_live_memmove.sh
```

Use the async proof path when you need to prove that ordinary Tokio callers can clone a handle, await real direct ENQCMD-submitted work, and still distinguish owner shutdown, direct runtime failures, legacy worker-fixture failures, and wrapped validation errors:

```bash
bash idxd-rust/scripts/verify_async_memmove.sh
```

Use the benchmark proof path when you need JSON-first evidence for direct Tokio memmove latency, concurrent submissions, and bounded throughput. The verifier defaults are intentionally short for operator checks:

```bash
bash idxd-rust/scripts/verify_tokio_memmove_bench.sh
```

Use the S04 collection workflow when you need a reviewer-ready evidence directory with focused command logs, verifier output directories, and a manifest:

```bash
bash idxd-rust/scripts/collect_tokio_memmove_evidence.sh
```

By default it writes a timestamped directory under `target/m007-s04-evidence/`. Set `M007_S04_EVIDENCE_OUTPUT_DIR` when you need a stable rerun path, and use the existing `IDXD_RUST_VERIFY_*` knobs to pass release-profile workload settings through to the underlying verifier.

For release-profile S04 hardware evidence, keep the same verifier or collection workflow but raise the profile and workload knobs explicitly:

```bash
IDXD_RUST_VERIFY_PROFILE=release \
IDXD_RUST_VERIFY_BYTES=4096 \
IDXD_RUST_VERIFY_ITERATIONS=1000 \
IDXD_RUST_VERIFY_CONCURRENCY=16 \
IDXD_RUST_VERIFY_DURATION_MS=5000 \
bash idxd-rust/scripts/verify_tokio_memmove_bench.sh
```

Use the software diagnostic benchmark when you need a host-free schema and Tokio runtime sanity check:

```bash
IDXD_RUST_VERIFY_BACKEND=software bash idxd-rust/scripts/verify_tokio_memmove_bench.sh
```

Software diagnostic artifacts are deliberately marked `claim_eligible=false`. They prove the benchmark contract and async control flow, not hardware acceleration. S04 hardware evidence must come from `backend=hardware`, `claim_eligible=true`, direct async rows, and the paired `direct_sync` comparison row.

Future worker-runtime planners should read the S05 worker-readiness handoff at `docs/report/architecture/010.worker_runtime_readiness_handoff.md` before treating M007 as execution or claim evidence. M007 is planning-ready for worker batching, MOVDIR64/MOVDIR64B, registry/pool, and Tonic/RPC work; execution readiness and claim readiness still require prepared-host verifier evidence.

Use the downstream async-handle proof path when you need to prove that a repo-local package outside `idxd-rust` can consume the public async owner/handle API from ordinary Tokio code:

```bash
bash accel-rpc/tonic-profile/scripts/verify_downstream_async_handle.sh
```

The downstream proof runs `tonic-profile`'s `downstream_async_handle` binary and validates `proof_seam=downstream_async_handle`, `consumer_package=tonic-profile`, `binding_package=idxd-rust`, `composition=tokio_join`, and the typed lifecycle/worker/validation fields. It deliberately does not call `idxd-rust`'s crate-local `await_memmove` binary, and it does not make `tonic-profile`'s `custom_codec.rs` an async integration seam.

In short:

- **`live_memmove`** answers "did the direct `DsaSession` path behave truthfully?"
- **`await_memmove`** answers "did the public async owner-plus-handle surface preserve truthful lifecycle-vs-direct-runtime-vs-validation failures?"
- **`tonic-profile` `downstream_async_handle`** answers "can a downstream Tokio consumer outside the binding crate use cloned public handles for real awaited operations without changing the synchronous codec seam?"

## Async ownership model

The async surface is intentionally split in two.

- `AsyncDsaSession` owns the direct async runtime and therefore owns shutdown.
- `AsyncDsaHandle` is what Tokio tasks clone and await.
- `AsyncMemmoveRequest` is the canonical async request shape. It owns both the source bytes and the destination buffer before it enters the queue.
- `AsyncMemmoveResult` returns the explicit owned destination buffer plus the validation report; callers should inspect `report.requested_bytes` to distinguish requested source bytes from any extra destination capacity.
- Destination allocation is explicit at the call site. The v1 public async API does not provide an allocation convenience helper or a borrowed copy-back helper; callers choose the destination capacity, submit the owned request, and receive the same destination ownership back in the result.
- Destination length advances only after successful completion. The direct runtime writes into spare capacity and the result exposes the initialized prefix only after validation succeeds; failed requests keep the rejected buffers available through the typed error path.
- Ordinary Tokio composition such as `tokio::join!` or spawned tasks still uses that same cloneable handle surface; cloned handles do not create extra sessions or extra hardware owners. Build an owned `AsyncMemmoveRequest` and call `memmove` when work must cross a task boundary.
- Direct async submissions use ENQCMD accept/reject semantics and operation-owned descriptor/completion/buffer state. A long-lived Tokio monitor observes per-request completion records and resolves futures only after terminal completion classification.
- Once a request has been accepted for hardware submission, aborting or dropping the awaiting Tokio task does not cancel the in-flight memmove. The monitor still observes terminal completion, keeps descriptor/completion/buffer state alive, and discards the result only if no receiver remains.
- Shutdown rejects new submissions with `owner_shutdown`. Direct operations that were already accepted remain owned by the runtime until their completion records are observed.
- The legacy blocking worker seam remains only as a hidden host-independent fixture path; `open` and `open_with_retries` do not silently fall back to synchronous `DsaSession::memmove_uninit`.
- Borrowed async zero-copy, software aggregation, batching, preallocated completion-record registries, and MOVDIR64 submission paths are future work. They are not part of the current v1 owner-plus-handle behavior.

A minimal owned async call looks like this:

```rust
use bytes::{Bytes, BytesMut};
use idxd_rust::{AsyncDsaSession, AsyncMemmoveRequest};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let owner = AsyncDsaSession::open("/dev/dsa/wq0.0")?;
let handle = owner.handle();

let source = Bytes::from_static(b"hello dsa");
let destination = BytesMut::with_capacity(source.len());
let request = AsyncMemmoveRequest::new(source, destination)?;
let result = handle.memmove(request).await?;

assert_eq!(&result.destination[..result.report.requested_bytes], b"hello dsa");
println!("copied {} bytes", result.report.requested_bytes);
# Ok(())
# }
```

That split matters operationally because it makes failure interpretation honest:

- if the owner shuts down before a reply exists, the async proof surface reports `error_kind=lifecycle_failure` with `lifecycle_failure_kind=owner_shutdown`,
- if the direct runtime rejects or loses a request before a trustworthy validation result exists, it reports `error_kind=direct_failure` with a `direct_failure_kind`,
- if a hidden legacy worker fixture breaks before a reply exists, it reports `error_kind=worker_failure` with a `worker_failure_kind`, and
- if the direct runtime successfully propagates a real memmove problem, it reports `error_kind=validation_failure` plus the underlying validation phase and error kind.

This is why the async verifier is the main operator entrypoint for the shared Tokio handle proof path rather than just another wrapper around the synchronous binary.

## One-command truthful proof

From the `accel-rpc` workspace root, run the hardware verifiers:

```bash
bash idxd-rust/scripts/verify_live_memmove.sh
bash idxd-rust/scripts/verify_async_memmove.sh
bash idxd-rust/scripts/verify_tokio_memmove_bench.sh
```

From the repo root, equivalent wrapper entrypoints are also available:

```bash
bash idxd-rust/scripts/verify_live_memmove.sh
bash idxd-rust/scripts/verify_async_memmove.sh
bash idxd-rust/scripts/verify_tokio_memmove_bench.sh
```

What the hardware verifiers do:

1. find a work queue or use `IDXD_RUST_VERIFY_DEVICE`,
2. check launcher prerequisites before attempting hardware work,
3. build the selected proof binary unless `IDXD_RUST_VERIFY_SKIP_BUILD=1`,
4. run the binary via `devenv shell -- launch ...`,
5. write a JSON artifact plus captured stdout/stderr into a temp output directory, and
6. reject malformed, incomplete, or contradictory artifacts.

`verify_tokio_memmove_bench.sh` also supports `IDXD_RUST_VERIFY_BACKEND=software`, which skips launcher preflight and validates the same benchmark schema as a non-claim-eligible diagnostic.

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

The benchmark verifier final line includes:

- `backend`
- `suite`
- `claim_eligible`
- `failure_class`
- `error_kind`
- `direct_failure_kind`
- `validation_phase`
- `validation_error_kind`
- `completed_operations`
- `failed_operations`
- `targets`
- `stdout`
- `stderr`

Examples:

```text
[verify_live_memmove] phase=done ... device_path=/dev/dsa/wq0.0 requested_bytes=64 page_fault_retries=0 final_status=0x01 validation_phase=completed verdict=pass
[verify_async_memmove] phase=done ... device_path=/dev/dsa/wq0.0 requested_bytes=64 page_fault_retries=0 final_status=0x01 error_kind=null async_lifecycle_failure_kind=null async_worker_failure_kind=null async_direct_failure_kind=null validation_phase=completed validation_error_kind=null verdict=pass
[verify_tokio_memmove_bench] phase=done ... backend=hardware suite=canonical claim_eligible=true targets=direct_async,direct_async,direct_async,direct_sync verdict=pass
[verify_tokio_memmove_bench] phase=done ... backend=software suite=canonical claim_eligible=false targets=software_direct_async_diagnostic,software_direct_async_diagnostic,software_direct_async_diagnostic verdict=pass
[verify_async_memmove] phase=done ... device_path=/dev/dsa/wq0.0 requested_bytes=64 error_kind=lifecycle_failure async_lifecycle_failure_kind=owner_shutdown async_worker_failure_kind=null async_direct_failure_kind=null validation_phase=null validation_error_kind=null verdict=expected_failure
```

On an unprepared host, the verifier still exits successfully when it can classify the failure honestly. For example, a launcher without `cap_sys_rawio+eip` ends with:

```text
[verify_async_memmove] phase=done ... verdict=expected_failure failure_phase=preflight launcher_status=missing_capability launcher_path=/path/to/dsa_launcher
```

## Direct binary usage

When you already know the launcher/capability setup is correct and want a smaller repro, run the binaries directly from the `accel-rpc` workspace root.

### Synchronous proof binary

```bash
cargo run -p idxd-rust --bin live_memmove -- \
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
cargo run -p idxd-rust --bin await_memmove -- \
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
- `direct_failure_kind`
- `validation_phase`
- `validation_error_kind`
- `message`

On success, `message` includes copied-bytes proof in the form `verified N copied bytes via direct async memmove on ...`.

## Failure classes

The verifiers preserve two layers of failure information.

### Launcher and verifier failures

These come from the shell wrapper before the memmove result is trusted:

- `launcher_status=missing_work_queue` — no default `/dev/dsa/wq*` node was found and no explicit device was provided.
- `launcher_status=missing_devenv` — the launch wrapper cannot be entered.
- `launcher_status=missing_launcher` — `tools/build/dsa_launcher` is absent or not executable.
- `launcher_status=missing_capability` — the launcher exists but does not carry `cap_sys_rawio`.
- `launcher_status=contradictory_overrides` — a binary override was supplied without `IDXD_RUST_VERIFY_SKIP_BUILD=1`, which would otherwise build one binary and execute another.
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

In async verifier output, async-shell failures stay separate from validation failures:

- `error_kind=lifecycle_failure` with `async_lifecycle_failure_kind=owner_shutdown` means the explicit owner closed the shared handle before a trustworthy validation result existed.
- `error_kind=direct_failure` with `async_direct_failure_kind=registration_closed|monitor_closed|submission_rejected|backpressure_exceeded|receiver_dropped|runtime_unavailable` means the direct runtime failed or classified an async lifecycle edge before a trustworthy validation result existed.
- `error_kind=worker_failure` with `async_worker_failure_kind=worker_init_closed|request_channel_closed|response_channel_closed|worker_panicked` is reserved for hidden legacy worker fixtures and should not appear on the public default path.
- `error_kind=validation_failure` means the async surface successfully propagated the underlying `MemmoveError`, which is preserved as `validation_phase` and `validation_error_kind`.

If you need the exact machine-readable payload, inspect the JSON artifact next to the captured stdout/stderr files. The verifier treats any disagreement between stdout and the artifact as a hard `phase=artifact_validation` failure.

## Useful overrides

The verifiers are intentionally configurable so they can be used both on real hosts and in regression tests:

- `IDXD_RUST_VERIFY_DEVICE` — explicit work-queue path.
- `IDXD_RUST_VERIFY_BYTES` — transfer size; defaults to `64` for the minimal proof run.
- `IDXD_RUST_VERIFY_OUTPUT_DIR` — keep artifacts in a known directory instead of a fresh temp dir.
- `IDXD_RUST_VERIFY_PREFLIGHT_TIMEOUT` and `IDXD_RUST_VERIFY_RUN_TIMEOUT` — bound stuck phases separately.
- `IDXD_RUST_VERIFY_SKIP_BUILD=1` — reuse an already-built proof binary.
- `IDXD_RUST_VERIFY_BINARY` — override the proof binary path. Pair this with `IDXD_RUST_VERIFY_SKIP_BUILD=1`.
- `IDXD_RUST_VERIFY_LAUNCHER_PATH` — override the launcher path.
- `IDXD_RUST_VERIFY_BACKEND` — for `tokio_memmove_bench`, choose `hardware` or the host-free `software` diagnostic backend.
- `IDXD_RUST_VERIFY_SUITE` — for `tokio_memmove_bench`, choose `canonical`, `latency`, `concurrency`, or `throughput`.
- `IDXD_RUST_VERIFY_ITERATIONS`, `IDXD_RUST_VERIFY_CONCURRENCY`, and `IDXD_RUST_VERIFY_DURATION_MS` — tune benchmark workload size while preserving bounded defaults.
- `IDXD_RUST_VERIFY_PROFILE` — choose the Cargo build profile. Use `release` for claim-oriented S04 benchmark evidence.

These are inputs to the verifiers themselves; the verifiers will fail if they depend on missing or contradictory knobs outside this list.

## Fast checks

From the repo root:

```bash
cd accel-rpc && cargo test -p idxd-rust --test validation_cli_contract -- --nocapture
cd accel-rpc && cargo test -p idxd-rust --test tokio_handle_contract --test async_validation_cli_contract --test async_verifier_contract --test async_benchmark_cli_contract --test async_benchmark_verifier_contract -- --nocapture
bash idxd-rust/scripts/verify_live_memmove.sh
bash idxd-rust/scripts/verify_async_memmove.sh
bash idxd-rust/scripts/verify_tokio_memmove_bench.sh
```

The Tokio-handle, CLI, and verifier contract tests exercise the non-hardware schemas for the public async surface and the benchmark surface. The shell verifiers are the truthful end-to-end proof commands for prepared hosts and the expected-failure proof commands for unprepared ones.
