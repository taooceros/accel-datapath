# A Cloud-Scale Characterization of Remote Procedure Calls

## Paper identity

- **Canonical title**: A Cloud-Scale Characterization of Remote Procedure Calls
- **Venue / year**: SOSP 2023
- **Authors noted in local artifacts**: Korakit Seemakhupt et al.
- **Repo pass confidence**: strong for the decomposition structure, production-study scope, and the main fleet-level findings used in the active Tonic review flow

## Why it matters here

This is the anchor paper for the queueing, stack-tax, wire, and tail-latency parts of the active Tonic characterization story. The repo uses it to avoid collapsing all non-application time into a vague network bucket.

## Problem

The paper asks how modern cloud RPC latency should be decomposed in production systems. The core challenge is that end-to-end latency can hide queueing, RPC-stack work, and wire contribution inside one average number.

## Method / mechanism

This is a measurement paper, not a mechanism paper. Its central move is a decomposition:

1. separate application processing from RPC latency tax,
2. then decompose that tax into request and response queueing, RPC processing plus network-stack work, and network-wire time,
3. while also keeping CPU-tax contributors visible enough for stack cost, serialization, networking, and compression to stay distinct.

## Evaluation setup

- fleet-scale observational study in an internal Google RPC production environment
- more than `10,000` methods and more than `1B` traces in the study shape captured by local repo artifacts
- sampled every 30 minutes across nearly two years
- compares methods, services, clusters, latency components, and CPU-tax views rather than only one lab microbenchmark

## Key findings

1. fleet-wide average RPC tax can look modest, but high-overhead methods and tail-heavy methods are much more tax-dominated
2. queueing and RPC-stack work need to stay visible instead of being folded into generic network delay
3. storage-heavy services dominate large parts of the traffic and transferred-byte budget in the summary captured by the repo
4. the slowest methods consume disproportionate total RPC time relative to their request fraction

## Limits and confidence

- strong source for decomposition vocabulary and production-study scope
- not a direct gRPC or Tonic paper
- descriptive rather than prescriptive, so it does not identify Rust async instrumentation points on its own

## Repo takeaways

- keep runtime and queueing separate from payload-path cost
- keep wire contribution separate from stack tax
- treat tail behavior as a decomposition problem, not just a p99 headline
- use this paper to justify bucketed Tonic measurement, not to claim that Google-stack percentages transfer directly to Tonic

## Source links within repo

- local paper artifacts: [`paper.pdf`](./paper.pdf), [`paper.txt`](./paper.txt), [`paper.md`](./paper.md)
- [`../../007.grpc_cost_breakdown_2026-04-12.md`](../../007.grpc_cost_breakdown_2026-04-12.md)
- [`../../008.paper_module_rebuild_analysis.md`](../../008.paper_module_rebuild_analysis.md)
- [`../../../../related_work/04_rpc_acceleration_transports.md`](../../../../related_work/04_rpc_acceleration_transports.md)
- [`../../../../../current.md`](../../../../../current.md)
