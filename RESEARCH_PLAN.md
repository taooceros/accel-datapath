# RESEARCH_PLAN.md

# The Batching Regime Change: When Hardware Gets Fast Enough That Software Becomes the Bottleneck

*Hongtao Zhang, University of Washington*
*Advisor: Arvind Krishnamurthy*

---

## 1. Executive Summary

A quiet regime change is underway in datacenter I/O. Hardware submission
mechanisms — MMIO doorbells for accelerators, NIC descriptor rings, io_uring
submission queues, NVMe command rings — have always carried per-operation
overhead that dwarfs the work they initiate for small operations. The
traditional response is **batching**: amortize one expensive doorbell across
many operations. But batching has a consequence that the systems community has
not fully reckoned with: once hardware cost is amortized to single-digit
nanoseconds per operation, the **software framework** that feeds the hardware
becomes the dominant bottleneck.

We have demonstrated this concretely for Intel DSA (Data Streaming
Accelerator). With batched submission, DSA hardware adds only ~5 ns/op of
amortized overhead — the 4-engine device is underutilized. Yet the full
async framework path (C++ P2300/stdexec) costs 38 ns/op, of which 21 ns is
pure framework overhead: scope tracking (14 ns) and per-operation connection
setup (7 ns). **Software is 70–80% of total per-operation cost.** By
progressively stripping framework layers, we achieved 84 Mpps on mock hardware
and 34 Mpps on real DSA — a 1.87x improvement — not by improving the hardware,
but by reducing the software between the application and the hardware.

This is not a DSA-specific finding. The same dynamic applies wherever batching
amortizes submission overhead:

- **RDMA NICs** at 180+ Mpps: NIC hardware can process descriptors faster than
  software can fill them; Mellanox BlueFlame and doorbell batching were invented
  precisely because per-descriptor MMIO was the bottleneck.
- **io_uring**: submission batching (`io_uring_submit` with multiple SQEs)
  amortizes the single syscall; the bottleneck shifts to SQE preparation in
  userspace.
- **NVMe**: command batching amortizes doorbell writes; at high IOPS the CPU
  cost of command construction dominates.
- **GPU command submission**: CUDA graph batching amortizes launch overhead;
  the bottleneck shifts to graph construction.

In every case, batching creates a new performance regime where the operations
are fast enough that the software framework — designed for microsecond-scale
I/O where framework overhead was negligible — becomes the limiting factor.

**The core insight**: Composability is not inherently expensive. Frameworks like
stdexec, gRPC's EventEngine, io_uring's liburing, and DPDK's rte_ethdev were
designed when per-operation hardware latency was microseconds. At that scale,
tens of nanoseconds of framework overhead is noise. Batching changes the
equation: when hardware cost drops to 5 ns/op, framework overhead at 20+ ns/op
is no longer noise — it is the majority of the cost. These frameworks were
never optimized for the nanosecond regime because such a regime did not exist
before batched submission became the norm.

**What we propose.** We will:

1. **Characterize the batching regime change** across hardware domains (DSA,
   RDMA NICs, io_uring, NVMe), measuring where software becomes the bottleneck
   in each and identifying the common structural causes.
2. **Decompose end-to-end RPC cost** (gRPC) to determine which components
   benefit from accelerator offload and at what message sizes, using the
   layer-removal methodology we developed and validated.
3. **Design frameworks for the nanosecond regime** — composable async
   abstractions that maintain the safety and expressiveness of current
   frameworks while reducing per-operation overhead to match batched hardware.
4. **Complete the intra-host data movement picture** by characterizing the
   host-to-accelerator path, complementing prior work that characterized the
   NIC-to-host path [Agarwal, Krishnamurthy et al., SIGCOMM 2024].

---

## 2. Motivation and Problem Statement

### 2.1 The Batching Regime Change

Every hardware accelerator and I/O device in a modern server is accessed
through a submission mechanism: the CPU prepares a descriptor (command) in
memory, then signals the device via an MMIO doorbell write or similar
notification. The doorbell is expensive — an uncacheable write to device-mapped
memory that serializes the CPU pipeline and crosses the on-die fabric. On
current Intel Xeons, a single MMIO doorbell costs approximately 150–500 ns
depending on the target device.

For large operations (multi-kilobyte DMA transfers, disk reads, network packet
transmission), doorbell overhead is negligible compared to operation time. But
for the small, frequent operations that dominate datacenter workloads —
8-byte memcpys in RPC serialization, 64-byte cache-line flushes, small-message
RDMA writes — the doorbell cost exceeds the work itself. A single DSA doorbell
costs ~160 ns; an 8-byte CPU memcpy costs ~3 ns.

**Batching** is the universal solution: accumulate multiple descriptors and ring
the doorbell once for the entire batch. Our MirroredRing submitter batches 32
DSA descriptors behind a single doorbell, reducing amortized hardware cost
from ~160 ns/op to ~5 ns/op. This is the same principle behind:

| System | Batching Mechanism | Amortized Overhead |
|---|---|---|
| DSA (this work) | MirroredRing: 32 descriptors/doorbell | ~5 ns/op |
| RDMA NIC (Mellanox) | Doorbell batching, BlueFlame | ~5–10 ns/op |
| io_uring | Multi-SQE submission per `io_uring_enter` | ~10–20 ns/op |
| NVMe | Command batching, 1 doorbell per batch | ~5–15 ns/op |
| GPU (CUDA) | Graph launch, stream batching | ~50–100 ns/op |

Once batching drives hardware cost into single-digit nanoseconds, a
previously-hidden cost becomes dominant: **the software framework that
prepares, submits, and completes operations.**

### 2.2 Software Frameworks Were Never Designed for the Nanosecond Regime

Modern async frameworks provide composability, safety, and expressiveness.
C++ P2300/stdexec provides structured concurrency with type-safe
sender/receiver chains. gRPC's EventEngine provides cross-platform async I/O
abstraction. io_uring's liburing provides safe submission queue management.
DPDK's rte_ethdev provides NIC-agnostic packet I/O.

These frameworks were designed when per-operation hardware latency was
microseconds:

| Era | Typical Operation | Hardware Latency | Framework Overhead | Overhead Fraction |
|---|---|---|---|---|
| Traditional I/O | disk read, TCP send | 10–1000 μs | 50–200 ns | <1% |
| Kernel bypass | RDMA send, DPDK tx | 1–5 μs | 20–50 ns | 1–5% |
| **Batched accelerator** | **DSA move, NVMe cmd** | **5–10 ns** | **20–40 ns** | **70–85%** |

At microsecond scale, framework overhead is invisible in end-to-end latency.
Developers reasonably chose simplicity and safety over nanosecond optimization.
Nobody inlined their stdexec `connect()` paths or pre-allocated operation
states because it didn't matter when hardware took 1000x longer.

**Batching changed this.** When hardware cost drops by two orders of magnitude,
framework overhead goes from invisible to dominant. This is not because
composability is inherently expensive — it is because these frameworks were
never optimized for a regime that didn't exist when they were designed.

### 2.3 The Evidence: stdexec on DSA

We built a complete C++ stdexec sender/receiver framework for Intel DSA and
measured where time goes using a **layer-removal methodology** (Section 3.2).
The results are unambiguous:

```
Full stdexec path (38 ns/op):
  scope.nest() + then()     14 ns    37%    ← async scope tracking + continuation chaining
  connect() + start()        7 ns    18%    ← per-op state construction (placement new of 448B)
  submit + poll + complete  17 ns    45%    ← actual hardware work + bookkeeping
                            -----
  Total:                    38 ns   100%

With batched submission, hardware adds only ~5 ns of the 17 ns "actual work."
The remaining 12 ns is descriptor preparation, completion checking, bookkeeping.
```

21 ns/op of pure framework overhead — for operations that the hardware
executes in 5 ns. The framework is 4x more expensive than the hardware.

**This overhead is not intrinsic to composability.** It reflects specific
implementation choices — per-operation heap allocation of 448-byte operation
states, runtime scope tracking for structured concurrency, generic
type-erasure for continuation chaining — that were reasonable when driving
microsecond-scale I/O and have never been revisited for the nanosecond regime.

### 2.4 Completing the Intra-Host Picture

Recent work from our group has systematically characterized the NIC-to-host
data path:

- **SIGCOMM 2024** ("Understanding the Host Network"): Data copies consume
  >50% of CPU cycles at 100G+; 49% cache miss rates on the receive side.
  Credit-based domain model quantifies host interconnect as a network.
- **SIGCOMM 2023** (hostCC): Host congestion control — controlling the *rate*
  of operations entering the host interconnect (IOMMU, memory bus).
- **OSDI 2024** (ZeroNIC): Data/control path separation achieves 17% CPU at
  100G (vs. 50% for Linux TCP) via custom FPGA NIC co-design.
- **MICRO 2025** (CXL-NIC): MMIO writes are the bottleneck for NIC command
  submission; replacing MMIO with CXL coherence messages yields 49% latency
  reduction.
- **SOSP 2024**: IOMMU overhead up to 60%, near-eliminable with better memory
  management.
- **HotNets 2025**: "Your Network Doesn't End at the NIC" — the intra-host
  network needs co-design with the inter-host network.

This body of work characterizes the **receive path**: NIC → PCIe → host
interconnect → memory → CPU. The bottlenecks are MMIO, cache misses, data
copies, and IOMMU overhead. Solutions are at the interconnect and congestion
control level.

**Our work characterizes the complementary path**: CPU → framework → descriptor
→ MMIO doorbell → accelerator → memory. This is the **offload/submission
path** — the path by which the host CPU drives on-die accelerators to perform
the very operations (copies, checksums, encryption) that dominate the receive
path.

Both paths hit the same physical bottlenecks — MMIO writes, cache misses,
memory bus contention — but from opposite directions. Together, they
characterize the full intra-host data movement pipeline.

The connection is particularly concrete for MMIO:

| Approach | Direction | Problem | Solution |
|---|---|---|---|
| CXL-NIC [MICRO 2025] | CPU → NIC | MMIO doorbells are expensive | Replace MMIO with CXL coherence (hardware) |
| This work | CPU → DSA | MMIO doorbells are expensive | Batch descriptors behind one doorbell (software) |

Software batching and CXL hardware replacement are **complementary solutions
to the same MMIO bottleneck**. Our work characterizes the software side;
CXL-NIC characterizes the hardware side. A combined approach — CXL-NIC
eliminating MMIO overhead + software batching reducing framework overhead —
could minimize submission cost entirely.

### 2.5 Multi-Threading Is Orthogonal

A natural question: why not simply use more CPU cores and more work queues to
achieve high aggregate throughput, without worrying about per-core efficiency?

Multi-threading does scale total throughput — each additional core with its own
work queue adds roughly linear capacity. But this does not change the per-core
offload economics. Each core independently decides: should I do this memcpy
on-CPU (3 ns), or submit it to DSA through the framework (38 ns)? The
crossover point where offload pays off depends on message size and framework
overhead, regardless of how many cores are running.

More fundamentally, using more cores for the same throughput means using more
power for the same work. At datacenter scale, the difference between 1 core
achieving 34 Mpps and 4 cores achieving the same 34 Mpps is a 4x difference in
power consumption for the offload path. In a world where datacenters are
power-constrained and accelerators are on-die (no PCIe power budget), per-core
efficiency directly translates to operations-per-watt.

Multi-threading is a scaling mechanism; per-core framework optimization is an
efficiency mechanism. They are orthogonal, and both matter.

### 2.6 The Fundamental Research Questions

1. **Regime characterization**: Across DSA, RDMA NICs, io_uring, and NVMe, at
   what batch size does software framework overhead become the dominant cost?
   What are the common structural causes (per-op allocation, type erasure,
   completion scanning)?

2. **Framework redesign**: Can composable async frameworks achieve sub-10 ns
   per-operation overhead — matching batched hardware cost — without sacrificing
   safety or expressiveness? What design principles distinguish ns-scale
   frameworks from μs-scale ones?

3. **End-to-end RPC decomposition**: For a gRPC call, what fraction of latency
   is memcpy, serialization, TLS, kernel, NIC? At what message sizes does
   each on-die accelerator offload actually reduce end-to-end latency?

4. **Multi-accelerator composition**: Can DSA→IAA→QAT be chained transparently?
   Does pipeline depth hide individual stage latencies, or does sequential
   submission overhead accumulate?

5. **Generalization**: Does the layer-removal + mock-hardware methodology
   generalize to other domains? Does the scheduling/submission separation
   principle hold for NIC rings, io_uring, GPU command buffers?

---

## 3. Preliminary Results

We have built a complete async framework for Intel DSA, developed two novel
measurement methodologies, and produced quantitative results that demonstrate
both the batching regime change and the feasibility of framework-level
optimization.

### 3.1 Complete Async Framework for Intel DSA

We implemented C++ sender/receiver (P2300/stdexec) bindings for all 8 Intel DSA
hardware operations: `data_move`, `mem_fill`, `compare`, `compare_value`,
`dualcast`, `crc_gen`, `copy_crc`, and `cache_flush`. Each operation is a
type-safe stdexec sender with:

- **Transparent page-fault retry** — DSA reports `COMP_PAGE_FAULT_NOBOF` on
  unmapped pages; the sender automatically touches the faulting page and
  re-submits, transparent to the caller.
- **Concept-based extensibility** — `TaskQueue` and `DescriptorSubmitter` are
  C++20 concepts with compile-time static dispatch (zero virtual calls on the
  hot path).
- **PollingRunLoop** — A custom stdexec run loop where a single thread drives
  both submission and completion polling, eliminating cross-thread coordination
  — the same pattern as eRPC and DPDK poll-mode drivers.

The framework's architecture exhibits a structural isomorphism with network I/O
that is central to our generalization thesis:

| DSA Framework | Network / I/O Analog |
|---|---|
| `DescriptorSubmitter` (ring buffer of HW descriptors) | NIC TX descriptor ring |
| `TaskQueue` (completion tracking) | NIC RX completion queue |
| `MirroredRingSubmitter` (memfd + dual mmap) | io_uring submission queue |
| `pre_poll()` flush before completion check | `io_uring_submit()` before `io_uring_wait_cqe()` |
| WQ backpressure (spin on full queue) | TCP `EAGAIN` / NIC TX ring full |

This isomorphism means findings about submission overhead, batching, and
completion polling transfer directly to NIC and io_uring programming.

### 3.2 Layer-Removal Methodology

Our most transferable contribution is the **layer-removal methodology** for
measuring per-layer abstraction cost.

Rather than using cycle counters (rdtsc) to time individual functions — which
misses interaction effects from instruction cache, branch prediction, and
out-of-order execution — we built three progressively-stripped benchmark
variants:

```
noalloc (full stdexec) → direct (no scope/then) → reusable (no connect/start)
      38 ns/op                 24 ns/op                  16.7 ns/op
          |______ 14 ns ______|      |_______ 7 ns ________|
          scope.nest + then          connect + start
```

Each variant removes a known set of abstractions. The throughput delta between
adjacent variants measures the cost of the removed layer — end-to-end,
including all microarchitectural interaction effects.

**Validation via negative result**: We also attempted the analytical approach —
decomposing the 38 ns baseline into per-phase estimates and predicting
optimization savings. Three targeted optimizations predicted to save 11 ns/op
actually saved only 2–3 ns/op. **The analytical model overpredicted by 4x**
because it could not account for compiler optimizations, out-of-order execution,
and phase interaction effects. This negative result is itself a methodological
contribution: it demonstrates that profiling-based cost attribution fails for
tightly-coupled nanosecond-scale pipelines, and validates layer-removal as the
correct alternative.

**Generality**: The layer-removal technique applies to any layered system. For
gRPC: build variants removing TLS, HTTP/2, protobuf, kernel — measure deltas.
For DPDK: strip rte_flow, RSS, VLAN — measure deltas. For io_uring: strip
liburing safety wrappers — measure deltas. The principle is the same:
progressive stripping gives measured per-layer cost without per-function
instrumentation.

### 3.3 Mock Hardware Methodology

To isolate software from hardware overhead, we built a **mock DSA** that
completes every operation instantaneously. Running identical benchmarks on mock
vs. real DSA cleanly separates software cost from hardware cost:

| Mode | Throughput | Per-op |
|---|---|---|
| Mock DSA (pure software) | 26 Mpps | 38 ns |
| Real DSA (stable regime) | 18 Mpps | 55 ns |
| Real DSA (unstable regime) | 9–11 Mpps | ~100 ns |

The mock eliminates the bistable throughput regime (Section 3.6), producing
clean, reproducible measurements. **Network analog**: a mock NIC (loopback with
zero wire latency) isolates protocol stack overhead from transit time. This
methodology generalizes to any system with hardware-in-the-loop variability.

### 3.4 Scheduling/Submission Orthogonality and Transparent Auto-Batching

A key architectural finding is the **separation of scheduling from submission**
— two orthogonal design dimensions:

- **Scheduling** = how the application decides when to issue operations (sliding
  window, batch, scoped workers)
- **Submission** = how descriptors physically reach hardware (immediate,
  double-buffered, fixed-ring, MirroredRing)

Our MirroredRing submitter uses `memfd_create` + dual `mmap` to create a
virtual-memory-aliased ring buffer where descriptors wrap without modular
arithmetic — the same technique used by io_uring. It batches 32 descriptors
behind a single MMIO doorbell, providing 1.2–2x throughput gain **completely
transparent to the scheduling layer**. The same sliding-window code runs
identically on immediate (1 doorbell/op, ~6 Mpps) or batched (1 doorbell/32
ops, 18–35 Mpps) backends.

An ablation study shows that **in-flight batch depth** (16 vs. 2 concurrent
batch slots) is the dominant performance factor, not ring buffer cleverness.
Deep submission queues matter more than clever memory management.

This scheduling/submission separation is not specific to DSA. It appears in
every hardware I/O system: NIC descriptor rings (submission) vs. socket API
(scheduling); io_uring SQ (submission) vs. application event loop (scheduling);
GPU command buffers (submission) vs. CUDA stream API (scheduling). We believe it
should be formalized as a design principle.

### 3.5 Quantitative Results

Hardware: Intel Xeon Gold 6438M (Sapphire Rapids), 4 DSA engines, 8B data_move.

**Layer-removal results (mock DSA — pure software cost):**

| Strategy | c=32 | c=1024 | c=2048 | Per-op (c=32) |
|---|---|---|---|---|
| `noalloc` (full stdexec) | 28.9 Mpps | 26.7 | 26.3 | 34.6 ns |
| `direct` (no scope/then) | 46.2 Mpps | 41.4 | 41.6 | 21.6 ns |
| `reusable` (no connect/start) | **83.9 Mpps** | 62.5 | 59.9 | **11.9 ns** |

Stable across 3 runs (stdev < 1 Mpps). All 8 DSA operations show consistent
speedups.

**Real DSA hardware confirms gains transfer:**

| Strategy | c=64 | c=256 | c=1024 | Speedup |
|---|---|---|---|---|
| `noalloc` (baseline) | 12.5 Mpps | 15.4 | 18.2 | 1.0x |
| `direct` | 14.3 | 24.7 | 27.5 | 1.5x |
| `reusable` | 15.0 | 29.5 | **34.0** | **1.87x** |

Gains are *larger* on real hardware because reducing software overhead also
improves hardware utilization — fewer poll traversals mean tighter batches,
keeping the DSA engines busier. This is a positive feedback loop: less software
overhead → better batching → more hardware throughput → proportionally even less
software overhead.

**The batching regime change, quantified:**

| Component | Cost (ns/op) | Fraction |
|---|---|---|
| Framework overhead (scope, then, connect, start) | 21 | 55% |
| Software bookkeeping (descriptor fill, poll, complete) | 12 | 32% |
| Hardware (amortized with batch=32) | ~5 | 13% |
| **Total** | **38** | **100%** |

Hardware is 13% of total cost. Software is 87%. The batching regime change
is real and quantified.

### 3.6 Bistable Throughput Regime

We discovered that the same benchmark configuration on real DSA can produce
either ~20 Mpps or ~10 Mpps depending on initial conditions. This **bistable
throughput regime** is absent from mock DSA.

Root cause: a positive feedback loop between hardware completion rate and
software poll traversal cost:

```
fewer completions/poll → more wasted O(N) scan time → longer submission gap
→ smaller effective batch → fewer completions (repeat)
```

**This is general.** Any poll-mode system (DPDK, io_uring) that scans
outstanding requests with O(N) cost can exhibit this bistability. NIC
completion queues avoid it via O(1) completion notification. The cure is the
same: O(1) completion mechanisms (completion bitmaps, hardware interrupts,
event-driven notification). This finding applies to any system integrating
hardware accelerators with poll-mode I/O loops.

### 3.7 Cache Hierarchy as Performance Boundary

Mock DSA experiments reveal that throughput degrades with concurrency even when
hardware is free. The cause is cache pressure from per-operation metadata:

| Concurrency | Working Set | Cache Level | Throughput | Per-op |
|---|---|---|---|---|
| 32 | 12 KB | L1d (48 KB) | 84 Mpps | 11.9 ns |
| 1024 | 384 KB | L2 (2 MB) | 62.5 Mpps | 16.0 ns |
| 2048 | 768 KB | L2 | 59.9 Mpps | 16.7 ns |
| 4096 | 1536 KB | L2/L3 boundary | 61.3 Mpps | 16.3 ns |

The L1→L2 transition adds ~4 ns/op, consistent with Sapphire Rapids L2 hit
latency. At c=2048, ~30% of per-operation time is cache-miss overhead.

**Implication for framework design**: per-operation metadata size directly
determines cache residency, which determines throughput. stdexec's 448-byte
operation state means c=100 fills L1. A framework designed for the ns-regime
must minimize per-operation metadata to maximize cache-resident concurrency.
This is a concrete design constraint absent from μs-scale framework design.

---

## 4. Proposed Research

### Thrust 1: Characterizing the Batching Regime Change Across Hardware Domains

**Years 1–2** | *Core question: Is the software-becomes-bottleneck phenomenon
general, and what are its common structural causes?*

#### Approach

Apply the layer-removal and mock-hardware methodologies to four hardware
domains beyond DSA:

**RDMA NIC (Mellanox ConnectX-6/7)**:
- Build progressively-stripped RDMA send benchmarks: full ibverbs → minimal
  ibv_post_send → raw WQE construction → pre-allocated WQEs
- Measure: at what batch size does ibverbs abstraction overhead exceed wire
  time? What is the cost of completion queue polling vs. hardware interrupts?
- Compare with DSA: same MMIO doorbell bottleneck, same batching solution,
  same regime change?

**io_uring**:
- Strip liburing wrappers progressively: full liburing API → raw
  io_uring_enter → pre-filled SQEs with manual submission
- Measure: what fraction of per-SQE cost is the kernel boundary vs. userspace
  preparation? How does SQE batching change the balance?
- Connection: io_uring's submission model is structurally identical to DSA's
  (ring buffer + doorbell). Does the same scheduling/submission separation
  apply?

**NVMe**:
- Use SPDK or raw NVMe passthrough to build stripped variants
- Measure: at high IOPS (millions/sec), what fraction of cost is command
  construction vs. doorbell vs. completion?

**GPU (CUDA)**:
- Compare CUDA graph launch (batched) vs. individual kernel launch
- Measure: with graph batching, does CUDA runtime overhead dominate?

For each domain, we produce: (a) a layer-removal cost decomposition showing
where time goes, (b) a batching crossover analysis showing when software
becomes dominant, and (c) identification of the specific framework operations
(allocation, type erasure, completion scanning) that dominate.

#### Expected Findings

Based on preliminary DSA results, we expect:
- The regime change occurs at batch sizes of 16–64 across all domains
- Common structural causes: per-operation memory allocation, generic dispatch
  (virtual calls or type erasure), O(N) completion scanning
- The scheduling/submission separation holds in all four domains
- Cache working-set effects (Section 3.7) are universal: frameworks that
  allocate large per-operation metadata hit cache boundaries earlier

#### Deliverables

- Cross-domain characterization paper: "The Batching Regime Change: When
  Software Becomes the Hardware Bottleneck"
- **Publication target**: OSDI/SOSP — this is a measurement-and-insight paper
  that reframes how the community thinks about framework design
- Layer-removal methodology formalized with guidelines for variant construction
- Open-source multi-domain benchmark suite

---

### Thrust 2: End-to-End RPC Component Analysis with Accelerator Offload

**Years 1–3** | *Core question: For a gRPC call end-to-end, where does time go,
and how much does each on-die accelerator actually save?*

#### Approach

Apply the layer-removal methodology to gRPC, building progressively-stripped
variants:

| Level | Configuration | What's Measured |
|---|---|---|
| 0 | Full gRPC (protobuf + HTTP/2 + TLS + kernel TCP) | Baseline end-to-end |
| 1 | Replace internal memcpys with DSA `data_move` | Memory copy cost + DSA offload benefit |
| 2 | Replace CRC/checksums with DSA `crc_gen` | Checksumming cost |
| 3 | Replace TLS (OpenSSL/BoringSSL) with QAT | Encryption cost + QAT offload benefit |
| 4 | Replace gzip/deflate with IAA | Compression cost + IAA offload benefit |
| 5 | Replace kernel TCP with io_uring zero-copy or DPDK | Kernel overhead |
| 6 | Shared-memory transport (no NIC) | NIC + wire cost |

Note: gRPC is written in C/C++ and uses its own async machinery (EventEngine),
not stdexec. Our stdexec + DSA work is the *case study* that validated the
methodology; gRPC decomposition is the *application* of that methodology.
The layer-removal technique is framework-agnostic — it works by measuring
end-to-end deltas, not by instrumenting specific framework internals.

Sweep across:
- **Message sizes**: 64 B to 1 MB (protobuf payloads)
- **Concurrency**: 1 to 10,000 outstanding RPCs
- **Payload types**: protobuf, FlatBuffers, Cap'n Proto, raw bytes
- **RPC patterns**: unary, server-streaming, bidirectional streaming

For each (message size, concurrency) point, produce a **crossover map**: at what
size does DSA offload beat CPU memcpy? QAT beat software AES-GCM? IAA beat
software deflate?

#### Connection to message-rate workloads

The batching regime change matters most for **high-message-rate workloads**
where per-operation overhead dominates:
- Key-value stores (Redis, Memcached): millions of small GETs/PUTs per second
- Microservices RPC fanout (DeathStarBench): tens of internal RPCs per request
- DNS resolvers, load balancer proxies: millions of lookups/sec
- Telemetry and logging pipelines: high-rate small events

An open question we will investigate: does this extend to emerging AI
workloads? Disaggregated inference, Mixture-of-Experts routing, and KV-cache
transfers all involve frequent small data movements between components.

#### Deliverables

- Complete gRPC latency decomposition across message sizes and concurrency
- Crossover maps for each accelerator showing break-even points
- Open-source instrumented gRPC with per-component timing hooks
- **Publication target**: NSDI — "Where Does Time Go in Accelerator-Assisted
  RPC?"

---

### Thrust 3: Framework Design for the Nanosecond Regime

**Years 2–4** | *Core question: Can composable async frameworks achieve sub-10
ns per-operation overhead without sacrificing safety or expressiveness?*

#### Approach

Our layer-removal results identify exactly where stdexec's 21 ns/op overhead
comes from: scope tracking (14 ns) and per-operation connection setup (7 ns).
Neither is fundamental to composability — they are implementation choices that
were never optimized for the ns-regime:

**Scope tracking (14 ns)**: `scope.nest()` registers each operation for
structured concurrency lifetime tracking. This is essential for correctness
(preventing dangling operations) but the current implementation uses runtime
bookkeeping per-operation. Alternative: compile-time scope inference for static
operation graphs, with runtime tracking only for dynamic patterns.

**Per-operation connection (7 ns)**: `connect()` constructs a 448-byte
operation state via placement new; `start()` initializes it. Alternative:
pre-allocated operation state pools with reset-and-reuse semantics (our
`reusable` strategy already demonstrates this works, achieving 84 Mpps).

**Design directions:**

1. **Reusable operation states for stdexec**: Extend the P2300 sender/receiver
   model with an `operation_pool` concept that pre-allocates and recycles
   operation states. This eliminates per-op allocation while preserving
   type safety and composability. Measure: does this close the gap between
   `noalloc` (26 Mpps) and `reusable` (84 Mpps)?

2. **Lightweight sender adapters**: Design sender combinators (`then`, `let`,
   `when_all`) with minimal per-operation metadata. Current stdexec combinators
   are generic and heap-allocate; ns-regime combinators should be specialized
   and stack/arena-allocated where possible.

3. **O(1) completion mechanisms**: Replace O(N) poll traversal with completion
   bitmaps or interrupt-driven notification. This breaks the bistable feedback
   loop (Section 3.6) and reduces completion overhead from O(concurrency) to
   O(completions).

4. **Cache-conscious metadata layout**: Per-operation metadata should be sized
   for L1 residency at target concurrency levels. At 48 KB L1d and c=128
   target, per-op metadata must be ≤375 bytes. stdexec's 448 bytes exceeds
   this; `reusable` at 384 bytes barely fits; the target should be <256 bytes.

5. **Multi-accelerator composition**: Extend the framework to chain
   DSA→IAA→QAT transparently. All three share the same MMIO doorbell +
   completion record programming model. The scheduling/submission separation
   already proven for DSA should extend to heterogeneous pipelines.

**Adaptive offload policy**: For small messages, CPU is faster than any
accelerator offload. Build an adaptive scheduler that dynamically chooses CPU
vs. accelerator per-operation based on message size, queue depth, and observed
throughput. The policy resembles TCP congestion control: probe for the optimal
operating point, back off on overload, converge to equilibrium.

#### gRPC EventEngine integration

gRPC uses its own EventEngine abstraction for async I/O, not stdexec. A
practical impact path is to apply ns-regime design principles to EventEngine:
- Pre-allocated operation state pools for gRPC closures
- Batched submission of sendmsg/recvmsg through io_uring with EventEngine
  as the scheduling layer
- DSA integration for buffer management within EventEngine's existing
  memory allocation hooks

This demonstrates that the insights generalize beyond stdexec to production
frameworks.

#### Deliverables

- ns-regime stdexec extensions (operation pools, lightweight combinators)
- Multi-accelerator (DSA + QAT + IAA) pipeline implementation
- Adaptive offload policy with measured crossover points
- gRPC EventEngine prototype with ns-regime optimizations
- **Publication target**: ASPLOS — "Composable Frameworks for Nanosecond-Scale
  Hardware"

---

### Thrust 4: Accelerator-Native Transports and Systems Integration

**Years 3–5** | *Core question: Can accelerator-native transports compete with
kernel-bypass and SmartNIC approaches using only commodity hardware?*

#### Approach

**gRPC accelerator transport**: Implement a custom gRPC transport that replaces
the default TCP transport with an accelerator-aware path:
- DSA for buffer management (zero-copy between protocol layers)
- IAA for message compression (`grpc-encoding: deflate`)
- QAT for TLS (`qat_engine` with OpenSSL)
- io_uring for kernel-bypass I/O
- Adaptive offload for per-message CPU/accelerator selection

**UCX transport plugin**: Implement `uct_md` (memory domain) and `uct_iface`
(transport interface) for DSA. UCX's component architecture allows pluggable
transports; our transparent auto-batching maps directly to UCX's non-blocking
API. This enables every MPI and OpenSHMEM implementation built on UCX to
benefit from DSA offload without application changes.

**End-to-end benchmarks:**
- DeathStarBench [ASPLOS 2019]: microservices with gRPC
- HyperProtoBench [RPCAcc 2024]: protobuf serialization-heavy workload
- OSU Micro-Benchmarks: point-to-point and collective latency/bandwidth
- Application kernels: distributed KV store, graph analytics RPC

**Comparison with state of the art:**

| System | Approach | Our Advantage |
|---|---|---|
| RPCAcc [arXiv 2024] | SmartNIC (FPGA) + DSA | No PCIe round-trip; commodity hardware |
| RpcNIC [HPCA 2025] | SmartNIC RPC accelerator | No special NIC; works with any NIC |
| ZeroNIC [OSDI 2024] | Custom FPGA NIC | Commodity Xeon; same data/control separation |
| Cornflakes [SOSP 2023] | NIC scatter-gather | Multi-accelerator; handles encryption |
| eRPC [NSDI 2019] | Kernel bypass (DPDK) | Offloads CPU entirely for data movement |

Our on-die approach requires no special hardware beyond a commodity Xeon,
eliminates PCIe round-trips, and composes multiple accelerators. ZeroNIC
achieves 17% CPU at 100G with a custom FPGA NIC; can on-die accelerators
achieve comparable savings with commodity hardware?

**CXL + DSA investigation**: CXL-attached memory introduces 150–300 ns access
latency. Our cache analysis (Section 3.7) shows that even L1→L2 (+4 ns)
degrades throughput by 25%. If DSA operates on CXL-attached buffers, does
the increased memory latency negate the offload benefit? This connects directly
to CXL-NIC [MICRO 2025] and the broader CXL memory disaggregation research.

#### Deliverables

- Accelerator-native gRPC transport (open source)
- UCX DSA transport plugin
- End-to-end comparison: on-die accelerator vs. kernel-bypass vs. SmartNIC
- CXL + DSA characterization
- **Publication target**: NSDI/SIGCOMM — "On-Die Accelerator Transports for
  Datacenter RPC"

---

## 5. Expected Outcomes and Broader Impacts

### 5.1 Technical Outcomes

- **Cross-domain characterization of the batching regime change**: the first
  systematic measurement showing that software frameworks become the bottleneck
  once hardware submission is batched, across DSA, RDMA, io_uring, and NVMe.

- **First rigorous gRPC latency decomposition with accelerator offload**:
  crossover maps showing at what message sizes and concurrency levels each
  on-die accelerator pays off.

- **Framework design principles for the nanosecond regime**: concrete guidelines
  (operation pooling, cache-conscious metadata, O(1) completion) that apply to
  stdexec, gRPC EventEngine, and other async frameworks.

- **Accelerator-native transports at competitive message rates**: open-source
  gRPC and UCX implementations on commodity Xeon hardware.

### 5.2 Impact on the Systems Research Community

**Reframing the framework design conversation.** The systems community has
spent decades optimizing hardware paths (NICs, switches, kernel bypass) while
treating software frameworks as fixed overhead. Our work shows that batching
has inverted this: hardware is now cheap and software is expensive. This
reframing should influence how the community designs async frameworks,
completion mechanisms, and submission APIs.

**Completing the intra-host picture.** Agarwal, Krishnamurthy et al.
characterized the NIC-to-host receive path; we characterize the host-to-
accelerator offload path. Together, the full intra-host data movement pipeline
is measured and understood. This is directly relevant to the "Your Network
Doesn't End at the NIC" vision [HotNets 2025].

**Bridging hardware and software solutions.** CXL-NIC provides a hardware
solution to the MMIO bottleneck; our batching provides a software solution.
These are complementary, and a combined approach could minimize submission
overhead entirely. Our characterization of the software side informs hardware
architects about what software can and cannot solve.

### 5.3 Connection to UW Systems Research

- **"Understanding the Host Network" [SIGCOMM 2024]**: We provide the
  accelerator-based response to the data-copy bottleneck identified by
  Vuppalapati, Agarwal, Krishnamurthy et al. They measured the receive path;
  we measure the offload path.

- **Host congestion control (hostCC) [SIGCOMM 2023]**: hostCC controls the
  *rate* of operations entering the host interconnect. Our work characterizes
  the *per-operation cost* of those operations. Together: rate control (hostCC)
  + per-operation optimization (this work) = efficient accelerator utilization.

- **Kernel-bypass networking (eRPC)**: Accelerator offload is complementary to
  kernel bypass. eRPC eliminates kernel overhead; on-die accelerators eliminate
  CPU data-movement overhead. The two compose: an eRPC-like system using DSA
  for copies and QAT for encryption offloads the CPU entirely.

- **CXL / disaggregated memory**: DSA is the natural data-movement engine for
  CXL-attached memory. Our CXL + DSA investigation determines whether DSA can
  serve as the bulk transfer mechanism in disaggregated memory systems.

### 5.4 Impact on Standards and Software Ecosystem

- **C++ P2300 (stdexec)**: Our measured 21 ns/op overhead is the first
  empirical characterization of P2300 cost on real hardware accelerators.
  Framework redesign findings (Thrust 3) directly inform WG21 proposals for
  operation pools and lightweight combinators.

- **Open-source tooling**: All framework code, benchmark infrastructure, and
  transport implementations released as open source.

### 5.5 Educational Impact

- Graduate training at the intersection of networking, hardware architecture,
  and systems programming.
- Tutorial materials for on-die accelerator programming (currently nonexistent
  outside Intel documentation).
- Open benchmark suite as research infrastructure for the community.

---

## 6. Timeline and Milestones

| Period | Thrust | Milestone | Publication Target |
|---|---|---|---|
| Y1 Q1–Q2 | T1, T2 | RDMA + io_uring layer-removal; gRPC baseline decomposition | HotNets/HotOS workshop |
| Y1 Q3–Q4 | T1, T2 | NVMe + GPU characterization; DSA memcpy in gRPC; crossover analysis | — |
| Y2 Q1–Q2 | T1, T2 | Cross-domain regime change paper; QAT TLS + IAA in gRPC | OSDI/SOSP submission |
| Y2 Q3–Q4 | T2, T3 | Adaptive offload policy; ns-regime stdexec extensions | — |
| Y3 Q1–Q2 | T2, T3 | gRPC accelerator transport beta; multi-accelerator pipeline | NSDI submission |
| Y3 Q3–Q4 | T3, T4 | UCX transport plugin; EventEngine ns-regime prototype | — |
| Y4 Q1–Q2 | T3, T4 | End-to-end benchmarks (DeathStarBench); on-die vs SmartNIC comparison | ASPLOS submission |
| Y4 Q3–Q4 | T4 | CXL + DSA characterization; open benchmark release | NSDI/SIGCOMM submission |
| Y5 | All | Production release; upstream contributions; standards proposals | — |

**Key risk mitigations:**
- DSA framework (our preliminary results) de-risks all thrusts by proving the
  sender/receiver model and measurement methodology work.
- Mock hardware methodology allows progress when real hardware access is
  limited.
- Layer-removal is framework-agnostic — works on gRPC, DPDK, io_uring without
  modifying their internals.
- Multi-threading fallback: if per-core optimization hits diminishing returns,
  multi-core scaling remains a viable path.

---

## 7. Related Work

### Host Network Characterization

- **"Understanding the Host Network"** [SIGCOMM 2024, Best Student Paper] —
  Vuppalapati, Agarwal, Schuh, Kasikci, Krishnamurthy, Agarwal. Copies
  consume >50% CPU at 100G+. Characterizes the receive path; we characterize
  the offload path.

- **hostCC** [SIGCOMM 2023] — Agarwal et al. Host congestion control for
  intra-host interconnect. Rate control for operations entering the host
  fabric.

- **ZeroNIC** [OSDI 2024] — Data/control path separation via FPGA NIC
  co-design. 17% CPU at 100G. Custom hardware; our approach uses commodity
  Xeon on-die accelerators.

- **CXL-NIC** [MICRO 2025] — Agarwal et al. MMIO writes are THE expensive
  primitive for NIC submission; CXL coherence replaces MMIO, 49% faster.
  Hardware solution to the same MMIO bottleneck we address with software
  batching.

- **"Your Network Doesn't End at the NIC"** [HotNets 2025] — Intra-host
  network needs co-design with inter-host. Our work provides measurement
  methodology for the intra-host offload path.

- **IOMMU overhead** [SOSP 2024] — Up to 60% overhead, near-eliminable with
  better memory management.

### RPC Acceleration

- **RPCAcc** [arXiv 2411.07632, 2024] — PCIe-attached FPGA RPC accelerator.
  55% CPU reduction with DSA memcpy offload. External hardware; we use on-die.

- **RpcNIC** [HPCA 2025] — SmartNIC-native RPC with RoCE. Requires specialized
  NIC.

- **Arcalis** [arXiv 2602.12596, 2026] — Near-cache RPC accelerator. Custom
  ASIC; our approach uses commodity hardware.

- **Cornflakes** [SOSP 2023] — Zero-copy serialization via NIC scatter-gather.
  Addresses copies but not encryption or compression.

### Intel Accelerators

- **DSA Characterization** [ASPLOS 2024] — Kuper et al. Definitive DSA
  performance characterization. Async/polling model essential.

- **DSA VPP Plugin** [Intel, 2024] — 1.9–2.6x throughput for packet forwarding.

- **IAA for Databases** [ADMS@VLDB 2025] — CMU. Up to 3.15x faster
  decompression.

- **QAT TLS** [Various Intel guides, 2024] — 51–69% improvement in TLS
  connection rates.

### Async Frameworks and I/O

- **P2300R10** (stdexec) adopted into C++26. Our work provides first empirical
  overhead characterization on real hardware.

- **uring_exec** — io_uring wrapped as stdexec scheduler. Demonstrates
  framework unification.

- **io_uring zero-copy Rx** [Linux 6.15] — Saturates 200G from single core.
  Composable with DSA for buffer management.

- **"A Wake-Up Call for Kernel-Bypass on Modern Hardware"** [DAMON 2025] —
  Modern hardware changes kernel-bypass tradeoffs.

### Zero-Copy and Serialization

- **zFlatBuffers** [IPCCC 2023] — Zero-copy serialization for RDMA-based RPC.
- **SerDes-free State Transfer** [EuroSys 2024] — Eliminates serialization for
  same-host serverless.

---

## 8. References

[1] Vuppalapati, Agarwal, Schuh, Kasikci, Krishnamurthy, Agarwal.
    "Understanding the Host Network." SIGCOMM 2024 (Best Student Paper).

[2] Agarwal et al. "hostCC: Host Congestion Control." SIGCOMM 2023.

[3] Agarwal et al. "ZeroNIC: Data/Control Path Separation." OSDI 2024.

[4] Agarwal et al. "CXL-NIC: Replacing MMIO with CXL Coherence." MICRO 2025.

[5] Agarwal et al. "Your Network Doesn't End at the NIC." HotNets 2025.

[6] RPCAcc. "A High-Performance PCIe-attached RPC Accelerator."
    arXiv:2411.07632, 2024.

[7] RpcNIC. "Enabling Efficient Datacenter RPC Offloading." HPCA 2025.

[8] Arcalis. "Near-Cache RPC Acceleration." arXiv:2602.12596, 2026.

[9] Margaritov et al. "Cornflakes: Zero-Copy Serialization." SOSP 2023.

[10] Kuper et al. "Quantitative Analysis of DSA in Modern Xeon Processors."
     ASPLOS 2024.

[11] P2300R10. "std::execution." ISO/IEC JTC1/SC22/WG21, C++26.

[12] Gan et al. "DeathStarBench: Open-Source Benchmark for Microservices."
     ASPLOS 2019.

[13] Kalia et al. "Datacenter RPCs can be General and Fast." NSDI 2019 (eRPC).

[14] "Efficient Zero-Copy Networking using io_uring." Kernel Recipes 2024.

[15] "A Hot Take on Intel IAA for DBMS." ADMS@VLDB 2025.

[16] "Protocol Buffer Deserialization DPU Offloading." SC 2024.

[17] "zFlatBuffers: Zero-Copy Serialization for RDMA-based RPC." IPCCC 2023.

[18] "SerDes-free State Transfer in Serverless Workflows." EuroSys 2024.

[19] "A Wake-Up Call for Kernel-Bypass on Modern Hardware." DAMON 2025.

[20] "IOMMU Overhead Characterization." SOSP 2024.
