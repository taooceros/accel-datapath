# Plan: Hardware Batch Doorbell MMIO

## Goal

Add hardware batch descriptor support (opcode 0x01) to reduce MMIO doorbell writes.
Keep all three variants available and benchmarkable:

1. **`DsaBase`** (existing) — immediate 1:1 doorbell per descriptor, unchanged
2. **`DsaBatchBase`** (new, transparent) — stages descriptors, flushes as hardware batch
3. **`BatchSender`** (new, explicit) — user-constructed batch of N senders, one doorbell

All three share the same device discovery, task queue, and polling infrastructure.
Operation senders (`dsa_data_move`, etc.) work with all variants unchanged — they are
already templated on `DsaType` and only call `dsa.submit(op, desc)`.

---

## Architecture Overview

```
                        Existing (unchanged)
                        ────────────────────
   op.start() ──▶ DsaBase::submit(op, desc) ──▶ _movdir64b(portal, desc)  [1 doorbell]
                                             └──▶ task_queue_.push(op)

                        Variant 2: Transparent Batching
                        ───────────────────────────────
   op.start() ──▶ DsaBatchBase::submit(op, desc) ──▶ memcpy desc into staged_descs[N]
                                                  └──▶ task_queue_.push(op)
              ... more ops ...
   flush()    ──▶ build batch_desc{opcode=0x01, desc_list_addr=staged_descs, count=N}
              ──▶ _movdir64b(portal, &batch_desc)                          [1 doorbell]

                        Variant 3: Explicit Batch Sender
                        ────────────────────────────────
   dsa_batch(dsa,
     dsa_data_move(dsa, s1, d1, sz),      BatchOperation::start():
     dsa_data_move(dsa, s2, d2, sz),  ──▶   collect descriptors from children
     dsa_data_move(dsa, s3, d3, sz)  ──▶   build batch_desc + contiguous array
   )                                  ──▶   _movdir64b(portal, &batch_desc)    [1 doorbell]
                                      ──▶   task_queue_.push(batch_op)
```

---

## Part 1: Transparent Batching (`DsaBatchBase`)

### 1.1 New class: `DsaBatchBase` (`src/dsa/dsa_batch.hpp`)

Wraps a `DsaBase` (has-a, not is-a) and intercepts `submit()` to stage descriptors.
Delegates device init, polling, and task queue to the inner `DsaBase`.

```cpp
template <template <typename> class QueueTemplate = dsa::MutexTaskQueue>
class DsaBatchBase {
public:
  using Inner = DsaBase<QueueTemplate>;
  using Queue = typename Inner::Queue;

  explicit DsaBatchBase(bool start_poller = true);

  // Same interface as DsaBase — senders don't know the difference
  void submit(dsa_stdexec::OperationBase *op, dsa_hw_desc *desc);
  void submit(dsa_stdexec::OperationBase *op);
  void poll();
  void flush();

  Queue &task_queue() noexcept;
  const Queue &task_queue() const noexcept;

private:
  Inner inner_;

  // Double-buffered staging array
  static constexpr size_t kMaxStagingSize = 32;
  alignas(64) dsa_hw_desc staged_[2][kMaxStagingSize];
  size_t staged_count_ = 0;
  int active_buf_ = 0;

  // Batch completion records (one per buffer, for lifetime tracking)
  alignas(32) dsa_completion_record batch_comp_[2] = {};
  bool batch_submitted_[2] = {false, false};

  // Device batch size limit (queried from hardware at init)
  size_t max_batch_size_ = kMaxStagingSize;
};
```

**Key design decision:** Composition over inheritance. `DsaBatchBase` has-a `DsaBase`
and forwards `poll()` / `task_queue()` to it. It overrides only the submission path.
This means `DsaBase` stays completely unchanged — no risk of regression.

### 1.2 `DsaBatchBase::submit()` — stage instead of doorbell

```cpp
void submit(OperationBase *op, dsa_hw_desc *desc) {
    if (desc != nullptr) {
        memcpy(&staged_[active_buf_][staged_count_], desc, sizeof(dsa_hw_desc));
        staged_count_++;
        if (staged_count_ >= max_batch_size_) {
            flush();  // auto-flush when staging buffer is full
        }
    }
    inner_.task_queue().push(op);  // queue for completion polling (unchanged)
}
```

Each sub-descriptor's `completion_addr` still points to the original operation's
`comp_buffer_`. The memcpy preserves this pointer. Hardware writes completion status
to each operation's own completion record. The existing `check_completion()` path
reads from `comp_ptr()->status` — completely unchanged.

### 1.3 `DsaBatchBase::flush()` — single doorbell for all staged descriptors

```cpp
void flush() {
    if (staged_count_ == 0) return;

    int prev = active_buf_ ^ 1;

    // Wait for previous batch's descriptor array to be released by hardware.
    // Hardware DMA-reads the array asynchronously after _movdir64b.
    // The array is safe to reuse once batch_comp_[prev].status != 0.
    // In practice this rarely spins — poll() runs between flushes.
    if (batch_submitted_[prev]) {
        while (batch_comp_[prev].status == 0) {
            _mm_pause();
        }
        batch_submitted_[prev] = false;
    }

    if (staged_count_ == 1) {
        // Single descriptor — submit directly, no batch overhead.
        // Descriptor array is a copy, so safe to submit from staged buffer.
        _mm_sfence();
        // use inner_'s portal/mode via a new submit_raw() or direct access
        submit_single(&staged_[active_buf_][0]);
    } else {
        // Build batch descriptor
        dsa_hw_desc batch{};
        memset(&batch, 0, sizeof(batch));
        batch.opcode = DSA_OPCODE_BATCH;
        batch.flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV;
        batch.desc_list_addr = reinterpret_cast<uint64_t>(&staged_[active_buf_][0]);
        batch.desc_count = static_cast<uint32_t>(staged_count_);
        batch.completion_addr = reinterpret_cast<uint64_t>(&batch_comp_[active_buf_]);
        memset(&batch_comp_[active_buf_], 0, sizeof(dsa_completion_record));

        _mm_sfence();
        submit_single(&batch);
        batch_submitted_[active_buf_] = true;
    }

    active_buf_ ^= 1;
    staged_count_ = 0;
}
```

### 1.4 Descriptor array lifetime (the hard constraint)

For non-batch: `_movdir64b` atomically copies 64 bytes into the device portal.
Source memory is free immediately.

For batch: The 64-byte batch descriptor is copied into the portal, but it contains
`desc_list_addr` — a pointer to the staging array in host RAM. The device DMA-reads
this array asynchronously. The array must remain valid until `batch_comp_.status != 0`.

Solution: **double-buffer**. While hardware reads buffer A, new descriptors stage
into buffer B. Before reusing A, check that A's batch completion is done.

```
Time ──────────────────────────────────────────────────▶

Buffer A: [desc0..desc4] submitted ──── HW reading ──── batch_comp_[A].status=1 ── reusable
Buffer B:                              [desc5..desc8] staging ── submitted ── HW reading ──
Buffer A:                                                        staging next batch ──
```

### 1.5 Hardware portal access

`DsaBatchBase` needs access to `wq_portal_` and `mode_` for the doorbell write.
Options:
- Add `DsaBase::submit_raw(dsa_hw_desc *desc)` that just does the MMIO write
  without pushing to the task queue. `DsaBatchBase` calls this for both single
  descriptors and batch descriptors.
- Or expose `portal()` and `mode()` from `DsaBase` (they're already on `DsaHwContext`).

Recommend `submit_raw()` — it keeps the MMIO logic in one place and handles the
dedicated vs shared WQ branching.

### 1.6 Max batch size query

During `DsaBatchBase` construction, after inner DsaBase discovers the device:

```cpp
// libaccel-config API
int wq_max = accfg_wq_get_max_batch_size(inner_.wq());
max_batch_size_ = std::min(static_cast<size_t>(wq_max), kMaxStagingSize);
```

Need to expose `wq_` from `DsaBase` (or query it during init and pass to
`DsaBatchBase`).

### 1.7 Run loop / poller integration

**Inline polling:** The poll lambda becomes:

```cpp
PollingRunLoop loop([&dsa] { dsa.flush(); dsa.poll(); });
```

No changes to `PollingRunLoop` itself. The flush happens every iteration — after
task execution stages descriptors, flush submits them as a batch, then poll checks
completion records.

**Background poller:** The poller thread becomes:

```cpp
poller_ = std::thread([this] {
    while (running_) { flush(); inner_.poll(); }
});
```

### 1.8 Thread safety

**Inline mode (single-thread):** No locking needed. `submit()`, `flush()`, `poll()`
all run on the run loop thread.

**Threaded mode:** `submit()` may be called from any thread (via `start()`), while
`flush()` runs on the poller thread. The staging buffer needs synchronization.

For v1: protect `staged_` / `staged_count_` / `active_buf_` with a spinlock (matches
existing task queue pattern). The critical section is one 64-byte memcpy + increment.

### 1.9 Page fault retry

When `notify()` detects a page fault, it calls `dsa_.submit(this, desc)` with the
modified descriptor. With transparent batching, this stages the retry into the current
batch. On the next `flush()`, it gets submitted with other pending work. This is
correct — the retry descriptor is a fresh copy in the staging array.

### 1.10 Type aliases

```cpp
// Transparent batching variants (parallel the existing aliases)
using DsaBatch = DsaBatchBase<dsa::MutexTaskQueue>;
using DsaBatchSingleThread = DsaBatchBase<dsa::SingleThreadTaskQueue>;
using DsaBatchSpinlock = DsaBatchBase<dsa::SpinlockTaskQueue>;
using DsaBatchLockFree = DsaBatchBase<dsa::LockFreeTaskQueue>;
// etc.
```

---

## Part 2: Explicit Batch Sender (`dsa_batch`)

### 2.1 User-facing API

```cpp
// Static batch — number of operations known at compile time
auto sender = dsa_batch(
    dsa_data_move(dsa, src1, dst1, 4096),
    dsa_mem_fill(dsa, dst2, pattern, 4096),
    dsa_data_move(dsa, src3, dst3, 4096)
);
// completion_signatures: set_value_t(), set_error_t(exception_ptr)

// Use like any other sender
scope.spawn(sender | stdexec::then([] { /* all done */ }));
co_await sender;
```

### 2.2 `BatchSender` (`include/dsa_stdexec/operations/batch.hpp`)

```cpp
template <class DsaType, class... ChildSenders>
class BatchSender {
public:
    using sender_concept = stdexec::sender_t;
    using completion_signatures =
        stdexec::completion_signatures<stdexec::set_value_t(),
                                       stdexec::set_error_t(std::exception_ptr)>;

    BatchSender(DsaType &dsa, ChildSenders... children);

    template <stdexec::receiver Receiver>
    auto connect(Receiver &&r) -> BatchOperation<DsaType, stdexec::__id<...>, ChildSenders...>;
};

template <class DsaType, class... ChildSenders>
auto dsa_batch(ChildSenders&&... senders) -> BatchSender<DsaType, std::decay_t<ChildSenders>...>;
```

### 2.3 `BatchOperation` — the operation state

Owns:
- A contiguous 64-byte-aligned array of N descriptors (N = sizeof...(ChildSenders))
- A batch completion record (32-byte aligned)
- Connected child operation states (to extract their descriptors and handle notify)
- The receiver

```cpp
template <class DsaType, class ReceiverId, class... ChildSenders>
class BatchOperation : public dsa::DsaOperationBase {
    static constexpr size_t N = sizeof...(ChildSenders);

    // Contiguous descriptor array for the batch
    alignas(64) dsa_hw_desc desc_array_[N];

    // Batch completion record
    alignas(32) dsa_completion_record batch_comp_ = {};

    // Child operation states (tuple of connected operations)
    // Each child's start() fills its own descriptor. We then
    // copy those descriptors into desc_array_ before submitting.
    std::tuple<ChildOpStates...> children_;

    DsaType &dsa_;
    Receiver receiver_;
};
```

### 2.4 `BatchOperation::start()` flow

```
start()
  ├─ for each child in children_:
  │    child.fill_descriptor()         // fill desc + comp fields, but don't submit
  │    memcpy child.desc_ptr() → desc_array_[i]   // copy into contiguous array
  │    // completion_addr in the copy still points to child's comp_buffer_
  ├─ build batch descriptor:
  │    opcode = DSA_OPCODE_BATCH
  │    desc_list_addr = &desc_array_[0]
  │    desc_count = N
  │    completion_addr = &batch_comp_
  ├─ _mm_sfence()
  ├─ dsa_.submit_raw(&batch_desc)      // ONE doorbell
  └─ push self into task queue          // ONE queue entry for the batch
```

**Challenge:** Today, `start()` on a child operation both fills the descriptor AND
calls `dsa_.submit()`. For the explicit batch, we need to separate descriptor filling
from submission. Two approaches:

**Option A: Two-phase start.** Add a `prepare()` method to operations that fills the
descriptor without submitting. `start()` = `prepare()` + `submit()`. The batch
operation calls `prepare()` on each child, copies descriptors, then submits once.

**Option B: No-submit flag.** Pass a flag or use a different DsaType wrapper that
captures the descriptor instead of submitting. E.g., `DsaCapture` that records the
descriptor pointer when `submit()` is called but doesn't ring the doorbell.

Recommend Option A — it's cleaner and doesn't require a fake DsaType.

### 2.5 `BatchOperation::notify()` flow

The batch operation is polled like any other operation. But it checks the **batch
completion record**, not individual ones:

```
notify()
  ├─ if batch_comp_.status == SUCCESS:
  │    for each child: child.notify()    // each reads its own comp record
  │    // all should be SUCCESS
  │    stdexec::set_value(receiver_)
  │
  ├─ if batch_comp_.status == BATCH_FAIL (0x05):
  │    // Some sub-descriptors failed. Check each child's comp record.
  │    for each child:
  │      if child comp status == PAGE_FAULT:
  │        touch page, resubmit child individually (falls back to non-batch)
  │      else if child comp status == ERROR:
  │        collect error
  │    if all retries handled: wait for individual completions
  │    else: stdexec::set_error(receiver_, aggregated_error)
  │
  └─ if batch_comp_.status == BATCH_PAGE_FAULT (0x06):
       // Fault reading the descriptor array itself (shouldn't happen since
       // desc_array_ is a member). Log fatal error.
```

### 2.6 Descriptor array lifetime

For the explicit batch sender, the `desc_array_` lives inside `BatchOperation` which
is alive in the task queue until `notify()` completes it. So the lifetime is naturally
correct — no double-buffering needed.

---

## Part 3: Keeping All Variants Benchmarkable

### 3.1 `DsaRef` type-erasure update (`benchmark/dsa_benchmark.cpp`)

The existing `DsaRef` erases over `submit(op, desc)`, `submit(op)`, and `poll()`.
For `DsaBatchBase`, add `flush()`:

```cpp
class DsaRef {
public:
  template <typename DsaType>
  explicit DsaRef(DsaType &dsa)
      : submit_desc_([&dsa](...) { dsa.submit(op, desc); }),
        submit_([&dsa](...) { dsa.submit(op); }),
        poll_([&dsa] { dsa.poll(); }),
        flush_([&dsa] {
            if constexpr (requires { dsa.flush(); }) {
                dsa.flush();
            }
        }) {}

  void submit(OperationBase *op, dsa_hw_desc *desc) { submit_desc_(op, desc); }
  void submit(OperationBase *op) { submit_(op); }
  void poll() { poll_(); }
  void flush() { flush_(); }  // no-op for non-batching variants

private:
  std::function<void(OperationBase *, dsa_hw_desc *)> submit_desc_;
  std::function<void(OperationBase *)> submit_;
  std::function<void()> poll_;
  std::function<void()> flush_;
};
```

### 3.2 Benchmark config extension (`benchmark/benchmark_config.toml`)

Add a new dimension for submission strategy:

```toml
[submission]
run_immediate = true    # DsaBase (existing, 1:1 doorbell)
run_batch = true        # DsaBatchBase (transparent batching)

[batch]
max_batch_size = 32     # staging buffer size for transparent batching
```

### 3.3 Benchmark dispatch extension

In the benchmark loop, add DsaBatch variants alongside existing ones:

```cpp
if (config.run_batch_mutex) {
    DsaBatch concrete_dsa(use_threaded_polling);
    DsaRef dsa(concrete_dsa);
    result.batch_mutex = run_benchmark(dsa, ...);
}
```

The `queue_type` column in CSV output gets new values like `"batch_mutex"`,
`"batch_ttas"`, etc. The visualization dashboard picks them up automatically.

### 3.4 The explicit BatchSender in benchmarks

Add a new scheduling pattern `"batch_submit"` that groups N operations and submits
them via `dsa_batch(...)` instead of individual senders:

```cpp
void run_batch_submit_inline(DsaRef &dsa, ...) {
    // Submit groups of `concurrency` operations as hardware batches
    for (size_t i = 0; i < num_ops; i += batch_size) {
        auto batch = build_batch(dsa, ops[i..i+batch_size]);
        scope.spawn(batch | stdexec::then(record));
    }
}
```

---

## File Change Summary

### New files

| File | Description |
|------|-------------|
| `src/dsa/dsa_batch.hpp` | `DsaBatchBase` class template + type aliases |
| `src/dsa/dsa_batch.ipp` | `DsaBatchBase` implementation (submit, flush) |
| `src/dsa/dsa_batch_instantiate.cpp` | Explicit template instantiations |
| `include/dsa_stdexec/operations/batch.hpp` | `BatchSender` + `BatchOperation` + `dsa_batch()` |

### Modified files

| File | Change |
|------|--------|
| `src/dsa/dsa.hpp` | Add `submit_raw(dsa_hw_desc *)` for raw doorbell write (no queue push). Expose `wq_` accessor for batch size query. |
| `src/dsa/dsa.ipp` | Implement `submit_raw()`. |
| `src/dsa/dsa_operation_base.hpp` | Add `prepare()` method that fills descriptor without submitting (for explicit batch). Or add a virtual/concept method. |
| `benchmark/dsa_benchmark.cpp` | Extend `DsaRef` with `flush()`. Add batch variant dispatch. Add `batch_submit` scheduling pattern. |
| `benchmark/benchmark_config.hpp` | Parse `[submission]` and `[batch]` config sections. |
| `benchmark/benchmark_config.toml` | Add submission strategy and batch config. |
| `xmake.lua` | Add new source files to build targets. |

### Unchanged files

| File | Why unchanged |
|------|--------------|
| `include/dsa_stdexec/operations/data_move.hpp` | Senders are templated on DsaType, work with DsaBatchBase as-is |
| `include/dsa_stdexec/operations/*.hpp` (all others) | Same — only call `dsa.submit(op, desc)` |
| `include/dsa_stdexec/run_loop.hpp` | No structural change. Users pass `[&dsa]{ dsa.flush(); dsa.poll(); }` |
| `include/dsa_stdexec/operation_base.hpp` | Unchanged — OperationFacade already has `get_descriptor()` |
| `src/dsa/task_queue.hpp` | Unchanged — polls individual comp records as before |
| `include/dsa_stdexec/sync_wait.hpp` | Unchanged |

---

## Implementation Order

1. **`submit_raw()` on DsaBase** — expose the raw doorbell write. Small, safe change.
2. **`DsaBatchBase`** — transparent batching with double-buffer. Test with existing
   senders and examples.
3. **Benchmark integration** — add DsaBatch variants to benchmark dispatch. Run
   comparative benchmarks.
4. **`BatchSender`** (explicit) — add `prepare()` to operation base, implement
   BatchOperation. Add `batch_submit` scheduling pattern to benchmarks.
5. **Tune** — benchmark different `max_batch_size` values, flush strategies.

---

## Testing Strategy

1. **Correctness (DsaBatchBase):** Run all existing examples (`example_data_move`,
   `example_mem_fill`, etc.) with `DsaBatch` instead of `Dsa`. All results must match.
2. **Correctness (BatchSender):** New test that batches 2-8 mixed operations and
   verifies all complete correctly.
3. **Batch size 1:** Verify single-descriptor degenerate case avoids batch overhead.
4. **Page fault retry:** mmap without prefault, verify retry through staging path.
5. **Lifetime:** Stress test with high concurrency to exercise double-buffer swap.
6. **Benchmark:** Compare all three variants (immediate, transparent batch, explicit
   batch) across message sizes and concurrency levels. Expect batch wins for small
   messages where doorbell overhead dominates.

---

## Open Questions

- **Optimal staging buffer size:** Start with 32, benchmark up to device max.
- **Flush policy:** Flush only at poll boundaries (better batching) vs also on
  threshold (lower latency). Could be a template parameter or runtime config.
- **Shared WQ:** Batch submission reduces `_enqcmd` retries since there's only one
  portal write per batch. This may be the biggest win. Needs benchmarking.
- **`prepare()` method design:** For explicit BatchSender, how to separate descriptor
  filling from submission cleanly without adding overhead to the non-batch path.
  Consider a concept or CRTP approach.
