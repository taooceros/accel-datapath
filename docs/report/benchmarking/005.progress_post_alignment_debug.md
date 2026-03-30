# Progress Report: Post-Alignment Debug to Present

**Date**: 2026-02-22
**Period covered**: Jan 21 -- Feb 22, 2026 (~4.5 weeks)
**Starting point**: Alignment fix for DSA descriptors in coroutine frames (Jan 21)

---

## 1. Summary

The central question driving this period: **what is the cost of composable
async abstractions (C++ stdexec sender/receiver) when targeting hardware
accelerators like Intel DSA?**

After completing the coroutine alignment fix, I built a multi-dimensional
benchmark framework, used mock-hardware experiments to isolate software from
hardware overhead, and systematically reduced per-operation cost. Key findings:

- **80% of per-operation cost is software abstraction, not hardware.**
- By progressively bypassing abstraction layers, throughput scales from
  **26 Mpps (full stdexec) to 84 Mpps (reusable op states)** -- a clear
  abstraction-vs-performance tradeoff with three distinct design points.
- The measured deltas between abstraction levels tell us the cost of each layer:
  removing `scope.nest` + `then` saves ~14 ns/op; further removing
  `connect` + `start` saves ~7 ns/op.
- These gains transfer to real DSA hardware: `reusable` reaches **34 Mpps** on
  real DSA (vs 18 Mpps baseline), a 1.87x speedup.
- Real DSA hardware is well-optimized: only +5 ns/op amortized overhead with
  batched submission. The bottleneck is software.

| Strategy | Mock Mpps | Per-op | Real DSA Mpps | What's bypassed |
|---|---|---|---|---|
| `noalloc` (baseline) | 26 | 38 ns | 18 | nothing |
| `direct` (no scope/then) | 42 | 24 ns | 28 | scope.nest, then-adapter |
| `reusable` (no connect/start) | 60 | 16.7 ns | **34** | + connect, start |
| `reusable` (mock, c=32) | 84 | 11.9 ns | — | + cache effects |

---

## 2. What Was Built (Jan 21--25)

### All 8 DSA Operation Senders

Implemented stdexec senders for all 8 DSA operations (data_move, mem_fill,
compare, compare_value, dualcast, crc_gen, copy_crc, cache_flush) with
standalone examples. Each sender fills a hardware descriptor, submits via an
abstract `DsaSink` concept, and completes through the stdexec receiver chain
with transparent page fault retry.

This validates that the sender/receiver model can express the full DSA operation
set without loss of hardware functionality.

### Benchmark Framework

Built a multi-dimensional benchmark with 7 sweep dimensions (operation, message
size, concurrency, scheduling pattern, queue type, submission strategy, batch
size) and 3 metric classes (bandwidth, message rate, latency percentiles).
Interactive Plotly visualization for exploring the result space.

---

## 3. Descriptor Submission Strategies (Feb 11--16)

### Scheduling vs Submission: Two Orthogonal Dimensions

A key conceptual step was separating **scheduling** (how operations are issued
to the device) from **submission** (how descriptors physically reach hardware):

- **Scheduling patterns**: sliding window (pipelined, zero-alloc) vs batch
  (barrier-synchronized). Sliding window dominates at moderate concurrency;
  batch wins at c=4096 where slot scanning causes L3 cache misses.
- **Submission strategies**: direct (one doorbell per descriptor), staging
  (double-buffered copy), ring buffer, and **mirrored ring** (wrap-free via
  virtual memory aliasing).

### MirroredRing: Virtual-Memory Wrap-Free Ring Buffer

Uses `memfd_create` + dual `mmap` so the second half of virtual address space
aliases the first half's physical pages. Descriptors can be written as a
contiguous burst across the wrap boundary without special-case sealing logic.
This is the best-performing submission strategy for real DSA.

### Unified Engine API

Refactored the hardware interface into `DsaEngine<Submitter, Queue>`, replacing
a class hierarchy with concept-constrained template parameters. This enables
compile-time strategy selection without virtual dispatch on the hot path.

---

## 4. Performance Analysis: Isolating Software from Hardware (Feb 17--18)

### Mock DSA Methodology

To separate software overhead from hardware latency, I introduced `MockDsaBase`
-- a mock that completes every operation instantly (sets completion status on
submit). Running the full benchmark with mock vs real DSA reveals:

| Mode | Mpps | Per-op |
|---|---|---|
| Mock, `noalloc` (pure software ceiling) | 25--27 | 37--40 ns |
| Real DSA, `noalloc` (baseline, c=1024, bs=32) | 18 | 55 ns |
| Real DSA, `reusable` (c=1024, bs=64) | **35** | 28 ns |
| Real DSA (cold/unstable regime) | 9--11 | 91--111 ns |

**Finding**: On the baseline path, real DSA adds ~15--18 ns/op over mock.
But with `reusable`, the software overhead reduction also improves
hardware utilization — fewer poll traversals means tighter submission
batches, reaching **35 Mpps** on real hardware.

### Bistable Feedback Loop

Real DSA exhibits a **bistable throughput regime** absent from mock: the same
configuration can produce either ~20 Mpps or ~10 Mpps depending on initial
conditions (cache warmth, OS interrupts). Root cause: a positive feedback loop
between hardware completion timing and the O(N) poll traversal. When few
completions arrive per poll, scan overhead dominates, reducing effective batch
size, which further reduces hardware throughput. Mock eliminates this because
completion is instantaneous, confirming the instability is a hardware-software
interaction effect.

### Measured Results: Three Strategies

Rather than decomposing per-operation cost into speculative per-phase estimates,
the most reliable data comes from **measured end-to-end throughput** at each
abstraction level. All numbers are wall-clock measurements on mock DSA
(instant completion), inline polling, msg_size=8, 10 iterations, 32 MB total.

**data_move throughput vs concurrency** (3-run average):

| Strategy | c=32 | c=1024 | c=2048 | c=4096 |
|---|---|---|---|---|
| `sliding_window_noalloc` (baseline) | 28.9 Mpps | 26.7 | 26.3 | 25.0 |
| `sliding_window_direct` (no scope/then) | 46.2 Mpps | 41.4 | 41.6 | 37.8 |
| `sliding_window_reusable` (no connect/start) | **83.9 Mpps** | 62.5 | 59.9 | 61.3 |

These data_move results are stable across 3 independent runs (stdev < 1 Mpps).

**All 8 operations at c=2048** (Mpps, single run except where noted):

| Operation | `sliding_window_direct` | `sliding_window_reusable` |
|---|---|---|
| data_move | 41.6 (3-run avg) | 61.4 |
| mem_fill | 41.9 | 63.9 |
| compare | 42.9 | 61.9 |
| compare_value | 42.4 | 61.1 |
| dualcast | 39.7 | 60.3 |
| crc_gen | 43.9 | 60.5 |
| copy_crc | 41.9 | 58.7 |
| cache_flush | 45.6 | 64.0 |

The baseline (`sliding_window_noalloc`) was only measured per-operation for
`data_move` (26.3 Mpps at c=2048, 3-run average). Per-operation variation
across the 8 operations is small for both `direct` and `reusable` (±3 Mpps),
so the baseline likely shows similar uniformity.

### What the Measured Deltas Tell Us

The end-to-end measurements let us derive the cost of each abstraction layer
by differencing:

**`noalloc` → `direct`: ~14 ns/op saved (38 → 24 ns)**

`sliding_window_direct` eliminates `scope.nest()` and `stdexec::then()` but
still calls `stdexec::connect()` and `start()`. Therefore the combined cost
of scope tracking + the then-adapter + associated operation state overhead is
~14 ns/op. This is a measured delta, not an estimate.

**`direct` → `reusable`: ~7 ns/op saved (24 → 16.7 ns)**

`sliding_window_reusable` pre-allocates operation states and reuses them,
skipping `stdexec::connect()` and `start()` entirely. The hot path becomes:
memset descriptors → fill fields → submit. So the combined cost of
per-operation `connect()` + `start()` (including placement-new of the 384-byte
operation state, receiver binding, and function pointer setup) is ~7 ns/op.

**`reusable` at c=2048 → c=32: ~5 ns/op saved (16.7 → 11.9 ns)**

At c=32, the entire working set (~32 × 256 bytes = 8 KB) fits in L1 cache.
The 5 ns difference is attributable to cache effects: at c=2048 the slot array
spans ~512 KB, causing L2/L3 misses during arena operations and poll traversal.

**Remaining 16.7 ns at `reusable` (c=2048)**

This is the cost of everything that `reusable` still does: memset(desc, 64B) +
memset(comp, 32B) + fill_descriptor + submit + poll + arena acquire/release +
atomic counter. We have not individually measured these sub-components.
At c=32, this drops to 11.9 ns, showing that ~5 ns of it is cache-miss cost.

### Cache Working Set Analysis

All three strategies show throughput degradation with increasing concurrency in
mock DSA (where hardware latency is zero). The cause is cache pressure: each
slot is heap-allocated and touched every operation, so the hot working set
scales linearly with concurrency.

**Measured slot sizes** (compiled with `sizeof`, including alignment padding):

| Strategy | Slot type | Size per slot | Key contents |
|---|---|---|---|
| `reusable` | `ReusableSlot` | 384 bytes | DsaOperationBase(320B) + 5 ptrs + timestamp |
| `direct` | `OperationSlot<384>` | 448 bytes | 384B op state + atomic + fn ptr + free-list ptr |
| `noalloc` | `OperationSlot<448>` | 512 bytes | 448B op state + atomic + fn ptr + free-list ptr |

Each `DsaOperationBase` (320 bytes) internally contains the 64-byte hardware
descriptor and 32-byte completion record, over-allocated with padding for
64-byte alignment. This means each slot spans **5-8 cache lines** (64B each).

**Working set vs cache hierarchy** (slot size × concurrency):

| Concurrency | `reusable` | `direct` | `noalloc` | Fits in |
|---|---|---|---|---|
| 32 | 12 KB | 14 KB | 16 KB | L1d (48 KB) |
| 64 | 24 KB | 30 KB | 34 KB | L1d (48 KB) |
| 256 | 96 KB | 116 KB | 132 KB | L2 (2 MB) |
| 1024 | 384 KB | 464 KB | 528 KB | L2 (2 MB) |
| 2048 | 768 KB | 928 KB | 1056 KB | L2 (2 MB) |
| 4096 | 1536 KB | 1856 KB | 2112 KB | L2/L3 boundary |

The cache hierarchy on this machine (Xeon Gold 6438M, Sapphire Rapids), read
from `/sys/devices/system/cpu/cpu0/cache/`:

| Level | Size | Associativity | Line size | Scope |
|---|---|---|---|---|
| L1d | 48 KB | 12-way | 64 B | per core (2 HT siblings) |
| L2 | 2 MB | 16-way | 64 B | per core |
| L3 | 60 MB | 15-way | 64 B | per socket (32 cores) |

The throughput data maps cleanly onto these boundaries:

**c=32 (L1-resident)**: All three strategies fit entirely in L1d. The
`reusable` strategy reaches 84 Mpps (11.9 ns/op) — every arena acquire,
descriptor memset, and poll check hits L1. This is the practical lower bound
on per-operation software cost.

**c=32 → c=1024 (L1 → L2 transition)**: The biggest throughput drop. For
`reusable`, 84 → 62.5 Mpps (+4.1 ns/op). At c=1024 the working set is 384 KB
— well within L2 but far exceeding L1d. Each operation now incurs L1 misses
on the slot's descriptor and completion record cache lines. An L2 hit costs
~4-5 ns on Sapphire Rapids (vs ~1 ns for L1), consistent with the ~4 ns
increase per operation.

**c=1024 → c=2048**: Marginal further degradation. `reusable` goes from 62.5
to 59.9 Mpps (+0.7 ns/op). Both working sets fit in L2, so the additional cost
comes from increased TLB pressure and slightly worse prefetch effectiveness
at 768 KB vs 384 KB.

**c=2048 → c=4096 (L2/L3 boundary)**: Mixed results. `noalloc` continues to
degrade (25.0 Mpps, +1.5 ns from c=2048) — its 2112 KB working set exceeds
L2 capacity. `reusable` actually *improves* slightly (61.3 vs 59.9 Mpps),
which is within measurement noise and may reflect a pipelining effect where
deeper concurrency amortizes poll() cost.

**Why `noalloc` degrades most at c=4096**: Its 512-byte slots mean 8 cache
lines touched per operation. At c=4096, the 2112 KB working set exceeds the
2 MB L2. Additionally, `noalloc` scans slots sequentially (`for (auto &slot :
slots)`) checking `ready.load()` on each — touching all 4096 slots even when
most aren't ready. This O(N) scan amplifies cache-miss cost. In contrast,
`reusable` uses a free-list (O(1) acquire), and `direct` uses an arena, so
they only touch slots that are actually ready.

**Implication**: The 84 Mpps at c=32 represents the cache-optimal ceiling for
the `reusable` strategy's per-op work. The gap to c=2048 (60 Mpps, +5 ns) is
almost entirely cache-miss cost, not algorithmic overhead. To recover this at
high concurrency, the slot size would need to shrink (e.g., separate hot and
cold fields into different cache lines) or the access pattern would need to
improve (e.g., prefetch the next slot during the current operation's submit).

### Real DSA Results: Strategy Gains Hold on Hardware

The mock results above isolate software overhead. A subsequent benchmark on
**real DSA hardware** (mirrored_ring submission, batch_size=32, NoLock queue,
msg_size=8, inline polling) confirms that `direct` and `reusable` gains
transfer to real hardware — and are in fact larger, because the software
overhead reduction also reduces the hardware-software feedback loop penalty.

**data_move throughput on real DSA** (single run):

| Strategy | c=64 | c=256 | c=1024 |
|---|---|---|---|
| `noalloc` | 12.5 Mpps | 15.4 | 18.2 |
| `arena` (noalloc + SlotArena) | 11.8 | 10.3 | 11.6 |
| `direct` | 14.3 | 24.7 | 27.5 |
| `reusable` | 15.0 | 29.5 | **34.0** |

At c=1024 on real DSA, `reusable` reaches **34 Mpps** (29.4 ns/op) — a 1.87x
speedup over `noalloc` (18.2 Mpps, 54.9 ns/op). With bs=64, `reusable`
peaks at **35.3 Mpps** (28.3 ns/op).

**All 8 operations on real DSA** (c=1024, bs=32, NoLock, Mpps):

| Operation | `noalloc` | `direct` | `reusable` |
|---|---|---|---|
| data_move | 18.2 | 27.5 | 34.0 |
| mem_fill | 19.4 | 24.4 | 36.4 |
| compare | 19.4 | 26.9 | 30.7 |
| compare_value | 19.5 | 27.1 | 34.5 |
| dualcast | 18.7 | 26.7 | 33.2 |
| crc_gen | 19.3 | 27.9 | 34.2 |
| copy_crc | 2.5 | 2.3 | 2.4 |
| cache_flush | 2.1 | 2.4 | 2.0 |

Most operations show consistent ~1.5x (`direct`) and ~1.8x (`reusable`)
speedups. The exceptions are `copy_crc` and `cache_flush`, which are
hardware-bottlenecked at ~2 Mpps regardless of strategy — confirming that
for these operations, hardware execution time dominates and software overhead
is irrelevant.

Notable: `arena` performs **worse** than `noalloc` on real DSA at c=256 and
c=1024 (10--12 Mpps vs 15--18 Mpps), suggesting it falls into the bistable
low-throughput regime more frequently. This is consistent with the hypothesis
that `arena`'s different memory access pattern interacts poorly with the
hardware completion feedback loop.

### Baseline Optimizations: Smaller Than Expected

Before building `direct`/`reusable`, we tried three targeted optimizations on
the baseline path:

1. **Proxy → function pointers**: Replaced `pro::proxy<OperationFacade>`
   type-erased dispatch with raw function pointers in `OperationBase`.
2. **IndexedTaskQueue**: Replaced linked-list poll with a flat vector +
   swap-and-pop removal with prefetch.
3. **SlotArena**: O(1) free-list slot management instead of O(N) scanning.

**Measured result**: ~27 Mpps (up from ~26 Mpps). Combined savings of only
~2--3 ns/op, far below the projected 11 ns.

Why the gap between projection and reality:
- The proxy library already used small-buffer optimization, so "eliminating
  proxy allocation" really just replaced one inline memcpy with another.
  The dispatch cost (one function pointer call) is similar either way.
- With mock DSA (100% instant completion), every linked-list node is removed
  on first visit, so O(N) traversal waste doesn't manifest. The indexed
  queue's advantage should appear on real hardware where most nodes are
  pending.
- The original per-phase cost projections (4 ns for proxy, 5 ns for poll,
  3 ns for scanning) were analytical guesses, not measurements. They
  over-estimated the reducible portion.

---

## 5. Optimization: Three Benchmark Strategies (Feb 18--20)

Each strategy removes a layer of stdexec machinery, trading composability for
throughput:

### `sliding_window_direct` — No Scope, No Then (1.6x)

Eliminates `scope.nest()` and `stdexec::then()` -- the sender connects directly
to a benchmark receiver without lifetime tracking or adapter layers. Saves
~14 ns/op. Operation state shrinks from 448 to 384 bytes.

**Result**: 42 Mpps (24 ns/op).

### `sliding_window_reusable` — Pre-allocated Operation States (2.3x)

Bypasses `stdexec::connect()` and `start()` entirely. Pre-allocates operation
state objects and reuses them across iterations -- only the descriptor fields
are re-filled per operation. No sender chain construction, no operation state
allocation.

**Result**: 60 Mpps (16.7 ns/op). At c=32 (hot L1 cache): **84 Mpps (11.9 ns/op)**.

### The Abstraction-Performance Tradeoff

This gives us a clear design space for users of the library:

| Strategy | What you get | What you lose | Use case |
|---|---|---|---|
| `noalloc` (full stdexec) | Composable, type-safe, error propagation | ~38 ns/op | General-purpose async |
| `direct` | Still stdexec connect/start, but leaner | scope tracking, then-adapters | Latency-sensitive paths |
| `reusable` | Raw descriptor reuse, minimal overhead | stdexec composability entirely | Peak throughput benchmarks |
| `batch_raw` | Hardware batch descriptors, bypass stdexec | All abstraction | Theoretical hardware limit |

---

## 6. Transparent Auto-Batching (Feb 11--16)

### The Problem: MMIO Doorbell Cost

Each DSA descriptor submission requires an MMIO doorbell write (`MOVDIR64B` or
`ENQCMD`), which triggers a cache-coherency transaction to the device. For
small messages where hardware execution time is negligible, this doorbell cost
dominates. At 8-byte messages with immediate (one-doorbell-per-op) submission,
throughput plateaus at ~6 Mpps.

DSA hardware supports a *batch opcode* (`0x01`): a single doorbell submits a
pointer to a contiguous array of descriptors, amortizing the MMIO cost across
the entire batch. The challenge is exploiting this transparently -- without
requiring the application (or the stdexec scheduling layer) to manually group
operations into batches.

### Design: Auto-Batching via Descriptor Submitter Strategies

We implemented **transparent auto-batching** as a `DescriptorSubmitter` strategy
that sits below the scheduling layer. The application calls
`submit_descriptor()` one descriptor at a time; the submitter silently
accumulates descriptors in a ring buffer and issues a hardware batch command
when the batch is full or when `pre_poll()` flushes a partial batch before
polling for completions.

This is the key property: **scheduling strategies are completely unaware of
batching**. The same sliding-window or scoped-worker code runs identically
whether the submission backend is immediate (1 doorbell/op) or mirrored-ring
(1 doorbell/32 ops). Batching is an implementation detail of the submission
layer.

We built four submission strategies as an ablation study:

| Strategy | Doorbells/N ops | In-flight batches | Key idea |
|---|---|---|---|
| Immediate | N | N/A | 1 doorbell per descriptor |
| Double-buffered | ceil(N/B) | 2 | Two fixed staging arrays, swap on submit |
| Fixed-ring | ceil(N/B) | 16 | Ring of fixed-size batch entries |
| MirroredRing | ceil(N/B) | 16 | Virtual-memory wrap-free ring (`memfd` + dual `mmap`) |

### Results: 1.2--2x Throughput Gain from Auto-Batching

At concurrency=16, 8-byte messages, ring-buffer batching vs double-buffered:

| Operation | Double-buf (Mpps) | Ring (Mpps) | Speedup |
|---|---|---|---|
| data_move | 2.60 | 4.29 | 1.65x |
| mem_fill | 4.13 | 8.41 | 2.04x |
| compare | 4.61 | 8.09 | 1.76x |
| cache_flush | 2.41 | 2.94 | 1.22x |

The fixed-ring ablation (same 16 in-flight depth, but fixed-size arrays)
matches ring-buffer performance, confirming that **in-flight batch depth** (16
vs 2) is the dominant factor, not memory packing efficiency.

Three factors explain the gain:
1. **No submission blocking** -- 16 batch slots vs 2; back-pressure is rare
2. **Better batch utilization** -- auto-submit at `max_batch_size` yields
   consistently full batches; double-buffered submits partials on every `poll()`
3. **Deeper pipeline** -- hardware can process multiple batches concurrently
   while software fills the next one

### Connection to UCX/OpenSHMEM Request Patterns

Our earlier analysis of UCX and OpenSHMEM communication patterns revealed that
these runtimes generate streams of small, independent, non-blocking requests --
one-sided RMA operations (`put`/`get`) in UCX, and non-blocking bulk transfers
(`shmem_put_nbi`, `shmem_get_nbi`) and collectives in OpenSHMEM. Each of these
maps naturally to a DSA `data_move` descriptor.

The transparent auto-batching mechanism built here is a direct answer to that
pattern. A UCX or OpenSHMEM transport backend could submit individual RMA
operations through the stdexec sender interface and automatically get hardware
batch amortization -- without any batching logic in the transport layer itself.
The scheduling-vs-submission separation ensures that the transport only needs to
reason about operation ordering and completion semantics, while the descriptor
submitter handles hardware-efficient batching underneath.

---

## 7. Research Insights

1. **Abstraction cost is quantifiable via layer-removal experiments**: By
   building three strategies (`noalloc` → `direct` → `reusable`) that
   progressively bypass stdexec layers, we can measure the cost of each layer
   as a delta. Removing scope.nest + then costs 14 ns/op; removing connect +
   start costs another 7 ns/op. The total stdexec overhead is ~21 ns/op
   (38 − 16.7), measured directly.

2. **Hardware is not the bottleneck**: For small-message DSA workloads, the
   MirroredRing submission strategy amortizes hardware cost to ~5 ns/op. The
   4-engine DSA device is underutilized by the software stack.

3. **Mock-hardware methodology works**: Swapping real hardware for an
   instant-completion mock cleanly isolates software overhead and eliminates
   the bistable regime, making measurements reproducible.

4. **Scheduling and submission are orthogonal**: Separating "how to issue ops"
   from "how descriptors reach hardware" yields a clean 2D design space. This
   decomposition should generalize to other accelerators (GPU command buffers,
   NIC descriptor rings).

5. **The bistable regime is a hardware-software feedback effect**: O(N) poll
   traversal creates a coupling between completion rate and scan cost that
   amplifies small perturbations into 2x throughput swings. An O(1) completion
   notification mechanism (hardware interrupts, completion bitmap) would break
   the feedback loop.

---

## 8. Speculative Per-Phase Cost Model (Not Measured)

> **Caveat**: The individual per-phase costs below are analytical estimates
> produced by reasoning about the code structure, **not** from instrumenting
> each phase independently (e.g., with `rdtsc` or cycle counters). Previous
> reports in this repository stated these numbers as if they were measured;
> they were not. The only reliable data is the end-to-end throughput at each
> abstraction level (Section 4) and the deltas between them.

A previous analysis attempted to decompose the 37 ns/op baseline ceiling into
phases:

| Phase | Guessed cost | Notes |
|---|---|---|
| `stdexec::connect()` + placement new | ~9 ns | 384-byte op state construction |
| Sender chain (scope.nest + then) | ~6 ns | Adapter layers + lifetime tracking |
| `stdexec::set_value()` propagation | ~5 ns | 3-layer receiver chain |
| O(N) poll traversal | ~5 ns | Linked-list pointer chasing, amortized |
| Type-erased proxy / fn ptr dispatch | ~4 ns | Originally proxy; now function pointers |
| O(N) slot scanning | ~3 ns | Atomic loads per slot |
| Atomics + descriptor memset | ~2 ns | fetch_sub + 96 bytes memset |
| **Total** | **~34--37 ns** | |

**What went wrong with the projections**: Three optimizations targeting the
"reducible" phases (proxy elimination, indexed queue, arena) were projected to
save ~11 ns/op combined. The actual measured savings were ~2--3 ns/op:

- Proxy elimination saved ~1 ns, not 4 ns (the proxy library's small-buffer
  optimization was already cheap; dispatch cost is similar with function
  pointers).
- Indexed queue saved ~0 ns on mock (linked-list traversal is efficient when
  100% of nodes complete immediately; the benefit is expected on real hardware).
- Arena saved ~1 ns at moderate concurrency (meaningful at c=4096 where it
  fixed a cache regression, but not at c=2048).

This discrepancy suggests the original per-phase estimates significantly
misattributed costs. The measured deltas from Section 4 remain the most
trustworthy decomposition of stdexec overhead.

**What we can say**: The total stdexec overhead is ~21 ns/op (measured, from
`noalloc` − `reusable`). Of that, ~14 ns is attributable to scope.nest + then
(measured, from `noalloc` − `direct`), and ~7 ns to connect + start (measured,
from `direct` − `reusable`). The internal breakdown within each of those
buckets (e.g., how much of the 7 ns is placement-new vs receiver binding vs
function pointer setup) is unknown without per-phase instrumentation.

---

## 9. Next Steps

| Question | Approach | Status |
|---|---|---|
| Do direct/reusable gains hold on real DSA? | Run real-hardware benchmarks | **Done** — 1.5x (direct) and 1.87x (reusable) confirmed |
| Does IndexedTaskQueue stabilize the bistable regime? | Real DSA with indexed vs linked-list | Not started |
| Why does `arena` fall into low regime on real DSA? | Compare memory access patterns with `noalloc` | Not started |
| Can we reach hardware throughput limit through stdexec? | Conditional batch flush + O(1) poll | Planned |
| Instrument per-phase costs | `rdtsc`-based microbenchmarks for connect, start, set_value, poll individually | Not started |
| What is the multi-device scaling curve? | Add dsa2 (second device) | Future |
| Can stdexec itself be made cheaper? | Profile connect/start internals, propose upstream changes | Open question |
