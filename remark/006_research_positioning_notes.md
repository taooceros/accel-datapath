# Research Positioning: Saksham Agarwal's Work and Our Differentiation

**Date**: 2026-02-23
**Purpose**: Memory file for research plan revision

Historical note:
Corrected by: docs/report/literature/005.accelerator_hostpath_2026-03-28.md
Why: This note keeps the host-to-accelerator positioning insight, but report `005` corrects the earlier certainty that a paper literally titled `CXL-NIC` was confirmed.

## Saksham Agarwal's Research Arc (key papers)

1. **HotNets 2022**: Host interconnect congestion identified (IOMMU, memory bus)
2. **SIGCOMM 2023 (hostCC)**: Host congestion control — controls *rate* of operations entering host interconnect
3. **SIGCOMM 2024 (Understanding Host Network)**: Credit-based domain model; copies >50% CPU at 100G; 49% cache misses
4. **OSDI 2024 (ZeroNIC)**: Data/control path separation → 17% CPU at 100G (vs 50% Linux TCP). FPGA NIC co-design.
5. **HotOS 2025 best-match candidate (`My CXL Pool Obviates Your PCIe Switch`)**: the current repo authority treats this as the best public match for the earlier `CXL-NIC` thread rather than a paper literally titled `CXL-NIC`; keep the MMIO/interconnect bottleneck takeaway here.
6. **HotNets 2025**: "Your Network Doesn't End at the NIC" — intra-host network needs co-design with inter-host
7. **SOSP 2024**: IOMMU overhead up to 60%. Can be near-eliminated with better memory management.

## What Saksham's work establishes that we build on

- Host CPU is the bottleneck at 100G+ (not network)
- Data copies dominate CPU cycles
- MMIO writes (PCIe) are expensive primitives
- Data/control path separation is a productive design principle

## What Saksham's work does NOT cover (our gap)

- Per-operation cost of driving on-die accelerators through software
- Composable async framework overhead measured on real hardware
- Crossover analysis: when does accelerator offload beat CPU?
- Bistable throughput regime in poll-mode accelerator loops
- Mock hardware methodology for isolating framework overhead

## Critical connection: earlier `CXL-NIC` thread and the current best-match candidate

The current repo literature record points to `My CXL Pool Obviates Your PCIe Switch` as the best public candidate for the earlier `CXL-NIC` thread, not a paper literally titled `CXL-NIC`.
Our batching amortizes MMIO doorbells for DSA — same problem, software solution.
The CXL-backed comparison lane remains the hardware/interconnect-side complement.

This means: **batching (our approach) and the CXL-backed hardware/interconnect lane are two solutions
to the same MMIO bottleneck**. Our work characterizes the software side; the later literature-backed candidate captures the hardware-side comparison. Together they bound the design space.

## Key insight for positioning

Saksham's arc: NIC hardware fast → host software slow → measure → control → redesign

Our arc should be: On-die accelerator hardware fast (when batched) → framework software
slow → measure → understand the threshold → redesign frameworks for ns-regime

The parallel is structural but the domain is different:
- Saksham: NIC → PCIe → host interconnect → memory (the receive path)
- Us: CPU → framework → descriptor → MMIO doorbell → accelerator → memory (the offload path)

Both hit the same physical bottlenecks (MMIO, cache misses, memory bandwidth) but
from different directions. His solutions are at the interconnect/congestion-control
level. Ours are at the software framework/batching level.

## Open question raised by this analysis

ZeroNIC achieves 17% CPU at 100G via data/control path separation on a *custom NIC*.
Can on-die accelerators (DSA, QAT, IAA) achieve similar CPU savings with *commodity
hardware*? This is a compelling comparison for the grant:
- ZeroNIC: custom FPGA NIC, $$$, single-vendor
- Our approach: commodity Xeon with on-die accelerators, works today
- Same principle (separate data/control paths) but different realization

## How this changes the research plan

The plan should NOT position us as "doing what Saksham did but for accelerators."
That's too derivative. Instead:

The plan should position us as **completing the picture** — Saksham measured the
NIC-to-host path; we measure the host-to-accelerator path. Together, the full
intra-host data movement pipeline is characterized. The batching insight and the
composability question are our unique angles that don't exist in his work.

The earlier `CXL-NIC` thread is also valuable: our software batching and the later CXL
hardware replacement are complementary solutions to the same MMIO bottleneck.
A combined approach (the CXL-backed candidate lane + DSA batching) could eliminate MMIO overhead entirely.
