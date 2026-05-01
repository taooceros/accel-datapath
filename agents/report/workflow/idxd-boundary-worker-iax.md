# idxd boundary worker: IAX/CRC wrapper slice

## Summary

Implemented the IAX/CRC slice in `idxd-rust`:

- Added `idxd-rust` owned wrappers over `idxd_sys::idxd_uapi::iax_hw_desc` and `idxd_sys::idxd_uapi::iax_completion_record` in `idxd-rust/src/iax_crc64.rs`.
- Added `#[repr(u8)]` IAX opcode/status enums for the CRC64 path and completion classification.
- Moved IAX completion polling, reset, fault-page touch, and T10DIF `crc16_t10dif` / `crc64_t10dif_field` helpers into `idxd-rust`.
- Updated IAX CRC64 operation code to use the local wrapper types/helpers instead of `idxd-sys` helper exports.
- Updated `live_idxd_op` and `idxd_representative_bench` to import `crc64_t10dif_field` from `idxd_rust`.
- Re-exported IAX wrapper and CRC helper symbols from `idxd-rust/src/lib.rs`.

## Changed files

- `idxd-rust/src/iax_crc64.rs`
- `idxd-rust/src/lib.rs`
- `idxd-rust/src/bin/live_idxd_op.rs`
- `idxd-rust/src/bin/idxd_representative_bench.rs`
- `progress.md`
- `agents/report/workflow/idxd-boundary-worker-iax.md`

## Validation

- Ran:

```bash
rustfmt --edition 2024 idxd-rust/src/iax_crc64.rs idxd-rust/src/lib.rs idxd-rust/src/bin/live_idxd_op.rs idxd-rust/src/bin/idxd_representative_bench.rs
```

- Ran:

```bash
cargo check --manifest-path ./Cargo.toml -p idxd-rust --lib
```

Result: fails outside this assigned IAX slice in `idxd-rust/src/async_direct.rs` because `DsaPortalExt` is not imported for `submit_enqcmd_once`. No IAX-specific errors were reported in the final check.

## Notes

- Requested input files `context.md` and `plan.md` were missing from the repo root, so implementation used the delegated task text and live source.
- IAX submission now calls raw 64-byte portal submission methods through the local wrapper descriptor pointer and uses the crate portal WQ-mode detection helper.
