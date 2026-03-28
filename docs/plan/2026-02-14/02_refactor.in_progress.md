# Refactoring Discussion: batch_all and Descriptor Abstraction

## Starting Point

The original `batch_all` plan (`batch_all.md`) proposed a `StagingDsa` wrapper that
intercepts `submit()` calls from child senders to capture descriptors into a contiguous
array, then submits one hardware batch descriptor. While mechanically sound, the design
discussion revealed deeper architectural questions about separation of concerns.

## Key Observations

### 1. Single-Threaded Simplification

Since we only support single-threaded operation for V1, many complexities disappear:

- **No `CountingReceiver` needed.** After submitting the batch descriptor, we can poll
  the batch completion record directly. `BatchAllOperation` is a single entry in the
  task queue, not N entries.
- **No child operation states kept alive.** Child senders' `start()` is synchronous
  (fills descriptor + calls submit). Nothing is in-flight until the batch descriptor
  is submitted. Child op states are just descriptor-filling scaffolding.
- **No atomic operations.** Single-threaded means no contention on counters or flags.

### 2. StagingDsa is Not a DSA

The `StagingDsa` wrapper satisfies the duck-typed `DsaType` interface but never touches
hardware. It's a **descriptor collector** — it intercepts `submit()` to copy descriptors
into a staging array. Calling it a "DSA" is misleading.

This led to the question: what concept does the `DsaType` template parameter actually
represent?

### 3. The Descriptor Sink Concept

Every operation sender calls `dsa_.submit(this, desc)` in `start()`. The template
parameter `DsaType` is really a **descriptor sink** — something that accepts a filled
descriptor. Three implementations exist:

- `DsaBase::submit()` — writes to hardware portal + pushes to task queue
- `DsaBatchBase::submit()` — copies to staging array, auto-flushes as batch
- `DescriptorCollector::submit()` — copies to staging array for `batch_all`

This parallels the stdexec **scheduler** concept: different schedulers satisfy the same
interface but dispatch work differently. Different descriptor sinks accept descriptors
but route them differently.

### 4. Separation of Submission and Completion

Currently `DsaBase::submit(op, desc)` does two things atomically:
1. Writes descriptor to hardware portal (submission)
2. Pushes `op` to the task queue (completion tracking)

These are conceptually independent. For `batch_all`, we want submission without
per-op completion tracking (the batch manages completion itself). For `DsaBatchBase`,
we want staging without immediate submission (until flush). The bundling is the problem.

### 5. DSA is Not a General Scheduler

In stdexec, `on(scheduler, sender)` means "run this sender on this execution context."
The scheduler is discovered at connect time via the receiver's environment. Senders
don't carry scheduler references.

DSA doesn't fit this model. It's a fixed-function accelerator, not a general execution
context. You can't "get onto DSA and then run arbitrary code." The work IS the
descriptor. There's no separation between "being on the context" and "doing the work."

The current `DsaScheduler::schedule()` pre-sets `comp.status = 1` for immediate
completion — a synthetic operation that just cycles through the task queue. It doesn't
"place" you onto DSA hardware.

**Conclusion:** DSA operation senders carrying a device handle reference is appropriate.
It's not a scheduler — it's a hardware device.

### 6. The Descriptor Filling Layer

Each operation sender's `start()` does three things:
1. Fills a `dsa_hw_desc` with opcode-specific fields (pure computation)
2. Creates a proxy for type-erased callbacks (framework bookkeeping)
3. Calls `dsa_.submit(this, desc)` (hardware interaction)

For `batch_all`, we only need step 1 — the descriptor filling. Steps 2 and 3 are
per-op overhead that batching eliminates.

This suggests extracting descriptor filling as standalone functions:

```cpp
inline void fill_data_move(dsa_hw_desc &desc, void *src, void *dst, size_t size) {
    desc.opcode = DSA_OPCODE_MEMMOVE;
    desc.flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV | IDXD_OP_FLAG_CC;
    desc.xfer_size = static_cast<uint32_t>(size);
    desc.src_addr = reinterpret_cast<uint64_t>(src);
    desc.dst_addr = reinterpret_cast<uint64_t>(dst);
}
```

These are ~5 lines per operation type, inlinable, zero overhead.

### 7. Could Senders Expose `.fill()`?

An alternative to free functions: each DSA sender exposes a `.fill(desc)` method.
The sender already carries the parameters (src, dst, size). `.fill()` would write
them into a descriptor without creating an operation state.

```cpp
auto sndr = dsa_data_move(src, dst, size);  // no dsa reference
sndr.fill(desc);                             // fills descriptor from parameters
```

This would make the sender a **descriptor source** in addition to being a stdexec
sender. `dsa_batch` could use `.fill()` instead of requiring a raw descriptor
callback.

**Decision deferred.** For V1, free `fill_*` functions are simpler and don't require
changing existing sender types. The `.fill()` approach can be added later if the
pattern proves valuable.

### 8. Should Senders Carry a DSA Reference?

If `.fill()` exists and doesn't need a DSA reference, then the DSA reference is only
needed for `submit()` in `start()`. The sender could be constructed without a DSA
reference — it's pure work description.

```cpp
auto sndr = dsa_data_move(src, dst, size);       // no dsa, just parameters
auto placed = dsa_submit(dsa, sndr);             // dsa provided at submission
```

This aligns with stdexec philosophy (senders are descriptions, not executions) but
doesn't map cleanly because DSA isn't a general execution context.

**Decision: keep DSA reference in senders for V1.** The single-op path doesn't benefit
from this separation. `dsa_batch` uses `fill_*` functions which bypass senders anyway.
Revisit if we later want sender reuse across different DSA devices.

## Two-Tier Batch API

### Tier 1: Raw Batch (Maximum Performance)

User fills descriptors directly into the contiguous array:

```cpp
auto sndr = dsa_batch(dsa, N, [&](std::span<dsa_hw_desc> descs) {
    for (size_t i = 0; i < N; ++i)
        fill_data_move(descs[i], src + i*sz, dst + i*sz, sz);
});
```

- Zero memcpy — descriptors filled in-place
- No child operation states, no proxies, no task queue entries per sub-op
- Single MMIO doorbell write
- Single task queue entry for batch completion polling
- As close to raw hardware as possible within the sender model

This is the "test hardware limits" path (goal #2).

### Tier 2: Composed Batch (Convenience, Future)

Uses the descriptor sink concept to let existing senders fill descriptors:

```cpp
auto sndr = dsa_batch_all(dsa, N, [&](auto& collector, size_t i) {
    return dsa_data_move(collector, src + i*sz, dst + i*sz, sz);
});
```

- One memcpy per sub-op (from child's desc buffer to contiguous array)
- Reuses existing operation senders unchanged
- Page fault handling from existing senders' `notify()` logic

**V1 implements Tier 1 only.** Tier 2 can be added later using the descriptor sink
concept if the convenience is needed.

## Page Fault Handling

For batch operations, page fault retry is opcode-driven — a single function handles
all opcodes based on the descriptor's opcode field:

```cpp
void adjust_for_page_fault(dsa_hw_desc &desc, const dsa_completion_record &comp) {
    // Touch faulting page
    volatile char *t = (char *)comp.fault_addr;
    (comp.status & DSA_COMP_STATUS_WRITE) ? *t = *t : *t;

    // Generic adjustment
    desc.src_addr += comp.bytes_completed;
    desc.dst_addr += comp.bytes_completed;
    desc.xfer_size -= comp.bytes_completed;

    // Opcode-specific
    if (desc.opcode == DSA_OPCODE_CRCGEN || desc.opcode == DSA_OPCODE_COPY_CRC)
        desc.crc_seed = static_cast<uint32_t>(comp.crc_val);
}
```

`BatchOperation::notify()` checks batch completion. On partial failure, scans
sub-completion records, calls `adjust_for_page_fault` for faulted sub-ops,
re-submits them individually through the real DSA.

## Future Optimization Paths

### Zero-Copy / Pre-Created Descriptors

The `span<dsa_hw_desc>` factory interface is already zero-copy — descriptors are filled
in-place in the batch array. Future pre-allocation changes who owns the backing storage,
not the API shape.

### Raw Hardware Limit Testing

`dsa_batch` with the span factory IS the minimal path. Overhead beyond raw hardware:
one sender connect, one `start()` call, one task queue entry. This is the absolute
minimum the stdexec model requires.

## Files to Create/Modify (V1)

### New
- `include/dsa_stdexec/operations/batch.hpp` — `fill_*` functions, `BatchSender`,
  `BatchOperation`
- `benchmark/dsa/strategy_batch_raw.cpp` — BatchRaw scheduling pattern

### Modified
- `benchmark/dsa/config.hpp` — Add `BatchRaw` to `SchedulingPattern` enum
- `benchmark/dsa/config.cpp` — Name/parse/listing for `BatchRaw`
- `benchmark/dsa/strategies.hpp` — Declare run_batch_raw_inline, add to table
- `xmake.lua` — Add strategy_batch_raw.cpp to dsa_benchmark target
- `benchmark/benchmark_config.toml` — Add `batch_raw` to [scheduling]

### Cleanup
- Delete `examples/batch_raw.cpp` (superseded by batch sender)
- Remove from examples list in xmake.lua

## Open Questions

1. **Batch completion record sentinel check.** If the batch descriptor itself fails
   (e.g., invalid `desc_list_addr`), no sub-op completion records are written. Should
   `BatchOperation` check the batch completion record for batch-level errors?

2. **Max batch size handling.** Hardware limit is typically 32 (queried via
   `accfg_wq_get_max_batch_size`). If `N > max_batch`, should `dsa_batch` split
   internally or require the caller to cap? Caller capping is simpler for V1.

3. **Integration with NVIDIA stdexec GPU patterns.** How does NVIDIA integrate GPU
   as a computation resource in stdexec? Their approach to fixed-function accelerators
   may inform our DSA design. (Research in progress.)
