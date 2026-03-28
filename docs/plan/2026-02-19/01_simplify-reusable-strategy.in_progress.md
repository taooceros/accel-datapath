# Simplify Reusable Sliding Window Strategy

## Problem

`reusable.cpp` duplicates the arena free-list from `helpers.hpp`. The `ReusableSlotArena` struct is a near-identical copy of `SlotArena<StorageSize>` — same `vector<unique_ptr>` ownership, same `free_head` pointer, same `acquire()`/`release()` logic. The only differences are:

1. `SlotArena` manages `OperationSlot<StorageSize>` — `ReusableSlotArena` manages `ReusableSlot`
2. `ReusableSlotArena` adds an `init_all()` convenience method

This duplication makes the reusable strategy appear more complex than it actually is.

## Root Cause

`SlotArena<StorageSize>` is hardcoded to `OperationSlot<StorageSize>`:

```cpp
template <size_t StorageSize> struct SlotArena {
  using Slot = OperationSlot<StorageSize>;  // <-- hardcoded
  ...
};
```

Any slot type with a `next_free` pointer would work, but the template parameter is `StorageSize` (an integer), not the slot type itself.

## Solution

### Step 1: Introduce `BasicSlotArena<SlotType>` in `helpers.hpp`

Extract the arena logic into a slot-type-generic template:

```cpp
template <typename SlotType> struct BasicSlotArena {
  std::vector<std::unique_ptr<SlotType>> pool;
  SlotType *free_head = nullptr;

  explicit BasicSlotArena(size_t capacity) {
    pool.reserve(capacity);
    for (size_t i = 0; i < capacity; ++i) {
      pool.push_back(std::make_unique<SlotType>());
      pool.back()->next_free = free_head;
      free_head = pool.back().get();
    }
  }

  SlotType *acquire() { ... }   // unchanged
  void release(SlotType *s) { ... }  // unchanged
  bool empty() const { ... }   // unchanged
};
```

### Step 2: Make `SlotArena<StorageSize>` a type alias

Backward-compatible — no changes to arena.cpp, direct.cpp, noalloc.cpp:

```cpp
template <size_t StorageSize>
using SlotArena = BasicSlotArena<OperationSlot<StorageSize>>;
```

### Step 3: Update `ArenaReceiver` and `DirectBenchReceiver`

These receivers hold `SlotArena<StorageSize>*` and `OperationSlot<StorageSize>*`. After step 2, `SlotArena<StorageSize>` is an alias for `BasicSlotArena<OperationSlot<StorageSize>>`, so existing code compiles unchanged. No modifications needed.

### Step 4: Rewrite `reusable.cpp` to use `BasicSlotArena<ReusableSlot>`

- Delete `ReusableSlotArena` entirely
- Replace with `BasicSlotArena<ReusableSlot>`
- Move the `init_all()` call to a free function or inline loop at the call site (it's a 3-line loop)

### Step 5: Build and verify

- `xmake build dsa_benchmark` must compile cleanly
- Optionally run benchmarks to confirm identical performance

## What Stays the Same

These are the **genuine optimizations** in the reusable strategy, not duplication:

- `ReusableSlot` with `relaunch()` — the core optimization (skip connect/start)
- `notify_impl` with `offsetof` — standard intrusive container pattern (same as Linux `container_of`)
- `with_reusable_fill` — descriptor-level dispatch (different abstraction level from `with_op_sender`)

## Files Changed

| File | Change |
|------|--------|
| `benchmark/dsa/helpers.hpp` | Add `BasicSlotArena<SlotType>`, make `SlotArena` an alias |
| `benchmark/dsa/strategies/sliding_window/reusable.cpp` | Delete `ReusableSlotArena`, use `BasicSlotArena<ReusableSlot>` |

## Risk

Low. The type alias ensures all existing strategies compile unchanged. The reusable strategy's hot path (`relaunch` + `notify_impl`) is untouched.
