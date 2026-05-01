# IDXD boundary cleanup worker: async/benchmark slice

## Scope

Edited only the assigned async direct, Tokio benchmark helper, and assigned async test files. `idxd-sys` was not edited by this worker.

`context.md` and `plan.md` requested by the task were not present at the repo root when this worker attempted to read them.

## Changes

- Replaced direct `idxd_sys` imports in async direct runtime files with `idxd-rust` wrapper exports:
  - `DsaHwDesc`
  - `EnqcmdSubmission`
  - `WqPortal`
  - `DsaPortalExt`
  - `touch_fault_page`
- Updated `tokio_memmove_bench/nonbatch.rs` to use `idxd-rust` DSA descriptor/completion/status/submission/portal exports instead of importing from `idxd_sys`.
- Updated `tokio_memmove_bench/software.rs` to publish success completion through `DsaCompletionStatus::Success.as_u8()` and wrapper descriptor/submission types.
- Updated assigned async tests to use `DsaCompletionStatus` and `EnqcmdSubmission` from `idxd-rust` instead of raw `idxd_sys` constants/types.
- Kept the benchmark hot path using the new `DsaPortalExt::submit_enqcmd_once(&DsaHwDesc)` wrapper submission API.

## Changed files

- `idxd-rust/src/async_direct.rs`
- `idxd-rust/src/async_direct/operation.rs`
- `idxd-rust/src/async_direct/test_support.rs`
- `idxd-rust/src/bin/tokio_memmove_bench/nonbatch.rs`
- `idxd-rust/src/bin/tokio_memmove_bench/software.rs`
- `idxd-rust/tests/async_memmove_contract.rs`
- `idxd-rust/tests/tokio_handle_contract.rs`

## Verification

Passed:

```bash
rustfmt --edition 2024 --check \
  idxd-rust/src/async_direct.rs \
  idxd-rust/src/async_direct/operation.rs \
  idxd-rust/src/async_direct/test_support.rs \
  idxd-rust/src/bin/tokio_memmove_bench/nonbatch.rs \
  idxd-rust/src/bin/tokio_memmove_bench/software.rs \
  idxd-rust/tests/async_memmove_contract.rs \
  idxd-rust/tests/tokio_handle_contract.rs

cargo check --manifest-path ./Cargo.toml -p idxd-rust --lib --bins

cargo check --manifest-path ./Cargo.toml -p idxd-rust \
  --test async_memmove_contract \
  --test tokio_handle_contract
```

The cargo checks pass with one warning from current tree code outside this slice:

```text
warning: variant `AnalyticsError` is never constructed
```

Known outside-slice failure:

```bash
cargo check --manifest-path ./Cargo.toml -p idxd-rust --tests
```

still fails in `idxd-rust/tests/memmove_contract.rs` because that unassigned test imports removed `idxd_sys::{DsaCompletionRecord, DsaHwDesc}`. The compiler suggests importing `idxd_rust::{DsaCompletionRecord, DsaHwDesc}` instead.

## Notes / risks

- The live wrapper surface uses root exports (`DsaHwDesc`, `DsaCompletionRecord`, `DsaCompletionStatus`, `EnqcmdSubmission`, `WqPortal`, `DsaPortalExt`) rather than the initially expected `sys_boundary` module. This worker adapted to the available wrapper names.
- `DsaCompletionStatus::mask` is currently not public to benchmark binaries, so `nonbatch.rs` has a small local `masked_dsa_status(status) -> status & 0x7f` helper. If the wrapper API exposes a public mask helper later, replace this local helper.
