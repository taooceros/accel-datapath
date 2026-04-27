# tonic-profile

`tonic-profile` is the repo-local Tonic profiling harness. It owns the gRPC codec/selftest evidence paths and, for M003/S05, also owns the downstream Tokio consumer proof for the public `idxd-rust` async owner/handle API.

## Downstream async-handle proof seam

The S05 proof seam is the standalone binary:

```bash
cargo run -p tonic-profile --bin downstream_async_handle -- \
  --device /dev/dsa/wq0.0 \
  --bytes 64 \
  --format json \
  --artifact /tmp/downstream_async_handle.json
```

For the operational hardware-aware verifier, run from the repo root or the `accel-rpc` workspace root:

```bash
bash accel-rpc/tonic-profile/scripts/verify_downstream_async_handle.sh
```

The verifier builds `tonic-profile`'s `downstream_async_handle` binary, runs it through the repo `launch`/`dsa_launcher` capability path, and validates the JSON artifact. A successful live run ends with `verdict=pass`; an unprepared host or unavailable work queue ends with `verdict=expected_failure` only when the failure is explicitly classified.

The artifact contract includes these downstream-specific fields:

- `proof_seam=downstream_async_handle`
- `consumer_package=tonic-profile`
- `binding_package=idxd-rust`
- `composition=tokio_join`
- `operation_count=2`
- `phase`
- `error_kind`
- `lifecycle_failure_kind`
- `worker_failure_kind`
- `validation_phase`
- `validation_error_kind`

This command is intentionally not a wrapper around `idxd-rust`'s crate-local `await_memmove` proof binary. It proves that a package outside the canonical binding crate can clone public async handles, compose them with ordinary Tokio code, await real memmove operations, and preserve typed lifecycle/worker/validation diagnostics.

## Codec boundary warning

`src/custom_codec.rs` remains a synchronous codec seam. Do not force `AsyncDsaHandle`, `block_on`, or spawned async behavior into the codec to satisfy S05. Async integration is proven by the downstream binary above; codec-lane DSA behavior is covered separately by the existing S03 verifier:

```bash
bash accel-rpc/tonic-profile/scripts/verify_s03_idxd_path.sh
```

## Quick checks

```bash
cargo test -p tonic-profile --test downstream_async_handle_contract -- --nocapture
bash accel-rpc/tonic-profile/scripts/verify_downstream_async_handle.sh
```
