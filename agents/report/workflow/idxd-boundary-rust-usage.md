# Code Context

## Files Retrieved
1. `idxd-rust/src/direct_memmove.rs` (lines 1-219) - sync DSA memmove owns raw descriptor/completion and submits through raw portal.
2. `idxd-rust/src/async_direct.rs` (lines 119-155, 294-328) - async DSA backend trait exposes raw DSA descriptor and raw ENQCMD result.
3. `idxd-rust/src/async_direct/operation.rs` (lines 90-199) - async operation forwards raw descriptors and handles retry touching.
4. `idxd-rust/src/session.rs` (lines 96-180) - generic IDXD session owns raw `WqPortal` and calls DSA/IAX operations.
5. `idxd-rust/src/lifecycle.rs` (lines 1-65) - shared blocking lifecycle takes raw `WqPortal`.
6. `idxd-rust/src/validation.rs` (lines 131-158) - public DSA completion snapshot is built from raw completion record.
7. `idxd-rust/src/iax_crc64.rs` (lines 328-357, 366-400, 520-570) - sync IAX crc64 owns raw IAX descriptor/completion and raw portal.
8. `idxd-rust/src/bin/tokio_memmove_bench/nonbatch.rs` (lines 8-11, 104-219) - benchmark hot path directly uses raw DSA sys types/functions.
9. `idxd-sys/src/lib.rs` (lines 31-50) - raw sys re-export surface used by idxd-rust.
10. `idxd-sys/src/portal.rs` (lines 8-18, 33-67, 199-242) - raw portal and ENQCMD submission primitives.

## Key Code

Raw `idxd-sys` imports in `idxd-rust/src` are concentrated in 13 files:

- `idxd-rust/src/direct_memmove.rs:3-6`: `DSA_COMP_NONE`, `DSA_COMP_STATUS_MASK`, `DsaCompletionRecord`, `DsaHwDesc`, `WqPortal`, `poll_completion`, `reset_completion`, `touch_fault_page`; tests also import `DSA_COMP_PAGE_FAULT_NOBOF`, `DSA_COMP_SUCCESS` at `direct_memmove.rs:251`.
- `idxd-rust/src/async_direct.rs:16`: `DsaHwDesc`, `EnqcmdSubmission`, `WqPortal`.
- `idxd-rust/src/async_direct/operation.rs:4`: `DsaHwDesc`, `EnqcmdSubmission`, `touch_fault_page`.
- `idxd-rust/src/async_direct/test_support.rs:8`: `DsaHwDesc`, `EnqcmdSubmission`.
- `idxd-rust/src/validation.rs:4-6`: DSA completion constants and `DsaCompletionRecord` for `CompletionSnapshot::from_record` (`validation.rs:151-158`).
- `idxd-rust/src/session.rs:5`: `WqPortal`; session stores it at `session.rs:103-106`, opens at `session.rs:115-123`, submits DSA/IAX at `session.rs:142-180`.
- `idxd-rust/src/lifecycle.rs:1`: `WqPortal`; blocking lifecycle passes it to operations (`lifecycle.rs:30`, `lifecycle.rs:41-57`).
- `idxd-rust/src/iax_crc64.rs:3-6`: `IAX_COMP_PAGE_FAULT_IR`, `IAX_COMP_SUCCESS`, `IaxCompletionRecord`, `IaxHwDesc`, `WqPortal`, `poll_iax_completion`, `reset_iax_completion`, `touch_iax_fault_page`; tests import `IAX_STATUS_ANALYTICS_ERROR` at `iax_crc64.rs:575`.
- `idxd-rust/src/bin/idxd_representative_bench.rs:12` and `idxd-rust/src/bin/live_idxd_op.rs:12`: `crc64_t10dif_field` for expected CRC calculation.
- `idxd-rust/src/bin/tokio_memmove_bench/nonbatch.rs:8-11`: raw DSA descriptor/completion, status constants, portal, reset/touch helpers; uses `submit_enqcmd_desc64` at `nonbatch.rs:211-219`.
- `idxd-rust/src/bin/tokio_memmove_bench/software.rs:8`: `DSA_COMP_SUCCESS`, `DsaHwDesc`, `EnqcmdSubmission` for fake backend completion injection.

Critical raw-type dependencies:

```rust
// idxd-rust/src/direct_memmove.rs:22-24
pub struct DirectMemmoveState {
    desc: DsaHwDesc,
    comp: DsaCompletionRecord,
```

```rust
// idxd-rust/src/async_direct.rs:119-120
pub trait DirectMemmoveBackend: Send + Sync + 'static {
    fn submit(&self, op_id: u64, descriptor: &DsaHwDesc) -> EnqcmdSubmission;
```

```rust
// idxd-rust/src/iax_crc64.rs:367-369
pub(crate) struct IaxCrc64State<'a> {
    desc: IaxHwDesc,
    comp: IaxCompletionRecord,
```

```rust
// idxd-sys/src/portal.rs:13-18
pub enum EnqcmdSubmission { Accepted, Rejected }
```

## Architecture

`idxd-rust` currently wraps operation semantics but not the raw IDXD boundary. Sync DSA and IAX paths both use `IdxdSession`/`WqPortal` ownership plus a shared blocking lifecycle. The operation state owns descriptor and completion record storage, fills descriptors, submits through `WqPortal`, polls completion records, classifies status, touches fault pages, and retries.

Async DSA (`async_direct`) reuses `DirectMemmoveState` for descriptor/completion ownership but exposes raw `DsaHwDesc` and `EnqcmdSubmission` in `DirectMemmoveBackend`, so tests and software backends also import raw sys types. The monitor checks completion snapshots through `DirectMemmoveState`, then retries using raw fault-touch helper.

The benchmark binary `tokio_memmove_bench/nonbatch.rs` bypasses library operation state for hot-path throughput and directly duplicates raw descriptor/completion lifecycle logic. This is the largest out-of-library raw usage and is likely intentionally performance-sensitive.

## Start Here

Start at `idxd-rust/src/direct_memmove.rs`. It already centralizes the DSA descriptor/completion lifecycle used by both sync and async memmove. A minimal wrapper surface should grow from `DirectMemmoveState` and then adjust async/backend signatures.

## Proposed minimal wrapper module surface

Add a small crate-private module, e.g. `idxd-rust/src/sys_boundary.rs` or `idxd-rust/src/raw.rs`, not a broad redesign. Surface should wrap only raw sys types already used:

1. `Portal` newtype over `idxd_sys::WqPortal`
   - `open(&Path) -> io::Result<Portal>`
   - `is_dedicated(&self) -> bool`
   - unsafe `submit_dsa(&self, &DsaDescriptor)` for blocking/spinning mode
   - unsafe `submit_dsa_once(&self, &DsaDescriptor) -> Submission`
   - unsafe `submit_iax(&self, &IaxDescriptor)`
   - maybe `submit_dsa_desc64_once(&self, *const u8) -> Submission` only if keeping `nonbatch.rs` direct fast path behind the boundary is required.

2. `Submission` enum mirroring `EnqcmdSubmission::{Accepted, Rejected}`
   - used by `DirectMemmoveBackend`, test support, software backend.

3. `DsaDescriptor` and `DsaCompletion` newtypes around `DsaHwDesc` and `DsaCompletionRecord`
   - `Default`
   - DSA methods actually used: `fill_memmove`, `fill_memmove_to_memory` (needed by `nonbatch.rs`), `set_completion`, `as_desc64_ptr` if preserving benchmark fast path, completion accessors `status/result/bytes_completed/fault_addr`.
   - helpers wrapping raw functions: `reset_completion`, `reset_completion_status`, `poll_completion`, `touch_fault_page`.

4. `IaxDescriptor` and `IaxCompletion` newtypes around `IaxHwDesc` and `IaxCompletionRecord`
   - `Default`
   - IAX methods used: `fill_crc64`, `set_completion`, `poll_iax_completion`, `reset_iax_completion`, `touch_iax_fault_page`, completion accessors `error_code/invalid_flags/fault_addr/crc64`.

5. Status constants re-exported or namespaced through wrapper module
   - DSA: `DSA_COMP_NONE`, `DSA_COMP_STATUS_MASK`, `DSA_COMP_SUCCESS`, `DSA_COMP_PAGE_FAULT_NOBOF`.
   - IAX: `IAX_COMP_SUCCESS`, `IAX_COMP_PAGE_FAULT_IR`, test-only `IAX_STATUS_ANALYTICS_ERROR`.
   - CRC helper `crc64_t10dif_field` can either be re-exported as a pure helper or left as direct `idxd-sys` usage in binaries; if goal is no raw sys imports, wrap/re-export it too.

Risks/constraints:
- Keep wrapper newtypes crate-private first to avoid public API churn.
- `DirectMemmoveBackend` is public and currently mentions `DsaHwDesc`; changing it to `DsaDescriptor` is a public API break unless this crate treats it as internal/unstable.
- `nonbatch.rs` uses raw descriptor pointer submission for speed (`submit_enqcmd_desc64`); wrapping it must preserve inlining and avoid extra allocation/locking.
- Some tests inspect raw descriptor fields (`IaxHwDesc::src1_addr`, completion address). Wrapper may need test-only accessors or `as_raw_for_test`.
