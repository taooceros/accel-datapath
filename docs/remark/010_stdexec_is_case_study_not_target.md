# stdexec Is the Case Study, Not the Target Framework

**Date**: 2026-02-23
**Source**: Conversation analysis during research plan revision

## Finding

gRPC is written in C/C++ and uses its own async machinery (EventEngine), not
stdexec. Our stdexec + DSA work is a **case study** that validated the
layer-removal methodology and quantified the batching regime change. It is
not the framework that production systems (gRPC, UCX, DPDK) would use.

## Why this matters for positioning

The research contribution is the **insight and methodology**, not the specific
stdexec implementation:

1. **Layer-removal methodology**: Works on any layered system. Doesn't require
   framework internals — just build progressively-stripped variants and measure
   end-to-end deltas. Applies to gRPC EventEngine, DPDK rte_ethdev, io_uring
   liburing, etc.

2. **Batching regime change**: A structural observation about hardware
   amortization that applies universally. Any framework sitting above batched
   hardware submission will exhibit the same pattern.

3. **Framework design principles for ns-regime**: Operation pooling,
   cache-conscious metadata, O(1) completion — these apply to EventEngine,
   not just stdexec.

## The stdexec→EventEngine translation

| stdexec concept | gRPC EventEngine analog |
|---|---|
| scope.nest() (14 ns) | Closure capture + event registration |
| connect() (5 ns) | Callback allocation + state setup |
| start() (2 ns) | Operation initiation |
| TaskQueue poll | EventEngine poller |

The specific numbers differ but the structural overhead categories are the
same. If EventEngine has similar per-closure overhead (likely, since it uses
heap-allocated callbacks), the batching regime change applies equally.

## Implication for the grant

- Lead with methodology and insight (general)
- Present stdexec data as the proof-of-concept (specific)
- Propose applying methodology to production frameworks (gRPC, UCX) as the
  research deliverable
- Don't claim stdexec will replace EventEngine — claim our measurement
  techniques and design principles will improve EventEngine

## What triggered this insight

The user asked "what is gRPC written in?" — forcing recognition that gRPC
doesn't use stdexec and therefore our stdexec implementation is a research
vehicle, not a deployment target.
