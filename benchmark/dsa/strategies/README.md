# Benchmark Strategies

Three families of scheduling patterns for DSA benchmark operations. Each family controls
how operations are submitted and awaited; within a family, variants trade allocation cost
against stdexec overhead.

## Strategy Families

### `sliding_window/` — Sliding Window

Keeps exactly `concurrency` operations in flight at all times. As each op completes a new
one is submitted immediately. Best for measuring sustained throughput with a fixed queue depth.

| Strategy | Description | Polling modes | ~ns/op |
|---|---|---|---|
| `sliding_window` | Baseline. `scope.spawn()` with heap alloc per op via stdexec `connect`. | inline, threaded | ~35 |
| `noalloc` | Same flow but pre-allocates `concurrency` `OperationSlot` buffers; ops use placement-new to avoid per-op heap allocation. | inline, threaded | — |
| `arena` | Like noalloc but uses a free-list `SlotArena` for O(1) slot acquire/release without scanning the slot array. | inline, threaded | — |
| `direct` | Bypasses `async_scope` and `stdexec::then`. Uses a custom `DirectBenchReceiver` that polls and recycles the slot directly. | inline only | ~13 |
| `reusable` | Bypasses `stdexec::connect`/`start` entirely. Pre-allocated `DsaOperationBase` storage is reused across ops; hot path is `memset → fill_descriptor → submit`. No page-fault retry. | inline only | ~8 |

**When to use:** Throughput benchmarking with realistic queue depths. `sliding_window` is the
correct baseline for measuring stdexec overhead. `direct`/`reusable` isolate hardware
throughput by removing framework overhead.

---

### `batch/` — Batch

Submits a full batch of `concurrency` operations, then waits for the entire batch to complete
before submitting the next. Each batch is a synchronization barrier. Measures throughput in
bursts and is useful for workloads where operations are naturally grouped.

| Strategy | Description | Polling modes |
|---|---|---|
| `heap_alloc` | Baseline batch. `scope.spawn()` with heap alloc per op. | inline, threaded |
| `noalloc` | Pre-allocated `OperationSlot[]` per batch; fill phase then poll-until-all-done barrier. | inline, threaded |
| `raw` | Submits ops via the `dsa_batch` hardware batch descriptor sender — a single batch command to the hardware rather than individual submissions. | inline only |

**When to use:** Workloads that naturally batch (e.g. scatter-gather, epoch-based
processing). `raw` is the reference point for hardware batch submission overhead.

---

### `scoped_workers/` — Scoped Workers

Spawns `concurrency` coroutine workers. Each worker owns every N-th operation
(`stride = num_workers`) and `co_await`s each op sequentially before moving to the next.
Represents the natural coroutine programming model — sequential async code with implicit
concurrency from multiple workers.

| Strategy | Description | Polling modes |
|---|---|---|
| `scoped_workers` | N `exec::task<void>` coroutines, each processing a strided slice of the work. | inline, threaded |

**When to use:** Measuring coroutine overhead and testing the natural P2300 usage pattern
where user code is written as sequential coroutines rather than explicit callback chains.

---

## Polling Modes

All strategies accept a polling mode parameter:

- **inline** — The calling thread drives the DSA poll loop (`PollingRunLoop`). Lower
  latency, no thread coordination overhead.
- **threaded** — A background thread polls DSA; the main thread submits via
  `DsaScheduler`. Decouples submission from polling at the cost of cross-thread signaling.

---

## Decision Guide

| Goal | Recommended strategy |
|---|---|
| Measure real-world stdexec throughput | `sliding_window` (baseline, inline or threaded) |
| Eliminate heap alloc from the benchmark | `sliding_window/noalloc` or `batch/noalloc` |
| Measure raw hardware throughput (no framework) | `sliding_window/reusable` |
| Measure minimal stdexec overhead (connect only) | `sliding_window/direct` |
| Measure hardware batch submission | `batch/raw` |
| Test coroutine programming model | `scoped_workers` |
| Compare inline vs background-thread polling | Run any strategy in both modes |

### Performance reference (inline polling, mock DSA)

```
reusable    ~8 ns/op   — hardware limit, no framework overhead
direct     ~13 ns/op   — stdexec connect/start, no scope or then
baseline   ~35 ns/op   — full stdexec path with heap alloc
```

Actual numbers vary with message size, concurrency, and whether real or mock DSA is used.
