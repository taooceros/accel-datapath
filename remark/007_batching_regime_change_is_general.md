# The Batching Regime Change Is Not DSA-Specific

**Date**: 2026-02-23
**Source**: Conversation analysis during research plan revision

## Finding

Batching amortizes MMIO doorbell (or equivalent submission) overhead from
hundreds of nanoseconds to single-digit nanoseconds per operation. This creates
a regime change where software framework overhead — previously invisible at
μs-scale hardware latency — becomes the dominant cost. This phenomenon is
general across all batched hardware submission systems, not specific to DSA.

## Evidence across domains

| System | Unbatched overhead | Batched overhead | Framework overhead |
|---|---|---|---|
| DSA (this work) | ~160 ns/doorbell | ~5 ns/op (batch=32) | 21 ns (stdexec) |
| RDMA NIC (Mellanox) | ~150-500 ns/doorbell | ~5-10 ns/op (BlueFlame) | ibverbs abstraction |
| io_uring | ~syscall per op | ~10-20 ns/op (multi-SQE) | liburing wrappers |
| NVMe | ~doorbell per cmd | ~5-15 ns/op (cmd batching) | command construction |
| GPU (CUDA) | ~launch overhead | ~50-100 ns/op (graph) | CUDA runtime |

In every case, once batching is applied, the hardware is no longer the
bottleneck. The software that prepares, submits, and completes operations is.

## Key insight

The HotOS-level observation: **batching doesn't just amortize hardware cost —
it shifts the bottleneck from hardware to software, exposing a class of
software overhead that was previously invisible.** This is a structural change
in the performance landscape, not an incremental improvement.

## Why it matters

- Our DSA results are not a curiosity about one accelerator; they are a
  specific instance of a general phenomenon
- The layer-removal methodology applies to all these domains
- The scheduling/submission separation (remark #004) is a universal design
  principle, not a DSA-specific optimization
- Framework redesign for the ns-regime is a cross-domain research agenda

## What triggered this insight

The user pointed out that "it doesn't have to be on-die accelerators" and
referenced a HotOS paper showing MMIO can be faster without memory fences.
Batching amortizes latency generally; the regime change happens whenever
amortized hardware cost drops below framework overhead.
