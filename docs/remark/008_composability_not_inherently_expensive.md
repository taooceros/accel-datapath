# Composability Is Not Inherently Expensive — Frameworks Were Never Optimized for ns-Scale

**Date**: 2026-02-23
**Source**: Conversation analysis during research plan revision

## Finding

The 21 ns/op stdexec overhead is NOT evidence that composable async
abstractions are inherently expensive. It is evidence that stdexec was designed
for μs-scale I/O (disk, network, timers) where 21 ns is noise, and was never
optimized for the ns-regime created by batched hardware submission.

## The reasoning

When stdexec was designed:
- Typical I/O: disk read (~10 ms), TCP send (~10-100 μs), timer (~ms)
- Framework overhead at 20-40 ns: **<0.01%** of total operation time
- Reasonable design choice: prioritize safety, composability, generality

After batching:
- Typical batched accelerator op: ~5 ns amortized hardware
- Framework overhead at 20-40 ns: **70-85%** of total operation time
- Now: framework overhead is the dominant cost

The framework didn't get slower. The hardware got faster (via batching), and
the framework was never re-examined for the new regime.

## Specific non-fundamental costs in stdexec

| Cost | ns | Why it exists | Why it's not fundamental |
|---|---|---|---|
| scope.nest() | ~8 ns | Structured concurrency tracking | Could be compile-time for static graphs |
| then() adapter | ~6 ns | Generic continuation chaining | Could be specialized/inlined |
| connect() | ~5 ns | 448B operation state allocation | Could be pooled/reused |
| start() | ~2 ns | Per-op initialization | Could be reset-in-place |

Our `reusable` strategy already proves this: by pre-allocating and reusing
operation states (bypassing connect/start), we reach 84 Mpps — without
changing the fundamental programming model.

## Why this framing matters for the grant

The research question is NOT "composability vs. performance" (a well-trodden
and uninteresting tradeoff). The research question IS: **can we redesign
composable frameworks for the ns-regime without sacrificing their safety and
expressiveness?** This is a framework engineering challenge, not a fundamental
limitation.

This reframing transforms the project from "measuring a known tradeoff" into
"identifying a design gap created by a regime change and filling it."

## What triggered this insight

The user challenged: "I feel like composability does not mean we have to
sacrifice performance, is it?" This forced re-examination of whether stdexec's
overhead reflects inherent cost or historical design assumptions.
