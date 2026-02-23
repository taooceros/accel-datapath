# MMIO Bottleneck Has Both Software and Hardware Solutions — They're Complementary

**Date**: 2026-02-23
**Source**: Conversation analysis + Saksham Agarwal positioning (remark #006)

## Finding

MMIO doorbell writes are the fundamental bottleneck for hardware submission
on current Intel platforms. Two independent research lines address this from
opposite sides:

| Approach | Who | Direction | Solution | Result |
|---|---|---|---|---|
| CXL-NIC [MICRO 2025] | Agarwal et al. | Hardware | Replace MMIO with CXL coherence messages | 49% latency reduction |
| This work | Us | Software | Batch descriptors behind one MMIO doorbell | ~5 ns/op amortized (vs ~160 ns unbatched) |

## Why they're complementary, not competing

**Software batching** reduces the *number* of MMIO doorbells (from N to N/32).
**CXL coherence** reduces the *cost* of each doorbell (from ~160 ns to ~80 ns).

Combined: fewer doorbells AND each doorbell is cheaper. The multiplicative
effect could make submission overhead negligible:
- Current: N × 160 ns = 160 ns/op
- Batching only: (N/32) × 160 ns ≈ 5 ns/op
- CXL only: N × 80 ns = 80 ns/op
- Both: (N/32) × 80 ns ≈ 2.5 ns/op

At 2.5 ns/op submission overhead, even framework overhead at 10 ns would be
the clear bottleneck — further strengthening the case for ns-regime framework
design.

## Broader pattern

This is an instance of a common systems pattern: hardware improvements don't
eliminate the need for software optimization; they shift the bottleneck and
make software optimization matter *more*. CXL-NIC doesn't make batching
unnecessary — it makes the framework overhead (which batching exposes) even
more dominant.

## Connection to research positioning

Saksham's group measures and fixes the hardware submission path (MMIO → CXL).
We measure and fix the software path above the submission mechanism (framework
→ descriptor → doorbell). Together, both paths are characterized and optimized.
Neither group's work subsumes the other.

## What triggered this insight

Analyzing CXL-NIC [MICRO 2025] while positioning against Saksham Agarwal's
research arc. The user asked how our work differs from Arvind's group's host
network characterization, leading to systematic comparison that revealed the
complementary relationship.
