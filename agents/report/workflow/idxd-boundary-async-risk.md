# Code Context

## Files Retrieved
1. `idxd-rust/src/async_session.rs` (lines 22-24, 129-147, 258-260, 307-352) - async public boundary and lifecycle state uses raw `u8` constants.
2. `idxd-rust/src/async_direct.rs` (lines 27-79, 119-155, 186-215, 294-397) - direct async runtime submission/completion error path; exposes raw `DsaHwDesc` in backend trait and raw completion formatting.
3. `idxd-rust/src/async_direct/operation.rs` (lines 47-153, 156-199, 203-310) - descriptor/completion continuation path, raw pointer casts, status `u8`, `touch_fault_page`, `EnqcmdSubmission` handling.
4. `idxd-rust/src/async_direct/monitor.rs` (lines 5-50) - polling monitor consumes `CompletionSnapshot` from backend and drives operation state.
5. `idxd-rust/src/direct_memmove.rs` (lines 3-6, 130-218) - shared descriptor/completion state already wrapping `DsaHwDesc`/`DsaCompletionRecord`, but still imports raw constants/helpers.
6. `idxd-rust/src/validation.rs` (lines 4-16, 131-159, 202-218, 478-573) - current typed-ish completion snapshot/action definitions and raw DSA status interpretation.
7. `idxd-rust/src/async_direct/test_support.rs` (lines 7-26, 53-116) - test backend API uses raw `CompletionSnapshot`, `EnqcmdSubmission`, and `DsaHwDesc`.
8. `idxd-rust/tests/async_memmove_contract.rs` (lines 734-1079) - async direct tests likely to break if status/submission wrappers change.
9. `idxd-rust/tests/tokio_handle_contract.rs` (lines 1-10, 46-156) - handle composition tests construct raw completion snapshots.

## Key Code

- Raw lifecycle state in async session:

```rust
// idxd-rust/src/async_session.rs:22-24
const LIFECYCLE_RUNNING: u8 = 0;
const LIFECYCLE_SHUTDOWN_REQUESTED: u8 = 1;
const LIFECYCLE_SHUTDOWN_COMPLETE: u8 = 2;
```

Risk: if adding a typed lifecycle enum, `AtomicU8` storage still needs conversion helpers; search for `lifecycle_state()` around `idxd-rust/src/async_session.rs:307` and shutdown checks.

- Backend trait leaks raw descriptor/submission types:

```rust
// idxd-rust/src/async_direct.rs:119-127
pub trait DirectMemmoveBackend: Send + Sync + 'static {
    fn submit(&self, op_id: u64, descriptor: &DsaHwDesc) -> EnqcmdSubmission;
    fn completion_snapshot(&self, _op_id: u64, state: &DirectMemmoveState) -> Option<CompletionSnapshot> { ... }
}
```

Risk: wrappers around descriptors/submissions will affect live portal backend, scripted backend, and many tests.

- Completion/error snapshot is still raw fields:

```rust
// idxd-rust/src/validation.rs:131-143
pub struct CompletionSnapshot {
    pub status: u8,
    pub result: u8,
    pub bytes_completed: u32,
    pub fault_addr: u64,
}
```

`classify_memmove_completion` compares `snapshot.status` to raw constants at `idxd-rust/src/validation.rs:488-500` and masks unknown statuses at `idxd-rust/src/validation.rs:565-569`.

- Direct state wraps raw descriptor/completion but forwards raw constants/helpers:

```rust
// idxd-rust/src/direct_memmove.rs:170-186
reset_completion(&mut self.comp);
self.desc.fill_memmove(self.src, self.dst, self.remaining);
self.desc.set_completion(&mut self.comp);
let status = self.comp.status();
if status == DSA_COMP_NONE { None } else { ... status & DSA_COMP_STATUS_MASK }
```

Likely best first typed-wrapper insertion point: `DirectMemmoveState` methods, not `async_direct` monitor.

- Async operation uses raw completion status and helper:

```rust
// idxd-rust/src/async_direct/operation.rs:126-135
Ok(CompletionAction::Success) => {
    self.finish_success(inner, snapshot.status);
}
Ok(CompletionAction::Retry(retry)) => {
    touch_fault_page(state.completion());
    state.apply_retry(retry);
    state.reset_and_fill_descriptor();
}
```

Risk: a typed status enum changes `finish_success(... final_status: u8)` and `MemmoveCompletion::new(... final_status: u8)`.

## Architecture

Async public calls enter `AsyncDsaSession`/handle in `idxd-rust/src/async_session.rs`, then use `DirectAsyncMemmoveRuntime::memmove` in `idxd-rust/src/async_direct.rs`. The runtime builds a `PendingOperation` (`async_direct/operation.rs`) which owns source/destination buffers and a `DirectMemmoveState` (`direct_memmove.rs`). `DirectMemmoveState` owns the raw `DsaHwDesc` and `DsaCompletionRecord`, fills descriptors, reads completion status, and delegates interpretation to `classify_memmove_completion` in `validation.rs`. A Tokio monitor (`async_direct/monitor.rs`) repeatedly asks the backend for `CompletionSnapshot`s and then `PendingOperation::handle_snapshot` decides success/retry/error.

## Edit Risks / Target Files

- `idxd-rust/src/validation.rs`: highest leverage. Introduce typed completion status/result here first. Tests and serialization currently expect `u8`/hex strings (`final_status`, `completion_status`). Keep stable formatting or add accessors.
- `idxd-rust/src/direct_memmove.rs`: descriptor/completion helper boundary. Raw `DsaHwDesc`, `DsaCompletionRecord`, `reset_completion`, `touch_fault_page`, `DSA_COMP_NONE`, `DSA_COMP_STATUS_MASK` usage should be hidden here if possible.
- `idxd-rust/src/async_direct.rs`: changing `DirectMemmoveBackend::submit(&DsaHwDesc) -> EnqcmdSubmission` has broad test impact. Consider wrapping `EnqcmdSubmission` only after scripted backend migration.
- `idxd-rust/src/async_direct/operation.rs`: retry path directly calls `touch_fault_page(state.completion())`; better as `DirectMemmoveState::touch_fault_page()` to avoid leaking raw completion record.
- `idxd-rust/src/async_session.rs`: lifecycle constants are a separate small wrapper opportunity (`LifecycleState` over `AtomicU8`) but lower priority than descriptor/completion.
- Tests likely to break: `idxd-rust/tests/async_memmove_contract.rs` lines 753, 794, 898, 921, 958, 995, 1006, 1036, 1056-1066; `idxd-rust/tests/tokio_handle_contract.rs` lines 72, 79, 109, 122, 146; `idxd-rust/src/async_direct/test_support.rs` lines 38-43, 53-58, 85-116.

## Start Here

Open `idxd-rust/src/validation.rs` first. It defines `CompletionSnapshot`, `MemmoveCompletion`, and `classify_memmove_completion`, the central raw-status boundary used by both blocking and async paths.

## Pi-intercom handoff

No safe `intercom` target was provided/available for this scouting task.
