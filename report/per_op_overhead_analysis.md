# Per-Operation Overhead Analysis and Optimization Roadmap

> **Accuracy note (2026-02-22)**: The per-phase cost breakdown in this report consists
> of **analytical estimates, not individually instrumented measurements**. The total
> end-to-end throughput (25--27 Mpps mock, 20--22 Mpps real) is measured, but the
> attribution of nanoseconds to individual phases (e.g., "9 ns for connect", "5 ns for
> poll") was produced by reasoning about the code structure, not by instrumenting each
> phase with cycle counters. Subsequent optimization experiments showed these estimates
> significantly over-predicted savings (projected 11 ns combined, actual ~2--3 ns).
> See `progress_post_alignment_debug.md` for the measured layer-removal results that
> provide more reliable overhead attribution.

## Executive Summary

Mock DSA benchmarks establish the pure software ceiling at **25-27 Mpps** (~37-40 ns/op).
Real DSA peaks at **20-22 Mpps** (~45-50 ns/op). This report decomposes the per-operation
overhead into 8 phases with estimated costs, identifies the top 4 optimizations, and projects
their combined impact at **35-40 Mpps** (mock) and **28-32 Mpps** (real DSA).

> **Note**: The projected targets above were not achieved. Actual post-optimization
> throughput was ~27 Mpps mock. See `optimization_results.md` for details.

## Methodology

- **Mock DSA**: `MockDsaBase` with instant completion (`DSA_COMP_SUCCESS` on submit),
  isolating software overhead from hardware latency.
- **Configuration**: NoLock queue, sliding_window_noalloc, concurrency=2048, msg_size=8,
  total_bytes=32MB, 3 iterations.
- **Measurement**: Wall-clock time for `num_ops` operations → per-op latency = time / num_ops.

## Per-Operation Cost Breakdown

Total: ~37 ns/op (mock, best case) → 27 Mpps

> **All per-phase costs below are estimates, not measurements.** They were produced by
> reasoning about the code, not by instrumenting individual phases.

### Phase 1: stdexec connect + placement new (9 ns, 24%)

```
strategy_noalloc.cpp:27
  slot->start_op(scope.nest(make_sender(offset) | stdexec::then(record)));
helpers.hpp:221-222
  auto *op = new (storage)
      Op(stdexec::connect(std::forward<Sender>(sender), SlotReceiver{&ready}));
```

`stdexec::connect()` constructs the full operation state object — for data_move with
inline polling, this is a `NestOp<ThenOp<DataMoveOp<DsaProxy, SlotReceiver>>>` weighing
~448 bytes. The connect call evaluates template recursion through scope.nest() → then() →
the DSA sender, constructing each layer in-place.

**Irreducible**: This is fundamental stdexec machinery. The only way to reduce it is to
avoid constructing sender chains per operation (e.g., batch raw descriptors).

### Phase 2: Proxy allocation on every start (4 ns, 11%)

```
operation_base_mixin.hpp:139-145
  struct Wrapper {
    Self *op;
    void notify() { op->notify(); }
    dsa_hw_desc *get_descriptor() { return op->desc_ptr(); }
  };
  self.proxy = pro::make_proxy<OperationFacade>(Wrapper{&self});
```

`pro::make_proxy` allocates a small-buffer-optimized proxy object on every `start()`.
The proxy wraps two function pointers (notify + get_descriptor) behind type erasure.

**Optimization**: Replace proxy with direct function pointers in OperationBase.
The concrete type is known at start() via CRTP / deducing this. Store `notify_fn` and
`get_descriptor_fn` as raw function pointers — zero allocation, zero indirection overhead.

**Projected savings**: 4 ns/op → 33 ns/op → ~30 Mpps

### Phase 3: O(N) linked-list poll traversal (5 ns amortized, 14%)

```
task_queue.hpp:152-168  (LockedTaskQueue::poll)
  while (curr != nullptr) {
    if (hw_ctx_.check_completion(curr)) { ... remove ... }
    else { pprev = &curr->next; curr = curr->next; }
  }
```

With concurrency=2048 in-flight, poll traverses 2048 `OperationBase*` pointers.
Each is a cache miss (operations scattered in OperationSlot storage at 448-byte stride).
`check_completion()` for mock always returns true, so every op is "completed" — but
the pointer chasing still costs ~2.5 ns per node.

With real DSA, most ops are NOT completed on any given poll, so the traversal is pure
waste for ~95% of nodes.

**Optimization**: Replace linked list with indexed array + active list. Poll iterates
a compact `uint32_t` array of active slot indices — cache-friendly sequential access
instead of random pointer chasing.

**Projected savings**: 3-5 ns/op → 28-30 ns/op → 33-36 Mpps

### Phase 4: O(N) slot scan for ready slots (3 ns amortized, 8%)

```
strategy_noalloc.cpp:21-23
  for (auto &slot : slots) {
    if (!slot->ready.load(std::memory_order_acquire)) continue;
```

At c=2048, touches 2048 cache lines (OperationSlot is 448 bytes, so ~7 cache lines each).
At c=4096, this dominates — the slot array exceeds L2 cache.

**Optimization**: Use `SlotArena` (already implemented in helpers.hpp:258) with its
intrusive free-list. `acquire()` pops a ready slot in O(1); `release()` pushes it back
on completion. No scanning required.

**Projected savings**: 2-3 ns/op at c=2048, 5+ ns/op at c=4096

### Phase 5: stdexec set_value propagation (5 ns, 14%)

```
operation_base_mixin.hpp:176-177
  stdexec::set_value(std::move(self.r_));
```

The receiver chain is `SlotReceiver` wrapped by `ThenReceiver` wrapped by `NestReceiver`.
set_value propagates through each layer, eventually reaching `SlotReceiver::set_value()`
which does a single `atomic_store(true)`.

**Irreducible**: Part of the stdexec receiver contract. The chain is already minimal
(3 layers for scope.nest + then + slot_receiver).

### Phase 6: proxy->notify() indirect dispatch (3 ns, 8%)

```
task_queue.hpp:178
  op->proxy->notify();
```

Virtual-like dispatch through `pro::proxy`. With Opt 1 (function pointer replacement),
this becomes a direct function pointer call — same cost as a regular indirect call but
without proxy indirection.

**Addressed by Opt 1**: Reducing from proxy dispatch to direct `fn(this)` saves ~1-2 ns.

### Phase 7: Sender chain overhead (6 ns, 16%)

The sender chain `scope.nest(make_sender(offset) | stdexec::then(record))` creates
temporary sender objects during construction. The `then()` adapter wraps the DSA sender
and the completion record in a new sender type, and `scope.nest()` adds lifetime tracking.

**Irreducible**: Fundamental to the stdexec composition model. Could be reduced by
using a raw descriptor path (batch_raw strategy) but that bypasses stdexec entirely.

### Phase 8: CompletionRecord + in_flight atomic (2 ns, 5%)

```
strategy_common.hpp:74-79
  void operator()(auto &&...) const {
    if (in_flight) in_flight->fetch_sub(1, std::memory_order_release);
  }
```

Atomic fetch_sub on shared counter. With latency sampling disabled, the chrono calls
are eliminated. The atomic is needed for the outer loop termination condition.

**Low priority**: 2 ns is small. Could be eliminated entirely with a different
termination strategy (count completions instead of tracking in-flight).

## Combined Projection

> **These projections were not achieved.** Actual combined savings were ~2--3 ns/op
> (27 Mpps), not the projected 11 ns/op (38 Mpps). See `optimization_results.md`.

| Optimization | Projected savings | Cumulative (mock) | Projected Mpps |
|-------------|---------|-------------------|------|
| Baseline | — | 37 ns/op | 27 |
| 1: Proxy → fn pointers | 4 ns | 33 ns/op | 30 |
| 3: SlotArena free-list | 3 ns | 30 ns/op | 33 |
| 2: Indexed array queue | 3-5 ns | 26 ns/op | 38 |
| **Total (projected)** | **~11 ns** | **~26 ns/op** | **~38** |
| **Actual (measured)** | **~2--3 ns** | **~35 ns/op** | **~27** |

For real DSA, the hardware adds ~10-15 ns/op, so the projected ceiling is:
- Current: 45-50 ns/op → 20-22 Mpps
- Optimized: 36-41 ns/op → 24-28 Mpps
- With conditional flush (Opt 4) enabling real batching: 32-36 ns/op → 28-31 Mpps

## Irreducible Floor

> **These are estimates, not measurements.** The individual phase costs were not
> instrumented. The measured total stdexec overhead (baseline − Level 2) is ~21 ns/op.

The estimated irreducible software overhead is:
- stdexec connect/start: ~9 ns (construct sender chain)
- stdexec set_value: ~5 ns (propagate completion)
- Sender chain construction: ~6 ns (scope.nest + then)
- Misc (atomic, memset desc/comp): ~2 ns
- **Total estimated irreducible**: ~22 ns/op → theoretical max ~45 Mpps

The measured decomposition via layer-removal experiments gives:
- scope.nest + then: ~14 ns (baseline − Level 1, measured)
- connect + start: ~7 ns (Level 1 − Level 2, measured)
- **Total measured stdexec overhead**: ~21 ns/op

Getting below ~17 ns/op (Level 2 at c=2048) requires reducing cache-miss cost
(Level 2 at c=32 reaches 11.9 ns) or bypassing the remaining per-op work entirely.

## Recommendation

Implement optimizations 1, 2, and 3 in parallel. Each is independent and can be
validated with mock benchmark. Optimization 4 (conditional flush) should be attempted
after 2 is validated, as the indexed queue changes how poll interacts with the submitter.

Priority order by risk-adjusted impact:
1. **Opt 1** (proxy → fn pointers): Low risk, 4 ns savings, touches 2 files
2. **Opt 3** (SlotArena): Low risk, 3 ns savings, touches 1 file, fixes regression
3. **Opt 2** (indexed queue): Medium risk, 3-5 ns savings, new data structure
4. **Opt 4** (conditional flush): Medium risk, real-DSA-only, depends on validation
