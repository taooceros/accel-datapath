# `dsa_batch` V1 Implementation Plan

**Status**: cancelled on 2026-04-15

Historical note:
Relationship: Preserved as a historical planning record; future work should start from a fresh plan instead of reviving this stale in-progress file.

## Scope

A single-threaded, inline-polling-only `dsa_batch` sender that submits N DSA
operations as one hardware batch descriptor. One MMIO doorbell write, one task
queue entry, one completion poll per batch — versus N of each today.

No changes to existing operation senders, task queues, or run loop.

## API

```cpp
#include <dsa_stdexec/batch.hpp>

// Factory receives a span of descriptors to fill in-place.
// batch_all sets completion_addr for each descriptor — factory must not.
auto sndr = dsa_batch(dsa, N, [&](std::span<dsa_hw_desc> descs) {
    for (size_t i = 0; i < descs.size(); ++i) {
        dsa::fill_data_move(descs[i], src + i * sz, dst + i * sz, sz);
    }
});

// Composes like any sender
dsa_stdexec::wait_start(std::move(sndr), loop);
```

The factory lambda receives `std::span<dsa_hw_desc>` — direct access to the
contiguous, 64-byte-aligned descriptor array inside `BatchOperation`. Descriptors
are filled in-place. Zero memcpy.

## Descriptor Fill Functions

Free functions extracted from existing operation senders' `start()` logic.
Each fills opcode-specific fields. The caller (factory lambda or `BatchOperation`)
sets `completion_addr` and zeroes the descriptor beforehand.

```cpp
namespace dsa {

// Each function fills opcode + flags + operation-specific fields.
// Does NOT set completion_addr (BatchOperation manages that).
// Descriptor must be zeroed before calling.

inline void fill_data_move(dsa_hw_desc &d, void *src, void *dst, size_t size) {
    d.opcode = DSA_OPCODE_MEMMOVE;
    d.flags  = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV | IDXD_OP_FLAG_CC;
    d.xfer_size = static_cast<uint32_t>(size);
    d.src_addr  = reinterpret_cast<uint64_t>(src);
    d.dst_addr  = reinterpret_cast<uint64_t>(dst);
}

inline void fill_mem_fill(dsa_hw_desc &d, void *dst, size_t size, uint64_t pattern) {
    d.opcode = DSA_OPCODE_MEMFILL;
    d.flags  = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV | IDXD_OP_FLAG_CC;
    d.xfer_size = static_cast<uint32_t>(size);
    d.dst_addr  = reinterpret_cast<uint64_t>(dst);
    d.pattern   = pattern;
}

inline void fill_compare(dsa_hw_desc &d, const void *src1, const void *src2, size_t size) {
    d.opcode = DSA_OPCODE_COMPARE;
    d.flags  = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV;
    d.xfer_size  = static_cast<uint32_t>(size);
    d.src_addr   = reinterpret_cast<uint64_t>(src1);
    d.src2_addr  = reinterpret_cast<uint64_t>(src2);
}

inline void fill_compare_value(dsa_hw_desc &d, const void *src, size_t size, uint64_t pattern) {
    d.opcode = DSA_OPCODE_COMPVAL;
    d.flags  = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV;
    d.xfer_size    = static_cast<uint32_t>(size);
    d.src_addr     = reinterpret_cast<uint64_t>(src);
    d.comp_pattern = pattern;
}

inline void fill_dualcast(dsa_hw_desc &d, const void *src, void *dst1, void *dst2, size_t size) {
    d.opcode = DSA_OPCODE_DUALCAST;
    d.flags  = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV | IDXD_OP_FLAG_CC;
    d.xfer_size = static_cast<uint32_t>(size);
    d.src_addr  = reinterpret_cast<uint64_t>(src);
    d.dst_addr  = reinterpret_cast<uint64_t>(dst1);
    d.dest2     = reinterpret_cast<uint64_t>(dst2);
}

inline void fill_crc_gen(dsa_hw_desc &d, const void *src, size_t size, uint32_t seed = 0) {
    d.opcode = DSA_OPCODE_CRCGEN;
    d.flags  = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV;
    d.xfer_size = static_cast<uint32_t>(size);
    d.src_addr  = reinterpret_cast<uint64_t>(src);
    d.crc_seed  = seed;
}

inline void fill_copy_crc(dsa_hw_desc &d, const void *src, void *dst, size_t size, uint32_t seed = 0) {
    d.opcode = DSA_OPCODE_COPY_CRC;
    d.flags  = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV;
    d.xfer_size = static_cast<uint32_t>(size);
    d.src_addr  = reinterpret_cast<uint64_t>(src);
    d.dst_addr  = reinterpret_cast<uint64_t>(dst);
    d.crc_seed  = seed;
}

inline void fill_cache_flush(dsa_hw_desc &d, void *dst, size_t size) {
    d.opcode = DSA_OPCODE_CFLUSH;
    d.flags  = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV;
    d.xfer_size = static_cast<uint32_t>(size);
    d.dst_addr  = reinterpret_cast<uint64_t>(dst);
}

} // namespace dsa
```

These live in `include/dsa_stdexec/batch.hpp` alongside the sender.
They are thin, inlinable, and can later be used by the existing operation senders
internally if we choose to deduplicate (not in V1).

## Core Types

### `BatchOperation`

```
include/dsa_stdexec/batch.hpp

BatchOperation<DsaType, Factory, ReceiverId>
  : public dsa::DsaOperationBase    // inherits desc_buffer_, comp_buffer_, proxy
```

**Members:**

```cpp
DsaType &dsa_;
Factory factory_;
size_t count_;
Receiver downstream_;

// Sub-descriptor array: contiguous, 64-byte aligned, max 32 entries
// (hardware limit from accfg_wq_get_max_batch_size, typically 32)
static constexpr size_t kMaxBatch = 32;
alignas(64) dsa_hw_desc sub_descs_[kMaxBatch];
alignas(32) dsa_completion_record sub_comps_[kMaxBatch];

// Batch-level completion record (inherited desc/comp used for batch descriptor)
// We reuse the inherited DsaOperationBase's desc_ptr() for the batch descriptor
// and comp_ptr() for the batch completion record.

// Page fault retry tracking
size_t pending_retries_ = 0;
```

**Why inherit `DsaOperationBase`?**
The batch operation itself needs to be pushed to the task queue as an `OperationBase*`.
The task queue calls `check_completion(op)` which reads `comp_ptr()->status`.
We use the inherited `comp_buffer_` for the batch completion record, and `desc_buffer_`
for the batch descriptor. This means `BatchOperation` looks like any other operation
to the task queue — no changes needed to the polling infrastructure.

**Lifecycle — `start()`:**

```
1. Zero sub_descs_[0..count_-1] and sub_comps_[0..count_-1]

2. Set completion_addr for each sub-descriptor:
     sub_descs_[i].completion_addr = &sub_comps_[i]

3. Call factory_(std::span{sub_descs_, count_})
   Factory fills opcode-specific fields in-place.

4. Build batch descriptor (in inherited desc_ptr()):
     desc->opcode         = DSA_OPCODE_BATCH
     desc->flags          = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV
     desc->desc_list_addr = &sub_descs_[0]
     desc->desc_count     = count_
     desc->completion_addr = comp_ptr()

5. Zero batch completion record: memset(comp_ptr(), 0, ...)

6. Init proxy for OperationFacade (notify + get_descriptor)

7. dsa_.submit(this, desc_ptr())
   → single MMIO doorbell write
   → pushes this to task queue (single entry)
```

After step 7, the hardware executes all N sub-descriptors concurrently.
The task queue polls `comp_ptr()->status` (the batch completion record).

**Lifecycle — `notify()`:**

Called when `comp_ptr()->status != 0` (batch completion record written by hardware).

```
batch_status = comp_ptr()->status & DSA_COMP_STATUS_MASK

Case 1: DSA_COMP_SUCCESS
  All sub-ops completed successfully.
  → set_value(std::move(downstream_))

Case 2: DSA_COMP_BATCH_FAIL or DSA_COMP_BATCH_PAGE_FAULT
  Some sub-ops failed. Scan sub_comps_[0..count_-1]:
  For each sub-op with page fault (DSA_COMP_PAGE_FAULT_NOBOF):
    → touch_faulting_page(sub_comps_[i])
    → adjust_for_retry(sub_descs_[i], sub_comps_[i])
    → zero sub_comps_[i]
    → dsa_.submit_raw(&sub_descs_[i])    // individual re-submit
    → pending_retries_++

  If pending_retries_ > 0:
    Switch to retry polling mode (see below)
  Else (all failures are non-recoverable):
    → set_error(std::move(downstream_), DsaError(...))

Case 3: Other error
  → set_error(std::move(downstream_), DsaError(...))
```

**Retry polling mode:**

After re-submitting faulted sub-ops individually, the `BatchOperation` must wait
for those retried ops to complete. Since the batch completion record is already
consumed, we can't reuse it.

Approach: re-enqueue self in the task queue. Override completion check to scan
individual sub-completion records instead of batch completion record.

```cpp
bool retry_complete() const {
    for (size_t i = 0; i < count_; ++i) {
        uint8_t s = sub_comps_[i].status & DSA_COMP_STATUS_MASK;
        if (s == 0) return false;  // still pending
    }
    return true;
}
```

When all retried sub-ops complete, `notify()` is called again. This time it
checks all sub-completion records for success/error and signals downstream.

Implementation detail: after the initial batch completes, set a `in_retry_mode_`
flag. Override the proxy's behavior: when `in_retry_mode_`, `check_completion`
scans sub-comps instead of reading batch comp. This requires either:
(a) Using a different `HwContext::check_completion` path, or
(b) Setting `comp_ptr()->status = 0` and writing it to 1 when `retry_complete()`
    returns true during notify.

Option (b) is simpler — in the retry notify path, after re-submitting faulted ops,
zero the batch `comp_ptr()` and check `retry_complete()` each time `notify()` is
called. But `notify()` is only called when `comp_ptr()->status != 0`...

Simplest approach for V1: after re-submitting retried ops individually, each gets
its own task queue entry via `dsa_.submit(this, &sub_descs_[i])`. But `this` isn't
a per-sub-op OperationBase. We need lightweight stubs.

**V1 simplification: no page fault retry for batch.**

For V1, treat any non-success batch completion as an error. Page fault retry in
batch mode is an optimization for a rare path. The benchmark pre-touches all
buffers, so page faults shouldn't occur in practice. Document as a known limitation.

This eliminates the entire retry polling complexity. `notify()` becomes:

```cpp
void notify() {
    uint8_t status = comp_ptr()->status & DSA_COMP_STATUS_MASK;
    if (status == DSA_COMP_SUCCESS) {
        stdexec::set_value(std::move(downstream_));
    } else {
        auto err = DsaError(status, *comp_ptr(), DSA_OPCODE_BATCH, "batch");
        stdexec::set_error(std::move(downstream_),
                           std::make_exception_ptr(std::move(err)));
    }
}
```

### `BatchSender`

```cpp
template <class DsaType, class Factory>
class BatchSender {
    using sender_concept = stdexec::sender_t;
    using completion_signatures =
        stdexec::completion_signatures<stdexec::set_value_t(),
                                       stdexec::set_error_t(std::exception_ptr)>;

    DsaType &dsa_;
    size_t count_;
    Factory factory_;

    auto connect(stdexec::receiver auto &&r) { ... }
};
```

Completion signature is `set_value_t()` — batch completes with no value.
Operations that produce values (compare, crc_gen) are not meaningful in batch
context since per-op results are not propagated. The batch is for throughput
(data_move, mem_fill, cache_flush, dualcast, etc.).

### `dsa_batch` Free Function

```cpp
template <class DsaType, class Factory>
auto dsa_batch(DsaType &dsa, size_t count, Factory &&factory) {
    return BatchSender<DsaType, std::decay_t<Factory>>(
        dsa, count, std::forward<Factory>(factory));
}
```

## Page Fault Helper (for future use)

Not used in V1 batch, but extracted for consistency. Used by existing senders
and future batch retry:

```cpp
namespace dsa {

inline void touch_faulting_page(const dsa_completion_record &comp) {
    volatile char *t = reinterpret_cast<volatile char *>(comp.fault_addr);
    (comp.status & DSA_COMP_STATUS_WRITE) ? *t = *t : (void)*t;
}

inline void adjust_for_retry(dsa_hw_desc &desc, const dsa_completion_record &comp) {
    desc.src_addr  += comp.bytes_completed;
    desc.dst_addr  += comp.bytes_completed;
    desc.xfer_size -= comp.bytes_completed;

    // Dualcast: also adjust dest2
    if (desc.opcode == DSA_OPCODE_DUALCAST)
        desc.dest2 += comp.bytes_completed;

    // Compare: adjust src2
    if (desc.opcode == DSA_OPCODE_COMPARE)
        desc.src2_addr += comp.bytes_completed;

    // CRC ops: carry partial CRC forward
    if (desc.opcode == DSA_OPCODE_CRCGEN || desc.opcode == DSA_OPCODE_COPY_CRC)
        desc.crc_seed = static_cast<uint32_t>(comp.crc_val);
}

} // namespace dsa
```

## Benchmark Integration: `batch_raw` Strategy

### Strategy Implementation

`benchmark/dsa/strategy_batch_raw.cpp`:

```cpp
void run_batch_raw_inline(DsaProxy &dsa, exec::async_scope &scope,
                          size_t concurrency, size_t msg_size, size_t total_bytes,
                          BufferSet &bufs, LatencyCollector &latency,
                          OperationType op_type) {
    dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

    size_t num_ops = total_bytes / msg_size;
    size_t op_idx = 0;

    while (op_idx < num_ops) {
        size_t batch_size = std::min(concurrency, num_ops - op_idx);

        auto sndr = dsa_batch(dsa, batch_size, [&](std::span<dsa_hw_desc> descs) {
            for (size_t i = 0; i < descs.size(); ++i) {
                size_t offset = (op_idx + i) * msg_size;
                fill_for_op(descs[i], op_type, bufs, offset, msg_size);
            }
        });

        dsa_stdexec::wait_start(std::move(sndr), loop);
        loop.reset();
        op_idx += batch_size;
    }
}
```

Where `fill_for_op` dispatches to the correct `fill_*` based on `OperationType`:

```cpp
static void fill_for_op(dsa_hw_desc &desc, OperationType op_type,
                         BufferSet &bufs, size_t offset, size_t msg_size) {
    switch (op_type) {
    case OperationType::DataMove:
        dsa::fill_data_move(desc, bufs.src.data() + offset,
                            bufs.dst.data() + offset, msg_size);
        break;
    case OperationType::MemFill:
        dsa::fill_mem_fill(desc, bufs.dst.data() + offset,
                           msg_size, BufferSet::fill_pattern);
        break;
    // ... etc for all 8 operations
    }
}
```

### Config Changes

`config.hpp` — add to `SchedulingPattern` enum:
```cpp
enum class SchedulingPattern {
    SlidingWindow, SlidingWindowNoAlloc, SlidingWindowArena,
    Batch, BatchNoAlloc, ScopedWorkers,
    BatchRaw  // <-- new
};
```

`config.cpp` — add name, parse, listing.

`strategies.hpp` — declare `run_batch_raw_inline`, add to dispatch table.
No threaded variant for V1 (single-threaded only).

`benchmark_config.toml` — add under `[scheduling]`:
```toml
batch_raw = false   # opt-in, not in default set
```

`xmake.lua` — add `strategy_batch_raw.cpp` to `dsa_benchmark` sources.

### Cleanup

- Delete `examples/batch_raw.cpp` (superseded by the batch sender)
- Remove `"batch_raw"` from examples target list in `xmake.lua`

## File Summary

### New Files

| File | Contents |
|------|----------|
| `include/dsa_stdexec/batch.hpp` | `fill_*` functions, `BatchSender`, `BatchOperation`, `dsa_batch()` |
| `benchmark/dsa/strategy_batch_raw.cpp` | `run_batch_raw_inline` benchmark strategy |

### Modified Files

| File | Change |
|------|--------|
| `benchmark/dsa/config.hpp` | Add `BatchRaw` to `SchedulingPattern` |
| `benchmark/dsa/config.cpp` | Add name/parse/listing for `BatchRaw` |
| `benchmark/dsa/strategies.hpp` | Declare `run_batch_raw_inline`, add to table |
| `benchmark/dsa/strategy_common.hpp` | Add `fill_for_op` helper |
| `xmake.lua` | Add `strategy_batch_raw.cpp` to benchmark, remove `batch_raw` example |
| `benchmark/benchmark_config.toml` | Add `batch_raw = false` |

### Deleted Files

| File | Reason |
|------|--------|
| `examples/batch_raw.cpp` | Superseded by `dsa_batch` sender |

## V1 Limitations

1. **No page fault retry in batch mode.** Batch failure → error. Pre-touch buffers
   to avoid page faults. Rare in practice with benchmark workloads.

2. **No per-sub-op result propagation.** Completion signature is `set_value_t()`.
   Operations like `compare` and `crc_gen` that produce values cannot return per-op
   results through batch. Use individual senders for those.

3. **Single-threaded inline polling only.** No threaded variant. The `BatchOperation`
   is not thread-safe.

4. **Max batch size = 32.** Fixed array, no dynamic allocation. Hardware limit is
   typically 32 (queried at runtime but capped at compile-time array size). Caller
   must ensure `count <= 32`.

5. **Immediate submission only.** `dsa_batch` calls `dsa_.submit(this, desc)` which
   goes through whatever `DsaType` provides. Using with `DsaBatchBase` (transparent
   batching) would be batch-inside-batch — degenerate but not incorrect. Recommend
   pairing `batch_raw` scheduling with `immediate` submission strategy only.

## Verification

```bash
xmake build dsa_benchmark
run -- --batch-raw --inline --immediate --no-latency --concurrency 32 --msg-size 4096
```

Compare `batch_raw` bandwidth/msg_rate against:
- `batch` with `immediate` — measures per-op sender overhead eliminated by hardware batching
- `batch_noalloc` with `immediate` — measures remaining overhead beyond noalloc
- `batch` with `double_buf_batch` — measures transparent batching vs explicit batching

## Future Work (Not V1)

- **Page fault retry**: Re-submit faulted sub-ops individually, poll to completion.
  Requires lightweight per-sub-op task queue stubs or a retry loop inside `notify()`.
- **Tier 2 composed API**: `dsa_batch_all` using descriptor sink concept to let
  existing operation senders fill descriptors through a collector.
- **`dsa_domain` customization**: Intercept `when_all(dsa_ops...)` and transform
  into hardware batch submission.
- **Zero-copy descriptors**: Pre-allocate descriptor arrays, reuse across batches.
  The `span<dsa_hw_desc>` interface is already compatible.
