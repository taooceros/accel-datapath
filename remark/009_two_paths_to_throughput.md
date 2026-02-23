# Two Paths to Maximum Throughput: More Cores vs. Better Batching

**Date**: 2026-02-23
**Source**: Conversation analysis during research plan revision

## Finding

There are two orthogonal paths to maximizing aggregate accelerator throughput:

1. **More cores / more work queues**: Scale horizontally. Each core runs the
   same framework, submits to its own WQ. Linear scaling up to device limits.

2. **Better batching + framework optimization**: Scale per-core efficiency.
   Reduce software overhead so each core feeds hardware faster, enabling
   larger effective batches and better hardware utilization.

These are orthogonal, not competing. Multi-threading is a scaling mechanism;
per-core optimization is an efficiency mechanism.

## Why per-core optimization matters even when you can add cores

1. **Power efficiency**: 1 core at 34 Mpps vs 4 cores at 34 Mpps (8.5 Mpps
   each) is a 4x difference in power for the same work. At datacenter scale,
   power is the binding constraint.

2. **Offload crossover point**: Each core independently decides CPU-vs-
   accelerator per operation. If framework overhead is 38 ns but CPU memcpy
   is 3 ns, offload only pays off above ~4-8 KB. Reducing framework overhead
   to 12 ns lowers this crossover, making offload viable for smaller messages
   — more of the workload benefits from accelerators.

3. **Batching quality**: Per-core software overhead determines how fast
   descriptors fill the batch buffer. Slower software → longer fill time →
   hardware sits idle between batches. There's a positive feedback loop:
   less overhead → tighter batching → better hardware utilization → higher
   throughput per core.

4. **Shared resource contention**: More cores means more contention on shared
   resources (LLC, memory bandwidth, accelerator shared WQs). Per-core
   efficiency reduces the number of cores needed, reducing contention.

## The non-obvious connection to batching

Batching is what makes per-core optimization matter. Without batching, each
operation costs ~160 ns of MMIO overhead, and 21 ns of framework overhead is
noise (13%). With batching, hardware drops to 5 ns and framework at 21 ns is
the majority (80%). Batching creates the regime where per-core optimization
has 4-6x headroom.

So the two paths interact: batching enables per-core optimization to matter,
and per-core optimization enables batching to work better (tighter fill rate).

## What triggered this insight

The user asked: "One trivial solution is to use multiple threads; how will
you defend that?" This forced articulation of why per-core efficiency matters
independently of horizontal scaling.
