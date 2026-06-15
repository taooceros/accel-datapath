# Shenango: Achieving High CPU Efficiency for Latency-sensitive Datacenter Workloads

## Paper identity

- **Canonical title**: Shenango: Achieving High CPU Efficiency for Latency-sensitive Datacenter Workloads
- **Venue / year**: NSDI 2019
- **Repo pass confidence**: first-pass folder backfill from the curated literature report set
- **Source URL**: https://www.usenix.org/system/files/nsdi19-ousterhout.pdf

## Why it matters here

Shenango is a counterpoint to pure poll-mode dedication: it tries to preserve microsecond-scale latency while greatly improving CPU efficiency through a dedicated IOKernel and very fast core reallocation. In this repo's literature stack, it is important because it shows that low-latency datapaths do not end the runtime-policy question.

## Problem

This paper is part of the broader literature-report corpus already summarized in `003.async_runtime_2026-03-28.md`. This folder backfill makes the paper itself locally navigable through `paper.pdf` and searchable through `paper.md`, so the repo no longer depends only on the cross-paper report for grounding.

## Method / mechanism

- **IOKernel**: run a privileged component on a dedicated core to steer packets and manage resource decisions.
- **Fine-grained core reallocation**: reassign cores at very short intervals instead of dedicating them permanently.
- **Demand detection**: detect quickly when a latency-sensitive app needs more cores.
- **Decoupled steering and execution**: keep low-latency I/O while avoiding permanent spin-poll core dedication.

## Evaluation setup

- local source PDF copied from `papers/top_tier_pdfs/` into this paper folder
- first-pass curated interpretation lives in `003.async_runtime_2026-03-28.md`
- this pass prioritizes a stable local paper folder and searchable markdown extraction over a fresh deep reread

## Key findings

- Shenango is a counterpoint to pure poll-mode dedication: it tries to preserve microsecond-scale latency while greatly improving CPU efficiency through a dedicated IOKernel and very fast core reallocation. In this repo's literature stack, it is important because it shows that low-latency datapaths do not end the runtime-policy question.
- **RQ2**: medium support for runtime-level redesign beyond the hot loop itself
- **RQ3**: medium support for RPC-path evaluation under realistic CPU-efficiency constraints
- **RQ4**: medium evidence that runtime policy remains visible at small timescales

## Limits and confidence

- first-pass backfill from curated literature reports rather than a fully rewritten paper note
- local `paper.md` is intended as searchable grounding and may retain PDF extraction artifacts
- Shenango focuses on scheduling and CPU efficiency, not the finer-grained submission/completion overhead decomposition that the DSA work measures directly.

## Repo takeaways

- this folder is the stable paper-specific home; the numbered literature reports remain the cross-paper synthesis layer
- use `paper.md` for direct textual grounding and the source literature report for the current repo-oriented interpretation

## Source links within repo

- local paper artifacts: [`paper.pdf`](./paper.pdf), [`paper.md`](./paper.md)
- [`../../003.async_runtime_2026-03-28.md`](../../003.async_runtime_2026-03-28.md)
- [`../../002.top_tier_index_2026-03-28.md`](../../002.top_tier_index_2026-03-28.md)
- [`../../006.question_correlation_matrix_2026-03-28.md`](../../006.question_correlation_matrix_2026-03-28.md)
- [`../../../../../current.md`](../../../../../current.md)
