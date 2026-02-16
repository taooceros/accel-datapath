# Decouple DsaSink: Extract DescriptorSubmitter Strategy (Part 2)

## Context

Part 1 (committed as `03d7cbf`) decoupled the stdexec operation layer from hardware
types by introducing the `DsaSink` concept boundary. Now we tackle the hardware layer
itself.

Today there are 4 separate classes that all satisfy `DsaSink`:
- `DsaBase` — direct MMIO submission
- `DsaBatchBase` — double-buffered staging
- `DsaRingBatchBase` — ring buffer staging
- `DsaFixedRingBatchBase` — fixed-capacity ring staging

They share identical boilerplate:
- `submit(op)` → `task_queue_.push(op)` (or `inner_.task_queue().push(op)`)
- `poll()` → `task_queue_.poll()` (or flush + `inner_.poll()`)
- Constructor: device discovery, WQ mapping, optional poller thread
- Destructor: stop poller, join thread
- `task_queue()` accessor, `flush()` stub

The 3 batch classes use composition (`has-a DsaBase`) and only differ in how they
stage descriptors before calling `inner_.submit_raw()`. But each reimplements the
full `submit(op, desc)` / `submit(op)` / `poll()` / constructor / destructor pattern.

## Goal

Extract the descriptor submission strategy into a pluggable component so that
`DsaBase` becomes `DsaEngine<DescriptorSubmitter>`, eliminating `DsaBatchBase`,
`DsaRingBatchBase`, and `DsaFixedRingBatchBase` as separate top-level classes.

```
BEFORE:  DsaBase  DsaBatchBase  DsaRingBatchBase  DsaFixedRingBatchBase
         (4 separate classes, duplicated boilerplate)

AFTER:   DsaEngine<DirectSubmitter>       = old DsaBase
         DsaEngine<StagingSubmitter>      = old DsaBatchBase
         DsaEngine<RingSubmitter>         = old DsaRingBatchBase
         DsaEngine<FixedRingSubmitter>    = old DsaFixedRingBatchBase
```

## Design

### `DescriptorSubmitter` concept

```cpp
template <typename T>
concept DescriptorSubmitter = requires(T &s, dsa_hw_desc *desc) {
    { s.submit_descriptor(desc) } -> std::same_as<void>;
    { s.flush() } -> std::same_as<void>;
    { s.pre_poll() } -> std::same_as<void>;
};
```

- `submit_descriptor(desc)` — stage or submit a descriptor to hardware
- `flush()` — flush any staged descriptors (no-op for direct)
- `pre_poll()` — hook called at the start of `poll()`, before `task_queue_.poll()`.
  Batch variants use this to drain partial batches before polling completions.

### Submitter implementations

#### `DirectSubmitter`

Replaces the MMIO logic currently inline in `DsaBase::submit()`:

```cpp
class DirectSubmitter {
public:
    void init(void *portal, accfg_wq_mode mode, accfg_wq *);
    void submit_descriptor(dsa_hw_desc *desc);  // _movdir64b / _enqcmd
    void flush() {}
    void pre_poll() {}
};
```

#### `StagingSubmitter`

Replaces `DsaBatchBase`'s double-buffered staging logic. Owns a `DirectSubmitter`
internally for flushing batch descriptors to hardware.

#### `RingSubmitter`

Replaces `DsaRingBatchBase`'s dual-ring logic (256-slot descriptor ring +
16-slot batch metadata ring). Owns a `DirectSubmitter`. Receives a `Queue &`
reference at init for backpressure polling.

#### `FixedRingSubmitter`

Replaces `DsaFixedRingBatchBase`'s fixed-capacity ring logic. Same pattern
as `RingSubmitter`.

### `DsaEngine<Submitter, QueueTemplate>`

Unified class replacing all 4 DSA classes:

```cpp
template <class Submitter, template <typename> class QueueTemplate>
class DsaEngine {
public:
    void submit(OperationBase *op, dsa_hw_desc *desc) {
        if (desc) submitter_.submit_descriptor(desc);
        task_queue_.push(op);
    }
    void submit(OperationBase *op) { task_queue_.push(op); }
    void poll() { submitter_.pre_poll(); task_queue_.poll(); }
    void flush() { submitter_.flush(); }
    void submit_raw(dsa_hw_desc *desc);  // kept for BatchOperation

private:
    AccfgCtx ctx_;
    accfg_wq *wq_;
    void *wq_portal_;
    Submitter submitter_;
    Queue task_queue_;
    std::thread poller_;
    std::atomic<bool> running_{false};
};
```

### Backpressure in ring submitters

Ring/FixedRing submitters spin-wait when the ring is full, calling
`task_queue_.poll()` to drain completions. The submitter receives a `Queue &`
reference at init:

```cpp
void init(void *portal, accfg_wq_mode mode, accfg_wq *wq, Queue &task_queue);
```

## Files

| File | Action | Description |
|------|--------|-------------|
| `src/dsa/descriptor_submitter.hpp` | **CREATE** | Concept + 4 submitter classes |
| `src/dsa/dsa.hpp` | MODIFY | `DsaBase` → `DsaEngine<Submitter, Q>` |
| `src/dsa/dsa.ipp` | MODIFY | Unified constructor/destructor |
| `src/dsa/dsa_batch.hpp` | **DELETE** | → `StagingSubmitter` |
| `src/dsa/dsa_batch.ipp` | **DELETE** | → `StagingSubmitter` |
| `src/dsa/dsa_ring_batch.hpp` | **DELETE** | → `RingSubmitter` |
| `src/dsa/dsa_ring_batch.ipp` | **DELETE** | → `RingSubmitter` |
| `src/dsa/dsa_fixed_ring_batch.hpp` | **DELETE** | → `FixedRingSubmitter` |
| `src/dsa/dsa_fixed_ring_batch.ipp` | **DELETE** | → `FixedRingSubmitter` |
| `src/dsa/dsa_instantiate.cpp` | MODIFY | All 24 combos |
| `src/dsa/dsa_batch_instantiate.cpp` | **DELETE** | Merged |
| `src/dsa/dsa_ring_batch_instantiate.cpp` | **DELETE** | Merged |
| `src/dsa/dsa_fixed_ring_batch_instantiate.cpp` | **DELETE** | Merged |
| `benchmark/dsa/main.cpp` | MODIFY | Update includes |
| `xmake.lua` | MODIFY | Remove deleted files |

## Type Aliases (backwards compatible)

```cpp
using Dsa             = DsaEngine<DirectSubmitter, MutexTaskQueue>;
using DsaBatch        = DsaEngine<StagingSubmitter, MutexTaskQueue>;
using DsaRingBatch    = DsaEngine<RingSubmitter, MutexTaskQueue>;
using DsaFixedRingBatch = DsaEngine<FixedRingSubmitter, MutexTaskQueue>;
// ... + SingleThread, TasSpinlock, Spinlock, Backoff, LockFree for each
```

## Implementation Order

1. Create `src/dsa/descriptor_submitter.hpp` — concept + `DirectSubmitter`
2. Refactor `dsa.hpp`/`dsa.ipp` — `DsaBase` → `DsaEngine<Submitter, Q>`. Build + test.
3. Add `StagingSubmitter`, delete `dsa_batch.hpp/ipp/instantiate`. Build + test.
4. Add `FixedRingSubmitter`, delete `dsa_fixed_ring_batch.*`. Build + test.
5. Add `RingSubmitter`, delete `dsa_ring_batch.*`. Build + test.
6. Merge instantiation files into single `dsa_instantiate.cpp`. Update `xmake.lua`.
7. Update `benchmark/dsa/main.cpp` includes.
8. Full build + benchmark with all strategies/queues. Zero warnings.

## Risk Notes

- **Backpressure**: Ring submitters get `Queue &` for spin-wait polling.
- **`submit_raw()`**: Kept on `DsaEngine` for `BatchOperation` in `batch.hpp`.
- **Destructor ordering**: Submitter cleanup before poller thread join.
- **24 instantiations**: 4 submitters x 6 queue types, same total as today.
