# WQ Depth Backpressure for Dedicated Work Queues

**Status**: cancelled on 2026-04-15

Historical note:
Relationship: Preserved as a historical planning record; future work should start from a fresh plan instead of reviving this stale in-progress file.

## Problem

Dedicated DSA work queues use `_movdir64b` (posted write) for descriptor submission.
When the WQ is full, `_movdir64b` **silently drops** the descriptor — no error, no retry.
The completion record for the dropped descriptor is never written (stays 0), causing
the benchmark to hang forever in the drain loop.

Observed: deterministic hang at `c=1024 sz=2048` with `sliding_window_reusable` strategy.
The submission rate exceeds the WQ drain rate (WQ size=128, concurrency=1024).

Shared WQs (`_enqcmd`) don't have this problem — they return non-zero when full.

## Solution

Extend `DescriptorSubmitter` concept with a unified backpressure interface.
`DsaEngine::submit()` gates on WQ depth using direct calls — zero overhead for
submitters that don't need it (batch submitters return capacity=0, optimizer
eliminates the dead check).

### New concept members

```
wq_capacity()       → WQ depth (0 = no gating)
inflight()          → current in-flight count
notify_complete(n)  → decrement inflight by n completed ops
```

### DirectSubmitter

Tracks inflight count. `submit_descriptor()` increments after `_movdir64b`.
Only active for dedicated WQ mode with known depth.

### Batch submitters

Return 0/no-op — they self-throttle via ring capacity (16 << 128 WQ slots).
Inner `DirectSubmitter` gets `nullptr` for `wq` to disable depth tracking.

### DsaEngine

`submit()`: `while (capacity > 0 && inflight >= capacity) poll();`
`poll()`: `notify_complete(task_queue_.poll())` to feed completions back.

## Alternatives considered

1. **Tracking in DirectSubmitter with std::function callback** — rejected,
   adds heap alloc + indirection on the hottest path (~8 ns/op target).
2. **if constexpr on Submitter type in DsaEngine** — rejected, fragile,
   doesn't compose. Unified interface is cleaner.
3. **Cap concurrency to WQ size** — too restrictive, changes benchmark semantics.

## Files

- `src/dsa/descriptor_submitter.hpp` — concept + all submitters
- `src/dsa/dsa.ipp` — DsaEngine::submit() and poll()
