# Transparent Auto-Batching Decouples Scheduling from Submission

**Date**: 2026-02-22
**Source**: `report/progress_post_alignment_debug.md`, Section 6

## Finding

Auto-batching via descriptor submitter strategies (MirroredRing) provides
1.2-2x throughput gain while being completely transparent to the scheduling
layer. The application submits one descriptor at a time; batching happens
underneath.

## Key design property

Scheduling strategies (sliding window, batch, scoped worker) are completely
unaware of batching. The same code runs identically whether the submission
backend is:
- Immediate: 1 doorbell per descriptor (~6 Mpps for 8-byte messages)
- MirroredRing: 1 doorbell per 32 descriptors (~18-35 Mpps)

## Why it matters

- Demonstrates that hardware batch optimization can be an implementation
  detail, not an API concern.
- Directly applicable to UCX/OpenSHMEM: transport backends submit individual
  RMA operations and get batch amortization for free.
- The ablation study (double-buffered vs fixed-ring vs mirrored-ring) shows
  **in-flight batch depth** (16 vs 2) is the dominant factor, not ring
  buffer cleverness.

## The MirroredRing trick

`memfd_create` + dual `mmap` creates a virtual-memory-aliased ring buffer
where the second half of address space maps to the same physical pages as
the first half. Descriptors can be written as a contiguous burst across
the wrap boundary without special-case logic. This is simpler and faster
than modular arithmetic on every write.
