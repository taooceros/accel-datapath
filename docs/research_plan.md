# Research Plan: The Batching Regime Change

**When Hardware Gets Fast Enough That Software Becomes the Bottleneck**

**Author**: Hongtao Zhang
**Advisor**: Arvind Krishnamurthy
**Last updated**: 2026-03-03

---

## 1. Executive Summary

A regime change is underway in datacenter I/O. Hardware submission mechanisms — MMIO doorbells for accelerators, NIC descriptor rings, io_uring submission queues — have always carried per-operation overhead that dwarfs the work they initiate for small operations. The universal response is **batching**: amortize one expensive doorbell across many operations. But batching has a consequence the systems community has not fully reckoned with: once hardware cost is amortized to single-digit nanoseconds per operation, the **software framework** that feeds the hardware becomes the dominant bottleneck.

We have demonstrated this concretely for Intel DSA (Data Streaming Accelerator). With batched submission, DSA hardware adds only ~5 ns/op of amortized overhead. Yet the full async framework path (C++ P2300/stdexec) costs 38 ns/op, of which 21 ns is pure framework overhead. **Software is 70-80% of total per-operation cost.** By progressively stripping framework layers, we achieved 84 Mpps on mock hardware and 34 Mpps on real DSA — a 1.87x improvement — by reducing software, not improving hardware.

This is not DSA-specific. The same dynamic applies wherever batching amortizes submission overhead: RDMA NICs, io_uring, NVMe, GPU command submission. We propose a 5-year research program to characterize this regime change across hardware domains, decompose end-to-end RPC cost, design frameworks for the nanosecond regime, and build accelerator-native transports.

---

## 2. The Problem: Batching Inverts the Bottleneck

### 2.1 Background

Every hardware accelerator and I/O device in a modern server is accessed through a submission mechanism: the CPU prepares a descriptor in memory, then signals the device via an MMIO doorbell write. The doorbell is expensive — an uncacheable write to device-mapped memory that serializes the CPU pipeline. On current Intel Xeons, a single MMIO doorbell costs approximately 150-500 ns depending on the target device.

For large operations (multi-kilobyte DMA transfers), doorbell overhead is negligible. But for the small, frequent operations that dominate datacenter workloads — 8-byte memcpys in RPC serialization, 64-byte cache-line flushes — the doorbell cost exceeds the work itself. A single DSA doorbell costs ~160 ns; an 8-byte CPU memcpy costs ~3 ns.

### 2.2 The Regime Change

**Batching** amortizes one doorbell across many operations. Our MirroredRing submitter batches 32 DSA descriptors behind a single doorbell, reducing amortized hardware cost from ~160 ns/op to ~5 ns/op. The same principle operates across domains:

| System | Batching Mechanism | Amortized Overhead |
|---|---|---|
| DSA (this work) | MirroredRing: 32 descriptors/doorbell | ~5 ns/op |
| RDMA NIC (Mellanox) | Doorbell batching, BlueFlame | ~5-10 ns/op |
| io_uring | Multi-SQE submission per `io_uring_enter` | ~10-20 ns/op |
| NVMe | Command batching, 1 doorbell per batch | ~5-15 ns/op |
| GPU (CUDA) | Graph launch, stream batching | ~50-100 ns/op |

Once batching drives hardware cost into single-digit nanoseconds, a previously hidden cost becomes dominant: **the software framework** that prepares, submits, and completes operations. Modern async frameworks were designed when per-operation hardware latency was microseconds:

| Era | Typical Operation | Hardware Latency | Framework Overhead | Overhead Fraction |
|---|---|---|---|---|
| Traditional I/O | disk read, TCP send | 10-1000 us | 50-200 ns | <1% |
| Kernel bypass | RDMA send, DPDK tx | 1-5 us | 20-50 ns | 1-5% |
| **Batched accelerator** | **DSA move, NVMe cmd** | **5-10 ns** | **20-40 ns** | **70-85%** |

### 2.3 The Core Insight

Composability is not inherently expensive. Frameworks like stdexec, gRPC's EventEngine, io_uring's liburing, and DPDK's rte_ethdev were designed when per-operation hardware latency was microseconds. At that scale, tens of nanoseconds of framework overhead is noise. Batching changes the equation: when hardware cost drops to 5 ns/op, framework overhead at 20+ ns/op is the majority of cost. These frameworks were never optimized for a regime that didn't exist before batched submission became the norm.

---

## 3. Preliminary Results

### 3.1 What We Built

A complete C++ stdexec sender/receiver framework for Intel DSA covering all 8 hardware operations (data_move, mem_fill, compare, compare_value, dualcast, crc_gen, copy_crc, cache_flush). Key properties:

- **Transparent page-fault retry**: DSA reports `COMP_PAGE_FAULT_NOBOF`; the sender automatically touches the page and re-submits
- **Concept-based extensibility**: `TaskQueue` and `DescriptorSubmitter` are C++20 concepts with compile-time static dispatch (zero virtual calls on hot path)
- **PollingRunLoop**: Single thread drives both submission and completion — same pattern as eRPC and DPDK poll-mode drivers
- **Transparent auto-batching**: Scheduling code calls `submit(descriptor)` one at a time; the MirroredRing backend silently accumulates and batch-submits. Throughput changes, scheduling code unchanged

### 3.2 Layer-Removal Methodology

Our most transferable contribution. Rather than timing individual functions with `rdtsc` — which misses interaction effects from instruction cache, branch prediction, and out-of-order execution — we built three progressively-stripped benchmark variants:

```
noalloc (full stdexec) → direct (no scope/then) → reusable (no connect/start)
      38 ns/op                 24 ns/op                  16.7 ns/op
          |______ 14 ns ______|      |_______ 7 ns ________|
          scope.nest + then          connect + start
```

Each variant removes a known set of abstractions. The throughput delta between adjacent variants measures the cost of the removed layer — end-to-end, including all microarchitectural interaction effects.

**Validation via negative result**: Three targeted optimizations predicted to save 11 ns/op actually saved only 2-3 ns/op. The analytical model overpredicted by 4x because it could not account for compiler optimizations, out-of-order execution, and phase interaction effects. This validates layer-removal as the correct methodology for nanosecond-scale systems.

### 3.3 Mock Hardware Methodology

`MockDsaBase` completes every operation instantaneously. Running identical benchmarks on mock vs. real DSA cleanly separates software from hardware overhead:

| Mode | Throughput | Per-op |
|---|---|---|
| Mock DSA (pure software) | 26 Mpps | 38 ns |
| Real DSA (stable regime) | 18 Mpps | 55 ns |
| Real DSA (unstable regime) | 9-11 Mpps | ~100 ns |

**Network analog**: a mock NIC (loopback with zero wire latency) isolates protocol stack overhead from transit time.

### 3.4 Quantitative Results

**Hardware**: Intel Xeon Gold 6438M (Sapphire Rapids), 4 DSA engines, 8B data_move.

**Layer-removal results (mock DSA — pure software cost):**

| Strategy | c=32 | c=1024 | c=2048 | Per-op (c=32) |
|---|---|---|---|---|
| `noalloc` (full stdexec) | 28.9 Mpps | 26.7 | 26.3 | 34.6 ns |
| `direct` (no scope/then) | 46.2 Mpps | 41.4 | 41.6 | 21.6 ns |
| `reusable` (no connect/start) | **83.9 Mpps** | 62.5 | 59.9 | **11.9 ns** |

**Real DSA hardware confirms gains transfer:**

| Strategy | c=64 | c=256 | c=1024 | Speedup |
|---|---|---|---|---|
| `noalloc` (baseline) | 12.5 Mpps | 15.4 | 18.2 | 1.0x |
| `direct` | 14.3 | 24.7 | 27.5 | 1.5x |
| `reusable` | 15.0 | 29.5 | **34.0** | **1.87x** |

Gains are *larger* on real hardware because reducing software overhead also improves hardware utilization — fewer poll traversals mean tighter batches.

**The batching regime change, quantified:**

| Component | Cost (ns/op) | Fraction |
|---|---|---|
| Framework overhead (scope, then, connect, start) | 21 | 55% |
| Software bookkeeping (descriptor fill, poll, complete) | 12 | 32% |
| Hardware (amortized with batch=32) | ~5 | 13% |
| **Total** | **38** | **100%** |

### 3.5 Key Findings

1. **Cache hierarchy as performance boundary**: At c=32 (12 KB, L1-resident), reusable reaches 84 Mpps. At c=1024 (384 KB, L2), only 62.5 Mpps. The L1→L2 transition adds ~4 ns/op. Per-operation metadata size directly determines throughput.

2. **Bistable throughput regime**: Real DSA produces either ~20 Mpps or ~10 Mpps for the same configuration. Root cause: positive feedback between O(N) completion scanning and hardware completion rate. Absent from mock. Any poll-mode system can exhibit this.

3. **Scheduling/submission orthogonality**: The same scheduling code (sliding window, batch, scoped workers) runs identically on any submission backend. This separation holds across hardware domains (NIC descriptor rings, io_uring, GPU command buffers).

4. **Batching exposes a new class of overhead**: The observation is not that batching is good (well-known) — it is that batching *shifts the bottleneck* from hardware to software, exposing framework overhead that was previously invisible.

---

## 4. Positioning: Completing the Intra-Host Picture

### 4.1 Relationship to Existing Work

Recent work from the group has characterized the NIC-to-host data path:

- **SIGCOMM 2024** ("Understanding the Host Network"): Data copies consume >50% of CPU cycles at 100G+; 49% cache miss rates on the receive side
- **SIGCOMM 2023** (hostCC): Host congestion control for the intra-host interconnect
- **OSDI 2024** (ZeroNIC): Data/control path separation via FPGA NIC co-design
- **MICRO 2025** (CXL-NIC): MMIO writes are the bottleneck for NIC submission; CXL coherence yields 49% latency reduction
- **HotNets 2025**: "Your Network Doesn't End at the NIC"

This body of work characterizes the **receive path**: NIC → PCIe → host interconnect → memory → CPU.

**Our work characterizes the complementary path**: CPU → framework → descriptor → MMIO doorbell → accelerator → memory. This is the **offload/submission path**.

### 4.2 MMIO: Same Bottleneck, Complementary Solutions

| Approach | Direction | Problem | Solution |
|---|---|---|---|
| CXL-NIC [MICRO 2025] | CPU → NIC | MMIO doorbells expensive | Replace MMIO with CXL coherence (hardware) |
| This work | CPU → DSA | MMIO doorbells expensive | Batch descriptors behind one doorbell (software) |

Combined: CXL eliminates MMIO overhead + software batching reduces framework overhead → minimal submission cost. Hardware improvements don't eliminate the need for software optimization — they shift the bottleneck and make software optimization matter *more*.

### 4.3 Multi-Threading Is Orthogonal

Using more CPU cores scales total throughput linearly, but doesn't change per-core offload economics. Each core independently decides: CPU memcpy (3 ns) or DSA offload (38 ns)? The crossover depends on message size and framework overhead, regardless of core count. At datacenter scale, 1 core at 34 Mpps vs. 4 cores at 34 Mpps is a 4x power difference.

---

## 5. Research Questions

1. **Regime characterization**: Across DSA, RDMA NICs, io_uring, and NVMe, at what batch size does software framework overhead become the dominant cost? What are the common structural causes?

2. **Framework redesign**: Can composable async frameworks achieve sub-10 ns per-operation overhead — matching batched hardware cost — without sacrificing safety or expressiveness?

3. **End-to-end RPC decomposition**: For a gRPC call, what fraction of latency is memcpy, serialization, TLS, kernel, NIC? At what message sizes does each on-die accelerator offload actually reduce end-to-end latency?

4. **Multi-accelerator composition**: Can DSA→IAA→QAT be chained transparently? Does pipeline depth hide individual stage latencies?

5. **Generalization**: Does the layer-removal + mock-hardware methodology generalize to other domains? Does the scheduling/submission separation principle hold for NIC rings, io_uring, GPU command buffers?

---

## 6. Proposed Research

### Thrust 1: Cross-Domain Characterization (Years 1-2)

*Core question: Is the software-becomes-bottleneck phenomenon general?*

Apply layer-removal and mock-hardware methodologies to four hardware domains beyond DSA:

**RDMA NIC (Mellanox ConnectX-6/7)**: Build progressively-stripped RDMA send benchmarks: full ibverbs → minimal ibv_post_send → raw WQE construction → pre-allocated WQEs. Measure: at what batch size does ibverbs overhead exceed wire time?

**io_uring**: Strip liburing wrappers progressively: full liburing API → raw io_uring_enter → pre-filled SQEs. Connection: io_uring's submission model is structurally identical to DSA's (ring buffer + doorbell).

**NVMe**: Use SPDK or raw NVMe passthrough. Measure: at high IOPS, what fraction is command construction vs. doorbell vs. completion?

**GPU (CUDA)**: Compare CUDA graph launch (batched) vs. individual kernel launch. Does CUDA runtime overhead dominate with graph batching?

**Deliverables**: Cross-domain characterization paper targeting **OSDI/SOSP**. Open-source multi-domain benchmark suite. Layer-removal methodology formalized with guidelines.

### Thrust 2: End-to-End RPC Decomposition (Years 1-3)

*Core question: For a gRPC call end-to-end, where does time go?*

Apply layer-removal to gRPC, building progressively-stripped variants:

| Level | Configuration | What's Measured |
|---|---|---|
| 0 | Full gRPC (protobuf + HTTP/2 + TLS + kernel TCP) | Baseline |
| 1 | Replace internal memcpys with DSA `data_move` | Memory copy cost |
| 2 | Replace CRC/checksums with DSA `crc_gen` | Checksumming cost |
| 3 | Replace TLS with QAT | Encryption cost |
| 4 | Replace gzip/deflate with IAA | Compression cost |
| 5 | Replace kernel TCP with io_uring zero-copy | Kernel overhead |
| 6 | Shared-memory transport (no NIC) | NIC + wire cost |

Sweep across message sizes (64 B to 1 MB), concurrency (1 to 10,000 RPCs), payload types, and RPC patterns. Produce **crossover maps**: at what size does DSA beat CPU memcpy? QAT beat software AES-GCM?

**Deliverables**: Complete gRPC latency decomposition. Crossover maps. Open-source instrumented gRPC. Targeting **NSDI**.

### Thrust 3: Framework Design for the Nanosecond Regime (Years 2-4)

*Core question: Can composable frameworks achieve sub-10 ns overhead?*

Our layer-removal identifies exactly where stdexec's 21 ns comes from. Neither source is fundamental to composability:

- **Scope tracking (14 ns)**: `scope.nest()` does runtime bookkeeping per-operation. Alternative: compile-time scope inference for static graphs.
- **Per-operation connection (7 ns)**: `connect()` constructs 448B state via placement new. Alternative: pre-allocated pools with reset-and-reuse (our `reusable` strategy proves this works at 84 Mpps).

Design directions:
1. **Operation pool concept** for stdexec: pre-allocate and recycle operation states
2. **Lightweight sender adapters**: specialized, stack/arena-allocated combinators
3. **O(1) completion mechanisms**: completion bitmaps instead of O(N) poll traversal
4. **Cache-conscious metadata**: <256 bytes/op target (vs. 448B current)
5. **Multi-accelerator composition**: DSA→IAA→QAT through the same framework

gRPC EventEngine integration as practical impact path.

**Deliverables**: ns-regime stdexec extensions. Multi-accelerator pipeline. Adaptive offload policy. gRPC EventEngine prototype. Targeting **ASPLOS**.

### Thrust 4: Accelerator-Native Transports (Years 3-5)

*Core question: Can accelerator-native transports compete with kernel-bypass and SmartNIC approaches using commodity hardware?*

**gRPC accelerator transport**: Custom transport with DSA (buffer management), IAA (compression), QAT (TLS), io_uring (kernel-bypass I/O), and adaptive per-message CPU/accelerator selection.

**UCX transport plugin**: `uct_md` + `uct_iface` for DSA. Enables every MPI and OpenSHMEM implementation on UCX to benefit from DSA offload.

**Comparison with state of the art:**

| System | Approach | Our Advantage |
|---|---|---|
| RPCAcc [arXiv 2024] | SmartNIC (FPGA) + DSA | No PCIe round-trip; commodity hardware |
| RpcNIC [HPCA 2025] | SmartNIC RPC accelerator | No special NIC; works with any NIC |
| ZeroNIC [OSDI 2024] | Custom FPGA NIC | Commodity Xeon; same data/control separation |
| eRPC [NSDI 2019] | Kernel bypass (DPDK) | Offloads CPU entirely for data movement |

**Deliverables**: Accelerator-native gRPC transport. UCX DSA plugin. End-to-end comparison. CXL + DSA characterization. Targeting **NSDI/SIGCOMM**.

---

## 7. Timeline and Milestones

| Period | Thrust | Milestone | Publication Target |
|---|---|---|---|
| Y1 Q1-Q2 | T1, T2 | RDMA + io_uring layer-removal; gRPC baseline decomposition | HotNets/HotOS workshop |
| Y1 Q3-Q4 | T1, T2 | NVMe + GPU characterization; DSA memcpy in gRPC; crossover maps | — |
| Y2 Q1-Q2 | T1, T2 | Cross-domain regime change paper; QAT TLS + IAA in gRPC | OSDI/SOSP submission |
| Y2 Q3-Q4 | T2, T3 | Adaptive offload policy; ns-regime stdexec extensions | — |
| Y3 Q1-Q2 | T2, T3 | gRPC accelerator transport beta; multi-accelerator pipeline | NSDI submission |
| Y3 Q3-Q4 | T3, T4 | UCX transport plugin; EventEngine ns-regime prototype | — |
| Y4 Q1-Q2 | T3, T4 | End-to-end benchmarks (DeathStarBench); on-die vs SmartNIC comparison | ASPLOS submission |
| Y4 Q3-Q4 | T4 | CXL + DSA characterization; open benchmark release | NSDI/SIGCOMM submission |
| Y5 | All | Production release; upstream contributions; standards proposals | — |

### Risk Mitigations

- DSA framework (our preliminary results) de-risks all thrusts by proving the measurement methodology works
- Mock hardware methodology allows progress when real hardware access is limited
- Layer-removal is framework-agnostic — works on gRPC, DPDK, io_uring without modifying internals
- Multi-threading fallback: if per-core optimization hits diminishing returns, multi-core scaling remains viable

---

## 8. Expected Outcomes and Impact

### 8.1 Technical Outcomes

- **Cross-domain characterization**: First systematic measurement showing software frameworks become the bottleneck once hardware submission is batched, across DSA, RDMA, io_uring, and NVMe
- **gRPC latency decomposition**: First rigorous decomposition with per-accelerator crossover maps
- **Framework design principles**: Concrete guidelines (operation pooling, cache-conscious metadata, O(1) completion) applicable to stdexec, gRPC EventEngine, and other async frameworks
- **Accelerator-native transports**: Open-source gRPC and UCX implementations on commodity Xeon

### 8.2 Community Impact

**Reframing the framework design conversation**: The systems community has optimized hardware paths while treating software frameworks as fixed overhead. Our work shows batching has inverted this. This reframing should influence how the community designs async frameworks, completion mechanisms, and submission APIs.

**Completing the intra-host picture**: The receive path (NIC-to-host) is characterized by prior work. We characterize the offload path (host-to-accelerator). Together, the full intra-host data movement pipeline is measured. Directly relevant to the "Your Network Doesn't End at the NIC" vision.

**Bridging hardware and software solutions**: CXL-NIC (hardware) and our batching (software) are complementary solutions to the same MMIO bottleneck. Our characterization informs hardware architects about what software can and cannot solve.

### 8.3 Standards and Ecosystem

- **C++ P2300 (stdexec)**: First empirical characterization of P2300 cost on real hardware. Framework redesign findings inform WG21 proposals for operation pools and lightweight combinators
- **Open-source**: All framework code, benchmarks, and transport implementations released

---

## 9. Related Work

### Host Network Characterization
- "Understanding the Host Network" [SIGCOMM 2024] — receive path characterization
- hostCC [SIGCOMM 2023] — host congestion control
- ZeroNIC [OSDI 2024] — FPGA NIC data/control separation
- CXL-NIC [MICRO 2025] — MMIO replacement with CXL coherence
- "Your Network Doesn't End at the NIC" [HotNets 2025]

### RPC Acceleration
- RPCAcc [arXiv 2024] — PCIe FPGA RPC accelerator with DSA
- RpcNIC [HPCA 2025] — SmartNIC-native RPC
- Arcalis [arXiv 2026] — Near-cache RPC accelerator
- Cornflakes [SOSP 2023] — Zero-copy serialization via NIC scatter-gather

### Intel Accelerators
- DSA Characterization [ASPLOS 2024] — Kuper et al., definitive performance study
- IAA for Databases [ADMS@VLDB 2025] — Up to 3.15x faster decompression
- QAT TLS — 51-69% improvement in TLS connection rates

### Async Frameworks
- P2300R10 (stdexec) adopted into C++26
- eRPC [NSDI 2019] — kernel-bypass datacenter RPCs
- io_uring zero-copy Rx [Linux 6.15] — saturates 200G from single core

---

## 10. References

[1] Vuppalapati, Agarwal, Schuh, Kasikci, Krishnamurthy, Agarwal. "Understanding the Host Network." SIGCOMM 2024.
[2] Agarwal et al. "hostCC." SIGCOMM 2023.
[3] Agarwal et al. "ZeroNIC." OSDI 2024.
[4] Agarwal et al. "CXL-NIC." MICRO 2025.
[5] Agarwal et al. "Your Network Doesn't End at the NIC." HotNets 2025.
[6] RPCAcc. arXiv:2411.07632, 2024.
[7] RpcNIC. HPCA 2025.
[8] Arcalis. arXiv:2602.12596, 2026.
[9] Margaritov et al. "Cornflakes." SOSP 2023.
[10] Kuper et al. "Quantitative Analysis of DSA." ASPLOS 2024.
[11] P2300R10. "std::execution." C++26.
[12] Gan et al. "DeathStarBench." ASPLOS 2019.
[13] Kalia et al. "eRPC." NSDI 2019.
[14] "Efficient Zero-Copy Networking using io_uring." Kernel Recipes 2024.
[15] "A Hot Take on Intel IAA for DBMS." ADMS@VLDB 2025.
[16] "A Wake-Up Call for Kernel-Bypass on Modern Hardware." DAMON 2025.

---

*This document synthesizes findings from the DSA-stdexec research prototype. Raw data and detailed analysis available in `docs/report/`, `remark/`, and the benchmark suite.*
