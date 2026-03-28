# dsa_stdexec -- stdexec (P2300) Integration Layer

This directory bridges the low-level DSA hardware interface (`src/dsa/`) with the
C++ sender/receiver model defined by P2300 (stdexec). It provides senders for each
DSA operation, a polling run loop, and the supporting infrastructure to compose DSA
work into stdexec pipelines.

## Key Files

| File | Purpose |
|------|---------|
| `operation_base.hpp` | Type-erased operation via `pro::proxy<OperationFacade>`. Stores an intrusive `next` pointer for queue linking. Proxy dispatches `notify()` and `get_descriptor()`. |
| `run_loop.hpp` | `PollingRunLoop` -- custom run loop that interleaves stdexec task execution with DSA completion polling. Primary execution model for maximizing message rate. |
| `sync_wait.hpp` | `wait_start(sender, loop)` -- drives the polling loop until the given sender completes. Used by inline-polling benchmark strategies. |
| `scheduler.hpp` | `DsaScheduler` for threaded polling mode (not the primary optimization target). |
| `submit.hpp` | Submit helpers for fire-and-forget sender launching. |
| `batch.hpp` | Hardware batch descriptor sender (`dsa_batch`). Submits a group of descriptors as a single hardware batch. |
| `descriptor_fill.hpp` | Helpers for populating individual descriptor fields. |
| `dsa_facade.hpp` | `DsaProxy` -- type-erased facade over `DsaEngine` variants, used by benchmark code. |
| `dsa_sink.hpp` | Sink adapter for consuming sender values. |
| `error.hpp` | Error types for DSA completion status codes. |
| `operations/` | Per-operation sender implementations. See [operations/README.md](operations/README.md). |

## Design Decisions

**Type erasure via `pro::proxy`**, not virtual dispatch. `pro::proxy<OperationFacade>`
enables heterogeneous operation types in intrusive linked lists without vtable overhead.
This keeps the hot poll path free of indirect calls while still allowing mixed operation
types in the same queue.

**`PollingRunLoop` as primary execution model.** The calling thread drives both
submission and completion in a tight loop with no cross-thread coordination. This
eliminates lock contention and context-switch overhead, which is critical for
maximizing message rate on small transfers.

**Static dispatch for completion checking.** `HwContext::check_completion()` is
resolved at compile time, avoiding virtual calls in the poll loop.

**Over-alignment at runtime.** `DsaOperationBase` computes 64-byte descriptor
alignment and 32-byte completion record alignment at runtime (via over-allocation)
instead of using `alignas()`. This ensures correct alignment even when the operation
is allocated inside a coroutine frame, where `alignas` is not guaranteed to be honored.

## See Also

- [operations/README.md](operations/README.md) -- per-operation sender details
- [../../AGENTS.md](../../AGENTS.md) -- project-wide conventions and architecture overview
