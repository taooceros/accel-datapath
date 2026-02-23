# Real DSA Exhibits a Bistable Throughput Regime

**Date**: 2026-02-22
**Source**: `report/progress_post_alignment_debug.md`, Section 4 "Bistable Feedback Loop"

## Finding

The same benchmark configuration on real DSA can produce either ~20 Mpps or
~10 Mpps depending on initial conditions (cache warmth, OS scheduling). This
bistability is absent from mock DSA.

## Mechanism

A positive feedback loop between poll scan cost and hardware completion rate:

1. Fewer completions per poll -> more wasted O(N) scan time
2. More scan time -> longer inter-submission gap
3. Longer gap -> smaller effective batch size
4. Smaller batch -> fewer completions per poll (back to step 1)

The `arena` strategy (which changes memory access patterns) falls into the
low-throughput state more often than `noalloc`, despite being algorithmically
identical (O(1) vs O(N) scan). This confirms the root cause is a
hardware-software interaction, not purely algorithmic.

## Why it matters

- Mock-hardware methodology is essential: it eliminates this feedback loop
  and gives reproducible measurements.
- An O(1) completion mechanism (interrupts, completion bitmap, event-driven
  notification) would break the loop.
- Benchmarking real DSA requires multiple runs and careful reporting of
  which regime was observed.

## Evidence

`arena` on real DSA: 10-12 Mpps at c=256/1024 (low regime)
`noalloc` on real DSA: 15-18 Mpps at c=256/1024 (high regime)
Both are ~26-27 Mpps on mock DSA (no bistability).
