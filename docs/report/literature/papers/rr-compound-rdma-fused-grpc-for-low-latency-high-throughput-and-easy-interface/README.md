# RR-Compound: RDMA-Fused gRPC for Low Latency, High Throughput, and Easy Interface

## Paper identity

- **Canonical title**: RR-Compound: RDMA-Fused gRPC for Low Latency, High Throughput, and Easy Interface
- **Venue / year noted in local artifacts**: TPDS 2024 metadata path
- **Repo pass confidence**: lower-confidence, metadata-grounded

## Why it matters here

RR-Compound stays in the active Tonic literature set because it is unusually direct to the compatibility-first design question: can a system keep the gRPC interface while replacing the transport path with an RDMA-aware fast path?

## Problem

The paper's role in the current repo flow is to represent an API-preserving gRPC-over-RDMA design point. That makes it useful for design-space context even though the current repo pass does not have strong full-paper grounding.

## Method / mechanism

Only a limited mechanism story is safely grounded in the local artifacts:

1. a fully compatible, drop-in gRPC framing goal,
2. an RDMA-enabled internal fast path,
3. runtime-tunable transport knobs visible from the artifact repository noted in the local sources.

## Evaluation setup

The current repo grounding is metadata and artifact-repository level rather than full directly read paper-level evaluation. This folder should not be read as a strong evaluation summary.

## Key findings

At the current confidence level, only qualitative or artifact-configuration details are safe:

1. compatibility-first gRPC-over-RDMA framing is the central design goal
2. RDMA is disabled by default in the artifact configuration noted by the repo
3. one polling thread is the default noted in the repo
4. the local notes record a `500 μs` busy-poll timeout and a `4096 KB` ring buffer per connection

## Limits and confidence

- weakest paper in the seeded set
- current repo pass is stronger on metadata and artifact configuration than on full paper ingestion
- keep this page qualitative until a stronger direct paper pass is added later
- this folder now includes a local `paper.pdf` and a fallback `paper.txt`, but the text artifact is only a partial/incomplete fallback rather than a normal full extraction because the available CLI extractors repeatedly timed out on this IEEE manuscript

## Repo takeaways

- include RR-Compound as design-space context for compatibility-preserving transport replacement
- do not treat it as a strong quantitative anchor in the current Tonic literature flow
- keep its claims clearly separated from the stronger Cloud-Scale, Protocol Buffers, TF-gRPC-Bench, and RPCAcc evidence

## Source links within repo

- local paper artifacts: [`paper.pdf`](./paper.pdf), [`paper.txt`](./paper.txt), [`paper.md`](./paper.md)
- [`../../007.grpc_cost_breakdown_2026-04-12.md`](../../007.grpc_cost_breakdown_2026-04-12.md)
- [`../../008.paper_module_rebuild_analysis.md`](../../008.paper_module_rebuild_analysis.md)
- [`../../../../related_work/04_rpc_acceleration_transports.md`](../../../../related_work/04_rpc_acceleration_transports.md)
- [`../../../../../current.md`](../../../../../current.md)
