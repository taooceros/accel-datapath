# Async framework and completion overhead

This topic groups the strongest prior work on queue structure, polling, completions, and runtime design. It is the closest literature base for the repo's claim that abstraction and completion policy can become visible once hardware is fast enough.

## Direct matches

- **MegaPipe** — 2012, **OSDI**. A strong submission/completion API paper, especially for request batching and event aggregation.
- **IX** — 2014, **OSDI**. A direct fast-path and completion-policy paper for poll-mode design.
- **Shenango: Achieving High CPU Efficiency for Latency-sensitive Datacenter Workloads** — 2019, **NSDI**. Important for showing that low-latency poll-mode execution has real CPU-efficiency tradeoffs.
- **Caladan: Mitigating Interference at Microsecond Timescales** — 2020, **OSDI**. Strong evidence that runtime and scheduling policy still matter at very small timescales.
- **Hermes: Enhancing Layer-7 Cloud Load Balancers with Userspace-Directed I/O Event Notification** — 2025, **SIGCOMM**. Strong recent evidence that userspace-visible worker state and event-notification policy can drive better dispatch than fixed kernel heuristics alone.
- **Shinjuku: Preemptive Scheduling for μsecond-scale Tail Latency** — 2019, **NSDI**. Useful counterpoint to run-to-completion and bounded-batch designs: it shows where preemption becomes the right runtime tool once microsecond tail latency matters more than pure polling efficiency.

## Key contrast paper

- **Datacenter RPCs can be General and Fast (eRPC)** — 2019, **NSDI**. The most useful contrast: abstraction is not necessarily too expensive, but high performance requires an aggressively specialized fast path that preserves locality, batching, and cheap completions.

## Why these matter here

The repo's strongest internal claim is not merely that DSA is fast; it is that once DSA is fast enough, the **software stack above it** becomes the bottleneck. These papers justify that framing by showing that scheduling, queue partitioning, batching, event delivery, userspace-guided dispatch, and preemption policy are often the real determinants of throughput and latency after lower-level overhead is reduced.

## Gap relative to this repo

Most prior work here stops at kernel bypass, runtime structure, or RPC fast paths. The repo pushes one layer higher by asking what happens when a composable async framework sits on top of a batched accelerator and its own overhead becomes directly measurable.
