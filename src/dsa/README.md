# Low-Level DSA Hardware Interface

This module provides the hardware abstraction layer for Intel Data Streaming
Accelerator (DSA). It handles device discovery, descriptor submission,
completion polling, and work queue management. Everything above this layer
(stdexec senders, benchmark strategies) builds on these primitives.

## Key Files

| File | Purpose |
|------|---------|
| `dsa.hpp` / `dsa.ipp` | `DsaEngine<Submitter, QueueTemplate>` -- device discovery via libaccel-config, `submit()`, `poll()`, WQ backpressure |
| `dsa_*_instantiate.cpp` | Explicit template instantiations for each submitter/queue combination to avoid redundant compilation |
| `descriptor_submitter.hpp` | `DescriptorSubmitter` concept. `DirectSubmitter` (immediate `_movdir64b`/`_enqcmd`), batch submitters (`BatchAdaptiveSubmitter`, `MirroredRingBatchSubmitter`, etc.) |
| `task_queue.hpp` | `TaskQueue` concept. Implementations: `LockedTaskQueue<Lock, HwCtx>`, `RingBufferTaskQueue`, `LockFreeTaskQueue` |
| `dsa_operation_base.hpp` | Base class for all hardware operations with runtime over-alignment for 64B descriptors and 32B completion records |
| `mirrored_ring.hpp` | RAII double-mapped ring buffer via `memfd_create` + two `MAP_FIXED` mappings; eliminates wrap-around handling |
| `mock_dsa.hpp` | Mock implementation for testing without hardware |
| `enum_format.hpp` | Format helpers for DSA enums |

## Design Decisions

**Concept-based TaskQueue.** Queue types satisfy the `TaskQueue` concept rather
than inheriting from a virtual base. This enables static dispatch in the hot
poll loop -- `HwContext::check_completion()` avoids virtual calls entirely.

**Concept-based DescriptorSubmitter.** `DirectSubmitter` performs immediate
`_movdir64b`/`_enqcmd` and tracks inflight count plus WQ depth for dedicated
mode backpressure. Batch submitters stage descriptors into a ring and submit
them as hardware batch descriptors; they self-throttle via ring capacity and
delegate `wq_capacity()`/`inflight()` to their inner `DirectSubmitter`.

**Runtime over-alignment (not `alignas`).** `DsaOperationBase` computes
alignment at runtime by over-allocating buffers for 64-byte aligned descriptors
and 32-byte aligned completion records. This is required for coroutine
compatibility -- `alignas` on coroutine frame members is not reliably honored.

**WQ backpressure for dedicated queues.** `DsaEngine::submit()` spins on
`poll()` when inflight descriptors reach WQ depth. Dedicated work queues accept
`_movdir64b` unconditionally; without backpressure, submissions silently drop
when the WQ is full.

**MirroredRing.** The double-mapped ring buffer maps the same physical pages
at two consecutive virtual addresses via `memfd_create`. Code can write
linearly across the boundary without wrap-around logic, which simplifies
batch submitter implementations.

## Type Aliases

| Alias | Queue Type | Use Case |
|-------|-----------|----------|
| `DsaSingleThread` | `SingleThreadTaskQueue` | No locks, best for single-thread inline polling |
| `DsaIndexed` | `IndexedTaskQueue` | Per-slot, no head contention, inline only |
| `Dsa` | `MutexTaskQueue` | General purpose, threaded polling |
| `DsaTasSpinlock` | TAS spinlock queue | Threaded polling comparison |
| `DsaSpinlock` | TTAS spinlock queue | Threaded polling comparison |
| `DsaBackoffSpinlock` | Backoff spinlock queue | Threaded polling comparison |
| `DsaLockFree` | `LockFreeTaskQueue` | Lock-free, threaded polling |

For inline polling (the primary optimization target), prefer `DsaSingleThread`
or `DsaIndexed`. The locked variants exist for threaded polling comparisons.

## Extending

**Adding a new queue type.** Implement the `TaskQueue` concept defined in
`task_queue.hpp`. The key methods are enqueue, dequeue, and a
`check_completion()` path compatible with `HwContext`. Then add an explicit
instantiation file (`dsa_<name>_instantiate.cpp`) following the existing
pattern to avoid recompiling the full template in every translation unit.

**Adding a new submitter.** Implement the `DescriptorSubmitter` concept in
`descriptor_submitter.hpp`. The submitter must provide `submit()`,
`notify_complete()`, `inflight()`, and `wq_capacity()`. For batch submitters,
delegate capacity tracking to an inner `DirectSubmitter`. Add corresponding
explicit instantiation files.

## See Also

See [CLAUDE.md](../../CLAUDE.md) for the full project architecture, build
instructions, and benchmark strategy taxonomy.
