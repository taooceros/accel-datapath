# Plan: Per-Operation Performance Improvements

**Status**: cancelled on 2026-04-15

Historical note:
Relationship: Preserved as a historical planning record; future work should start from a fresh plan instead of reviving this stale in-progress file.

## Goal

Reduce per-operation software overhead from ~37 ns/op (mock baseline) toward ~25 ns/op,
targeting 35-40 Mpps on mock and 28-32 Mpps on real DSA hardware.

## Current Per-Op Budget (Mock DSA, NoLock, sliding_window_noalloc)

| Phase | Cost | Source |
|-------|------|--------|
| stdexec connect + placement new + start | ~9 ns | helpers.hpp:210-224 |
| `pro::make_proxy<OperationFacade>()` on every start | ~4 ns | operation_base_mixin.hpp:145 |
| `proxy->notify()` indirect dispatch | ~3 ns | task_queue.hpp:178 |
| O(N) linked-list poll traversal | ~5 ns (amortized) | task_queue.hpp:152-168 |
| O(N) slot scan for ready slots | ~3 ns (amortized) | strategy_noalloc.cpp:21-23 |
| CompletionRecord + in_flight atomic | ~2 ns | strategy_common.hpp:74-79 |
| stdexec set_value propagation | ~5 ns | operation_base_mixin.hpp:176-177 |
| Sender chain overhead (scope.nest + then) | ~6 ns | stdexec internal |
| **Total** | **~37 ns/op** | |

## Optimization 1: Eliminate Per-Start Proxy Allocation

**Impact**: ~4 ns/op saved (11% of budget)
**Risk**: Low
**Files**: `include/dsa_stdexec/operations/operation_base_mixin.hpp`

### Problem

`DsaOperationMixin::start()` at line 145 creates a new `pro::proxy<OperationFacade>`
on every operation start:

```cpp
self.proxy = pro::make_proxy<OperationFacade>(Wrapper{&self});
```

The `Wrapper` struct captures `&self`, but `self` doesn't move after construction —
the proxy could be initialized once and reused.

### Approach

Store the `Wrapper` directly inside `OperationBase` and construct the proxy once
during `connect()` (or lazily on first `start()`), not on every `start()` call.

Two options:
- **Option A**: Add a `set_proxy()` method called from the operation constructor,
  storing a `Wrapper` that captures `this`. Since the operation state lives in
  placement-new storage that doesn't move, the pointer is stable.
- **Option B**: Replace `pro::proxy<OperationFacade>` with a simple vtable-like
  struct containing two function pointers (`notify` and `get_descriptor`), eliminating
  proxy overhead entirely. This is safe because OperationBase already knows its
  concrete type through the Wrapper.

**Recommended**: Option B — minimal proxy overhead. The function pointer pair is
8 bytes smaller than `pro::proxy` and has zero allocation cost.

### Implementation

1. In `operation_base.hpp`, replace:
   ```cpp
   struct OperationBase {
     pro::proxy<OperationFacade> proxy;
     OperationBase *next = nullptr;
   };
   ```
   With:
   ```cpp
   struct OperationBase {
     void (*notify_fn)(OperationBase *self) = nullptr;
     dsa_hw_desc *(*get_descriptor_fn)(OperationBase *self) = nullptr;
     OperationBase *next = nullptr;

     void notify() { notify_fn(this); }
     dsa_hw_desc *get_descriptor() { return get_descriptor_fn(this); }
   };
   ```

2. In `operation_base_mixin.hpp::start()`, replace `pro::make_proxy` with:
   ```cpp
   // Set up function pointers (no allocation, just pointer assignment)
   using Self = std::remove_reference_t<decltype(self)>;
   self.notify_fn = [](OperationBase *base) {
     static_cast<Self *>(base)->notify();
   };
   self.get_descriptor_fn = [](OperationBase *base) {
     return static_cast<Self *>(base)->desc_ptr();
   };
   ```

3. In `task_queue.hpp`, change `op->proxy->notify()` to `op->notify()`.

4. Remove `#include <proxy/proxy.h>` from `operation_base.hpp` and the
   `PRO_DEF_MEM_DISPATCH` / `OperationFacade` definitions.

5. Update `dsa_facade.hpp` if it references `OperationFacade` for the sink interface.

6. Run mock benchmark to verify improvement.

## Optimization 2: Indexed Array Task Queue (Replace O(N) Poll)

**Impact**: ~3-5 ns/op saved (8-14% of budget), eliminates bistable behavior on real DSA
**Risk**: Medium (new data structure)
**Files**: `src/dsa/task_queue.hpp`, `src/dsa/mock_dsa.hpp`

### Problem

`LockedTaskQueue::poll()` traverses the entire linked list of in-flight operations,
checking each completion record. With concurrency=2048, this means 2048 pointer-chasing
loads per poll call, even if only a few completed.

### Approach

Replace the intrusive linked list with a fixed-size array indexed by slot ID.
Operations get a slot index on push; poll iterates only a compact bitmap or
checks only newly-completed entries.

**Option A — Indexed Array + Completion Bitmap**:
- Fixed array of `OperationBase*` pointers (size = max concurrency)
- Free-list stack for O(1) slot allocation
- Poll checks completion records of active slots (bitmap-guided)
- Completed slots are freed and ops notified

**Option B — Epoch-based batch poll**:
- Group operations by submission epoch
- Poll only checks the oldest epoch's completion records
- When an epoch is fully complete, notify all at once

**Recommended**: Option A — straightforward, cache-friendly array access vs pointer chasing.

### Implementation

```cpp
template <dsa_stdexec::HwContext HwCtx>
class IndexedTaskQueue {
  struct Slot {
    dsa_stdexec::OperationBase *op = nullptr;
  };

  std::vector<Slot> slots_;
  std::vector<uint32_t> free_stack_;  // free slot indices
  uint32_t active_count_ = 0;
  // Compact list of active slot indices for poll
  std::vector<uint32_t> active_list_;
  HwCtx hw_ctx_;

public:
  void push(OperationBase *op) {
    uint32_t idx = free_stack_.back();
    free_stack_.pop_back();
    slots_[idx].op = op;
    active_list_.push_back(idx);
    active_count_++;
  }

  size_t poll() {
    // Iterate active_list_, check completion, swap-remove completed
    size_t completed = 0;
    size_t i = 0;
    while (i < active_list_.size()) {
      auto *op = slots_[active_list_[i]].op;
      if (hw_ctx_.check_completion(op)) {
        free_stack_.push_back(active_list_[i]);
        slots_[active_list_[i]].op = nullptr;
        // Swap with last for O(1) removal
        active_list_[i] = active_list_.back();
        active_list_.pop_back();
        op->notify();  // or batch notify after loop
        completed++;
      } else {
        i++;
      }
    }
    active_count_ -= completed;
    return completed;
  }
};
```

Key advantage: sequential array access instead of pointer-chasing linked list.
With mock DSA (instant completion), poll becomes: check status byte (always success) +
notify — no wasted traversal of non-complete items.

## Optimization 3: Use SlotArena in sliding_window_noalloc (Fix O(N) Slot Scan)

**Impact**: ~2-3 ns/op saved at high concurrency, fixes c=4096 regression
**Risk**: Low (SlotArena already exists in helpers.hpp:258)
**Files**: `benchmark/dsa/strategy_noalloc.cpp`

### Problem

The inner loop in `sliding_window_noalloc_impl_inline` scans ALL slots looking for
`ready == true`:

```cpp
for (auto &slot : slots) {
  if (!slot->ready.load(std::memory_order_acquire)) continue;
  // ... start op
}
```

At c=4096, this touches 4096 cache lines per scan. With only ~32 slots ready per pass,
97% of cache line loads are wasted.

### Approach

Replace the O(N) slot vector scan with the existing `SlotArena` (helpers.hpp:258),
which provides O(1) acquire/release via an intrusive free-list:

```cpp
SlotArena<SlotSize> arena(concurrency);
while (next_op < num_ops) {
  while (auto *slot = arena.acquire()) {
    if (next_op >= num_ops) { arena.release(slot); break; }
    size_t offset = next_op * msg_size;
    slot->start_op(scope.nest(make_sender(offset) | stdexec::then(record)));
    ++next_op;
  }
  dsa.poll();
}
```

The `ArenaReceiver` (helpers.hpp:291) already handles releasing slots back to the arena
on completion. This replaces `SlotReceiver` + atomic ready flag with the free-list.

### Implementation

1. Change `sliding_window_noalloc_impl_inline` to use `SlotArena<SlotSize>` instead of
   `vector<unique_ptr<OperationSlot<SlotSize>>>`.
2. Use `ArenaReceiver<SlotSize>` instead of `SlotReceiver` (or modify `start_op` to
   accept the arena).
3. The arena's `acquire()` returns nullptr when all slots are in-flight, naturally
   rate-limiting to the concurrency level.

## Optimization 4: Conditional pre_poll Flush

**Impact**: Enables effective batching on real DSA (could unlock 25-30 Mpps)
**Risk**: Medium (affects real DSA correctness — must not starve)
**Files**: `src/dsa/descriptor_submitter.hpp`

### Problem

`MirroredRingSubmitter::pre_poll()` unconditionally flushes partial batches every poll.
The sliding_window_noalloc loop calls `dsa.poll()` after each scan pass, so the
effective batch size = ops per scan (~32), regardless of configured `batch_size`.

### Approach

Only flush when the batch is "full enough" or when no new work is being submitted:

```cpp
void pre_poll() {
  BatchEntry &current = batches_[batch_index(batch_fill_)];
  if (current.state != BatchState::Filling || current.count == 0) {
    reclaim_completed();
    return;
  }
  // Only flush if batch is at least half full, or all other batches are complete
  // (i.e., we're about to starve the hardware)
  bool should_flush = (current.count >= max_batch_size_ / 2)
                   || (batch_fill_ == batch_head_);  // no other batches in flight
  if (should_flush) {
    seal_and_submit_current();
  }
  reclaim_completed();
}
```

This is a real-DSA-only optimization. For mock DSA it has no effect since there's no
hardware batching.

## Execution Order

1. **Opt 1 (proxy elimination)** — independent, low risk, measurable with mock
2. **Opt 3 (SlotArena)** — independent, low risk, fixes c=4096 regression
3. **Opt 2 (indexed queue)** — medium risk, biggest impact on real DSA
4. **Opt 4 (conditional flush)** — depends on Opt 2 being validated

After each optimization, run mock benchmark to measure improvement.

## Success Criteria

| Metric | Current | Target |
|--------|---------|--------|
| Mock NoLock sliding_window c=2048 msg=8 | 25-27 Mpps | 35-40 Mpps |
| Mock NoLock sliding_window c=4096 msg=8 | ~18 Mpps (regressed) | 35+ Mpps |
| Real DSA sliding_window c=2048 msg=8 | 20-22 Mpps | 28-32 Mpps |
| Run-to-run variance (real DSA) | 8-22 Mpps | <15% variance |
