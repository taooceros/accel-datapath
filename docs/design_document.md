# Design Document: stdexec + Intel DSA Framework

**Author**: Hongtao Zhang
**Last updated**: 2026-03-03
**Status**: Active development

---

## 1. Overview

This project provides C++ sender/receiver (P2300/stdexec) bindings for Intel Data Streaming Accelerator (DSA). The primary goal is **maximizing message rate** (ops/sec) for small transfers using inline polling. The framework serves dual purposes: (1) a practical high-performance DSA programming model and (2) a research vehicle for studying the cost of composable async abstractions in the nanosecond regime.

### Key Results

| Metric | Value |
|---|---|
| Full stdexec baseline (mock DSA, c=2048) | 26 Mpps, 38 ns/op |
| Reusable strategy (mock DSA, c=32, L1-resident) | **84 Mpps, 11.9 ns/op** |
| Reusable strategy (real DSA, c=1024, bs=32) | **34 Mpps** (1.87x over baseline) |
| Measured stdexec overhead | 21 ns/op (55% of total) |
| Hardware cost (amortized, batch=32) | ~5 ns/op (13% of total) |

---

## 2. System Architecture

The system is organized in three layers. Each layer has a single responsibility and communicates through well-defined concept-based interfaces.

```
┌─────────────────────────────────────────────────────────────┐
│  Application / Benchmark / UCX Transport                    │
│  Scheduling policy: sliding window, batch, scoped workers   │
│  co_await dsa_data_move(src, dst, 8)                        │
├─────────────────────────────────────────────────────────────┤
│  stdexec Integration Layer  (include/dsa_stdexec/)          │
│  PollingRunLoop · operation senders · DsaScheduler          │
├─────────────────────────────────────────────────────────────┤
│  Submission Backend  (src/dsa/)                             │
│  DsaEngine<Submitter, Queue>                                │
│  immediate / ring-buffer / mirrored-ring                    │
├─────────────────────────────────────────────────────────────┤
│  Intel DSA Hardware                                         │
│  Async DMA engine — 4 engines on Sapphire Rapids            │
└─────────────────────────────────────────────────────────────┘
```

### Source Layout

| Directory | Purpose |
|---|---|
| `src/dsa/` | Low-level DSA: DsaEngine, task queues, descriptor submitters, alignment |
| `include/dsa_stdexec/` | stdexec integration: PollingRunLoop, operation senders, schedulers |
| `include/dsa_stdexec/operations/` | Per-operation sender implementations (8 DSA ops) |
| `benchmark/dsa/` | Multi-dimensional benchmark framework with TOML config |
| `benchmark/dsa/strategies/` | Benchmark strategy implementations (3 families, 9+ variants) |
| `examples/` | Per-operation standalone examples |
| `test/` | Unit and integration tests |
| `tools/` | `dsa_launcher` capability helper (CAP_SYS_RAWIO) |
| `dsa-config/` | accel-config device configurations |

---

## 3. Hardware Abstraction Layer (`src/dsa/`)

### 3.1 DsaEngine

`DsaEngine<Submitter, QueueTemplate>` is the central hardware interface. It is parameterized by two concepts:

- **DescriptorSubmitter**: How descriptors physically reach the hardware (immediate MMIO, ring-buffered batch, mirrored-ring batch)
- **TaskQueue**: How in-flight operations are tracked for completion polling

Both are concept-constrained template parameters — no virtual dispatch on the hot path. The engine handles device discovery, descriptor submission, completion polling, and WQ management.

**Type aliases for common configurations:**

| Alias | Queue | Use Case |
|---|---|---|
| `DsaSingleThread` | NoLock | Best for single-thread inline polling |
| `DsaIndexed` | Per-slot indexed | Experimental |
| `Dsa` | Mutex | General purpose |
| `DsaTasSpinlock` | TAS spinlock | Low-contention multi-thread |
| `DsaSpinlock` | Ticket spinlock | Fair multi-thread |
| `DsaLockFree` | Lock-free list | Wait-free (worst throughput due to 3x traversal) |

### 3.2 Descriptor Submitters

Five implementations forming an ablation study:

| Submitter | Mechanism | Doorbells per N ops | Peak Mpps |
|---|---|---|---|
| `DirectSubmitter` | 1 MMIO per descriptor | N | ~6 |
| `DoubleBufSubmitter` | 2 staging arrays, swap on submit | ceil(N/B) | ~4 |
| `FixedRingSubmitter` | Ring of fixed-size batch entries | ceil(N/B) | ~18 |
| `RingSubmitter` | Contiguous ring buffer | ceil(N/B) | ~18 |
| `MirroredRingSubmitter` | `memfd_create` + dual `mmap` wrap-free ring | ceil(N/B) | **~35** |

The MirroredRing uses virtual-memory aliasing: the second half of the virtual address space maps to the same physical pages as the first half. Descriptors that cross the ring boundary are contiguous in virtual memory without modular arithmetic or early batch sealing. This is the same technique used by io_uring.

**Key finding**: An ablation study showed that **in-flight batch depth** (16 vs. 2 concurrent batch slots) is the dominant factor — not ring buffer cleverness. Deep submission queues matter more than clever memory management.

### 3.3 Task Queues

Seven implementations satisfying the `TaskQueue` concept:

- **NoLockTaskQueue**: No synchronization; single-thread only. Best performance.
- **LockedTaskQueue** (Mutex, TAS, Ticket, Backoff): Various lock strategies.
- **LockFreeTaskQueue**: Wait-free but consistently worst throughput (3 list traversals per poll).
- **IndexedTaskQueue**: Flat vector with swap-and-pop; experimental.

### 3.4 DsaOperationBase

Runtime over-alignment for hardware descriptors:
- 64-byte alignment for the hardware descriptor
- 32-byte alignment for the completion record
- Over-allocates at construction time (not `alignas`) because `alignas` on coroutine frame members is not reliably honored by compilers
- Total size: 320 bytes per `DsaOperationBase`

### 3.5 WQ Backpressure

Dedicated work queues accept `_movdir64b` unconditionally — there is no hardware feedback when the WQ is full. Without software backpressure, submissions silently drop. `DsaEngine::submit()` spins on `poll()` when in-flight descriptors reach WQ depth. For batch submitters (which manage their own depth), the check is optimized away.

---

## 4. stdexec Integration Layer (`include/dsa_stdexec/`)

### 4.1 PollingRunLoop

The primary execution model. A single calling thread drives both submission and completion in a tight loop:

1. Check for pending work
2. Submit descriptors (via DescriptorSubmitter)
3. Poll completion records
4. Fire receiver callbacks for completed operations

No cross-thread coordination, no lock contention, no context switches. This mirrors eRPC and DPDK poll-mode drivers.

### 4.2 Operation Senders

Eight type-safe stdexec senders, one per DSA operation:

| Sender | DSA Opcode | Return Type |
|---|---|---|
| `dsa_data_move` | 0x03 (memmove) | void |
| `dsa_mem_fill` | 0x04 | void |
| `dsa_compare` | 0x05 | comparison result |
| `dsa_compare_value` | 0x06 | match result |
| `dsa_dualcast` | 0x07 | void |
| `dsa_crc_gen` | 0x09 | CRC value |
| `dsa_copy_crc` | 0x0C | CRC value |
| `dsa_cache_flush` | 0x14 | void |

Each sender follows the CRTP pattern via `DsaOperationMixin`:
1. Fill hardware descriptor fields
2. Submit via `DsaEngine::submit()`
3. Poll completion record via `TaskQueue`
4. Propagate result through stdexec receiver chain
5. Transparent page-fault retry (`DSA_COMP_PAGE_FAULT_NOBOF`)

### 4.3 OperationBase

Originally used `pro::proxy<OperationFacade>` for type erasure in the intrusive completion list. Replaced with raw function pointers after profiling showed proxy was already cheap (SBO path), saving ~1 ns/op.

### 4.4 DsaScheduler and Threaded Mode

`DsaScheduler` provides a stdexec scheduler for threaded mode (dedicated poller thread). Less performant than inline polling but useful for integration with existing event loops.

### 4.5 dsa_batch

Hardware batch sender that submits a batch descriptor pointing to a contiguous array of up to 32 regular descriptors. Used by the `batch_raw` strategy.

---

## 5. Benchmark Framework (`benchmark/dsa/`)

### 5.1 Design

Multi-dimensional sweep across 7 parameters:

| Dimension | Values |
|---|---|
| Operation | 8 DSA operations |
| Message size | 8 B to 1 MB |
| Concurrency | 1 to 4096 in-flight ops |
| Scheduling pattern | sliding_window, batch, scoped_workers |
| Queue type | NoLock, Mutex, TAS, Ticket, Backoff, LockFree |
| Submission strategy | direct, ring, mirrored_ring |
| Batch size | 1, 8, 16, 32, 64, 128 |

TOML-based configuration with CLI override. CSV output. Interactive Plotly visualization via `benchmark/visualize_interactive.py`.

### 5.2 Strategy Taxonomy

Three families of scheduling strategies:

**Sliding Window** (highest throughput): Keeps C ops in-flight at all times. As soon as one completes, submit the next immediately. Five progressive variants strip stdexec layers:

| Variant | What's Removed | Mock ns/op | Mock Mpps |
|---|---|---|---|
| `sliding_window` | Nothing (full stdexec + heap alloc) | ~35 | ~28 |
| `noalloc` | Heap allocation (placement-new) | ~35 | ~28 |
| `arena` | O(N) slot scan (O(1) free-list) | ~35 | ~28 |
| `direct` | `scope.nest()` + `then()` | ~24 | ~42 |
| `reusable` | `connect()` + `start()` | ~12-17 | **60-84** |

**Batch** (barrier-synchronized): Submit all C ops, wait at barrier, repeat. Three variants: `heap_alloc`, `noalloc`, `raw` (hardware batch descriptor).

**Scoped Workers** (coroutine-based): N persistent coroutines, each `co_await`s its own op sequentially. Structured concurrency — coroutines cancel cleanly on scope exit.

---

## 6. Key Design Decisions

### 6.1 Concept-Based Extensibility (Not Inheritance)

Both `TaskQueue` and `DescriptorSubmitter` are C++20 concepts. New implementations are added by satisfying concept constraints — no virtual base class. The hot path is monomorphic, enabling full inlining and zero-cost abstraction at the hardware interface level.

### 6.2 Scheduling/Submission Orthogonality

The most important architectural decision: **scheduling strategies are completely unaware of submission strategy**. The same sliding-window code runs identically whether the backend is immediate (1 doorbell/op, ~6 Mpps) or MirroredRing (1 doorbell/32 ops, ~35 Mpps). Batching is an implementation detail of the submission layer. Analogy: TCP Nagle — the application writes individual bytes; TCP accumulates them into packets.

### 6.3 PollingRunLoop as Primary Execution Model

Single-thread inline polling eliminates all cross-thread coordination. This is critical for maximizing message rate on small transfers where per-operation overhead must be minimized.

### 6.4 Runtime Over-Alignment

`DsaOperationBase` over-allocates for 64-byte descriptor and 32-byte completion record alignment. Required because `alignas()` on coroutine frame members is not reliably honored by compilers.

### 6.5 Type Erasure via Function Pointers

Replaced `pro::proxy<OperationFacade>` with raw function pointers. Enables heterogeneous operation types in intrusive linked lists. The hot poll path stays free of indirect calls.

### 6.6 WQ Backpressure

Software-enforced backpressure prevents silent descriptor drops on dedicated work queues. `DsaEngine::submit()` spins on `poll()` when in-flight count reaches WQ depth.

### 6.7 Transparent Page Fault Retry

`DSA_COMP_PAGE_FAULT_NOBOF` triggers automatic page touch + re-submit with byte offset adjustment. Callers never see page faults.

---

## 7. Performance Characterization

### 7.1 The Batching Regime Change

Without batching, each MMIO doorbell costs ~160 ns — framework overhead (21 ns) is only 13% of total and barely matters. With batching (batch=32), MMIO amortizes to ~5 ns/op, making framework overhead **4x larger than hardware cost**.

| Component | Cost (ns/op) | Fraction |
|---|---|---|
| Framework overhead (scope, then, connect, start) | 21 | 55% |
| Software bookkeeping (descriptor fill, poll, complete) | 12 | 32% |
| Hardware (amortized, batch=32) | ~5 | 13% |
| **Total** | **38** | **100%** |

### 7.2 Layer-Removal Results

Measured end-to-end deltas, not analytical estimates:

```
noalloc (full stdexec) → direct (no scope/then) → reusable (no connect/start)
      38 ns/op                 24 ns/op                  16.7 ns/op
          |______ 14 ns ______|      |_______ 7 ns ________|
          scope.nest + then          connect + start
```

- **scope.nest() + then()**: 14 ns/op (37% of baseline)
- **connect() + start()**: 7 ns/op (18% of baseline)
- **Total stdexec overhead**: 21 ns/op (55% of baseline)

### 7.3 Cache Hierarchy Effects

Per-operation metadata size determines cache residency and throughput:

| Concurrency | Working Set (reusable) | Cache Level | Throughput | Per-op |
|---|---|---|---|---|
| 32 | 12 KB | L1d (48 KB) | 84 Mpps | 11.9 ns |
| 64 | 24 KB | L1d | ~75 Mpps | ~13.3 ns |
| 1024 | 384 KB | L2 (2 MB) | 62.5 Mpps | 16.0 ns |
| 2048 | 768 KB | L2 | 59.9 Mpps | 16.7 ns |
| 4096 | 1536 KB | L2/L3 boundary | 61.3 Mpps | 16.3 ns |

The L1→L2 transition adds ~4 ns/op, consistent with Sapphire Rapids L2 hit latency (~4-5 ns).

**Slot sizes**: reusable=384B, direct=448B, noalloc=512B. Each slot spans 5-8 cache lines.

### 7.4 Bistable Throughput Regime

Real DSA exhibits a bistable regime absent from mock: the same configuration produces either ~20 Mpps or ~10 Mpps. Root cause is a positive feedback loop:

```
fewer completions/poll → more wasted O(N) scan → longer submission gap
→ smaller effective batch → fewer completions (repeat)
```

An O(1) completion mechanism (completion bitmaps, hardware interrupts) would break this loop.

### 7.5 Real DSA Hardware Results

Real DSA at c=1024, bs=32, NoLock queue, inline polling:

| Strategy | Mpps | Speedup |
|---|---|---|
| `noalloc` (baseline) | 18.2 | 1.0x |
| `direct` | 27.5 | 1.51x |
| `reusable` | **34.0** | **1.87x** |

Gains are *larger* on real hardware because reducing software overhead also improves hardware utilization — fewer poll traversals mean tighter batches, keeping DSA engines busier.

Hardware-bottlenecked operations (`copy_crc`, `cache_flush`) show ~2 Mpps regardless of strategy — confirming that software optimization only matters when hardware is not the bottleneck.

---

## 8. Methodological Contributions

### 8.1 Layer-Removal Methodology

Rather than timing individual functions with `rdtsc` (which misses microarchitectural interaction effects), we build progressively-stripped benchmark variants. The throughput delta between adjacent variants measures the end-to-end cost of the removed layer.

**Validation**: Three targeted optimizations predicted (analytically) to save 11 ns/op actually saved only 2-3 ns/op. The 4x over-prediction demonstrates that profiling-based cost attribution fails for tightly-coupled nanosecond-scale pipelines.

### 8.2 Mock Hardware Methodology

`MockDsaBase` completes every operation instantaneously. Running identical benchmarks on mock vs. real DSA cleanly separates software cost from hardware cost. The mock eliminates the bistable regime, producing clean, reproducible measurements.

---

## 9. Structural Isomorphism with Network I/O

The DSA framework's architecture maps directly to network I/O systems:

| DSA Framework | Network / I/O Analog |
|---|---|
| `DescriptorSubmitter` (ring buffer of HW descriptors) | NIC TX descriptor ring |
| `TaskQueue` (completion tracking) | NIC RX completion queue |
| `MirroredRingSubmitter` (memfd + dual mmap) | io_uring submission queue |
| `pre_poll()` flush before completion check | `io_uring_submit()` before `io_uring_wait_cqe()` |
| WQ backpressure (spin on full queue) | TCP `EAGAIN` / NIC TX ring full |

This isomorphism means findings about submission overhead, batching, and completion polling transfer directly to NIC and io_uring programming.

---

## 10. Build System and Dependencies

- **Build**: xmake (C++23, GCC 15, mold linker)
- **Required flags**: `-menqcmd` and `-mmovdir64b` for DSA intrinsics
- **Environment**: Nix flake (`devenv shell`)
- **Dependencies**: stdexec, libaccel-config, fmt, proxy, tomlplusplus
- **Hardware**: Intel Xeon Gold 6438M (Sapphire Rapids), 4 DSA engines, 48 KB L1d, 2 MB L2

Build commands:
```bash
devenv shell                           # Enter Nix development shell
xmake                                  # Build all targets
xmake f -m release && xmake            # Release mode
run                                    # Run benchmarks (auto dsa_launcher + build mode)
dsa_launcher ./build/.../binary        # Run with CAP_SYS_RAWIO
```

---

## 11. Open Questions and Future Directions

1. **Can stdexec itself be made cheaper?** The 21 ns overhead is not fundamental — `reusable` proves the model works at 12 ns. Can upstream stdexec adopt operation pooling?
2. **Does IndexedTaskQueue stabilize the bistable regime?** O(1) completion tracking may break the feedback loop on real hardware.
3. **Multi-device scaling**: What is the throughput curve with 2+ DSA devices?
4. **Per-phase instrumentation**: The internal breakdown within the 14 ns (scope.nest + then) and 7 ns (connect + start) buckets is unknown.
5. **Multi-accelerator composition**: Can DSA→IAA→QAT be chained transparently through the same framework?
6. **Adaptive offload policy**: At what message size does DSA offload beat CPU memcpy, given framework overhead?

---

*This document synthesizes findings from `docs/report/design_decisions.md`, `docs/report/progress_post_alignment_debug.md`, `docs/report/stdexec_overhead_results.md`, `docs/report/mock_benchmark_results.md`, and the module READMEs. See those files for raw data and detailed analysis.*
