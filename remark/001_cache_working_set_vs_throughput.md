# Cache Working Set Determines Throughput Ceiling at High Concurrency

**Date**: 2026-02-22
**Source**: `report/progress_post_alignment_debug.md`, Section 4 "Cache Working Set Analysis"

## Finding

Mock DSA throughput degrades 25-40% as concurrency increases from 32 to 4096,
even though hardware latency is zero. The cause is cache pressure from
per-operation slot data structures.

## Key data

| Concurrency | `reusable` Mpps | Working set | Cache level |
|---|---|---|---|
| 32 | 84 | 12 KB | L1d (48 KB) |
| 1024 | 62.5 | 384 KB | L2 (2 MB) |
| 2048 | 59.9 | 768 KB | L2 |
| 4096 | 61.3 | 1536 KB | L2/L3 boundary |

The L1 -> L2 transition (c=32 -> c=1024) accounts for +4.1 ns/op, consistent
with L2 hit latency (~4-5 ns) replacing L1 hits (~1 ns) on Sapphire Rapids.

## Why it matters

- The 84 Mpps at c=32 is the true software floor -- not 60 Mpps.
- At realistic concurrency (1024+), ~30% of per-op time is cache misses.
- Slot sizes (384-512 bytes, 5-8 cache lines each) are dominated by the
  over-allocated DsaOperationBase (320 bytes) needed for 64-byte descriptor
  alignment.

## Implication

To improve throughput at high concurrency without changing the algorithm:
shrink slot size (hot/cold field separation) or improve access pattern
(prefetch next slot during current submit).
