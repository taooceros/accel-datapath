# Analytical Per-Phase Cost Models Over-Predicted Savings by 4x

**Date**: 2026-02-22
**Source**: `report/progress_post_alignment_debug.md`, Section 8

## Finding

An analytical decomposition of per-operation cost into 8 phases predicted
~11 ns/op of reducible overhead. Three optimizations targeting those phases
actually saved ~2-3 ns/op.

## What went wrong

| Optimization | Predicted | Actual | Why wrong |
|---|---|---|---|
| Proxy -> fn pointers | 4 ns | ~1 ns | Proxy SBO was already cheap; dispatch cost similar |
| Indexed queue | 3-5 ns | ~0 ns (mock) | 100% completion rate = no wasted traversal |
| SlotArena free-list | 3 ns | ~1 ns | Only helps at c=4096 cache boundary |

The analytical model reasoned about code structure ("this does a placement
new of 384 bytes, that must cost ~9 ns") without accounting for:
- Compiler optimizations (inlining, dead code elimination)
- Microarchitectural effects (OoO execution hiding latencies)
- Interaction effects between phases (hot instruction cache from tight loop)

## Lesson

For performance work: **measure, don't model**. Layer-removal experiments
(remark #002) gave accurate numbers. Analytical decomposition is useful for
generating hypotheses, not for predicting savings.

The total was approximately right (~34-37 ns summed vs ~38 ns measured), but
the attribution to individual phases was wrong. This is the classic
"right answer for wrong reasons" problem in performance analysis.
