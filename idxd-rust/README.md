# idxd-rust

`idxd-rust` is the Rust owner for the IDXD binding stack. It wraps the raw
`idxd-sys` bindgen/UAPI and work-queue portal boundary with small synchronous
Rust types and operation APIs.

## Boundary

- `idxd-sys` owns generated `linux/idxd.h` bindings and raw MMIO portal
  submission primitives only.
- `idxd-rust/src/raw/dsa_memmove.rs` owns the thin DSA descriptor/completion
  wrapper needed by memmove.
- `idxd-rust/src/raw/iax_crc64.rs` owns the thin IAX descriptor/completion
  wrapper needed by crc64.
- `idxd-rust/src/raw/work_queue.rs` owns work-queue mode detection and the small
  safe-layer portal wrapper.
- Operation modules (`direct_memmove.rs`, `iax_crc64.rs`, `session.rs`) own
  validation, retry policy, lifecycle, and public API behavior.

## Public synchronous APIs

- `DsaSession::memmove(dst, src)` for compatibility callers.
- `IdxdSession::<Dsa>::memmove(dst, src)` for the generic DSA session path.
- `IdxdSession::<Iax>::crc64(src)` / `IdxdSession::<Iaa>::crc64(src)` for the
  representative IAX/IAA crc64 path.

Only synchronous session and operation surfaces are part of this crate.

## Verification

Host-free checks:

```bash
cargo test --manifest-path ./Cargo.toml -p idxd-rust --lib --tests
cargo test --manifest-path ./Cargo.toml -p idxd-sys --lib --tests
```

Hardware checks must use the launcher-backed verifier scripts for the relevant
synchronous operation path.

## Representative proof contract

`live_idxd_op` is the live representative operation proof binary. Use
`verify_idxd_representative_ops.sh` with `IDXD_RUST_VERIFY_DSA_DEVICE`,
`IDXD_RUST_VERIFY_IAX_DEVICE`, and optional `IDXD_RUST_VERIFY_DSA_SHARED_DEVICE`
to exercise `dsa-memmove` and `iax-crc64` paths. Verifier final lines report
`verdict=pass` or `verdict=expected_failure` and include `artifact_paths`,
`stdout_paths`, and `stderr_paths`. Diagnostics remain no-payload. This proof is
not a benchmark.
