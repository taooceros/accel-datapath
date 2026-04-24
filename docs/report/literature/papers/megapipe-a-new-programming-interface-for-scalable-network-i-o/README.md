# MegaPipe: A New Programming Interface for Scalable Network I/O

## Paper identity

- **Canonical title**: MegaPipe: A New Programming Interface for Scalable Network I/O
- **Venue / year**: OSDI 2012
- **Repo pass confidence**: first-pass folder backfill from the curated literature report set
- **Source URL**: https://www.usenix.org/system/files/conference/osdi12/osdi12-final-40.pdf

## Why it matters here

MegaPipe redesigns network I/O around a per-core channel abstraction and explicitly makes partitioning, lightweight endpoints, and batching first-class for message-oriented workloads. In this repo's terms, it is an early API-level argument that submission and completion structure can be a dominant systems bottleneck, not just a low-level implementation detail.

## Problem

This paper is part of the broader literature-report corpus already summarized in `003.async_runtime_2026-03-28.md`. This folder backfill makes the paper itself locally navigable through `paper.pdf` and searchable through `paper.md`, so the repo no longer depends only on the cross-paper report for grounding.

## Method / mechanism

- **Per-core channel abstraction**: use a per-core bidirectional kernel↔user-space channel for both requests and event notifications.
- **Partitioning**: assign work and connection ownership explicitly across cores to preserve locality.
- **Lightweight socket (`lwsocket`)**: use a cheaper endpoint abstraction than a traditional socket for message-oriented workloads.
- **Batching**: group request submission and event delivery to amortize per-operation overhead.

## Evaluation setup

- local source PDF copied from `papers/top_tier_pdfs/` into this paper folder
- first-pass curated interpretation lives in `003.async_runtime_2026-03-28.md`
- this pass prioritizes a stable local paper folder and searchable markdown extraction over a fresh deep reread

## Key findings

- MegaPipe redesigns network I/O around a per-core channel abstraction and explicitly makes partitioning, lightweight endpoints, and batching first-class for message-oriented workloads. In this repo's terms, it is an early API-level argument that submission and completion structure can be a dominant systems bottleneck, not just a low-level implementation detail.
- **RQ2**: supports framework redesign around lighter submission/completion paths
- **RQ4**: evidence that batching and queue ownership generalize beyond DSA

## Limits and confidence

- first-pass backfill from curated literature reports rather than a fully rewritten paper note
- local `paper.md` is intended as searchable grounding and may retain PDF extraction artifacts
- MegaPipe is still a network-I/O API paper rather than an accelerator paper. It does not address on-die accelerators, composable async abstractions on top of batched hardware, or the nanosecond-scale framework-overhead decomposition that drives this repo.

## Repo takeaways

- this folder is the stable paper-specific home; the numbered literature reports remain the cross-paper synthesis layer
- use `paper.md` for direct textual grounding and the source literature report for the current repo-oriented interpretation

## Source links within repo

- local paper artifacts: [`paper.pdf`](./paper.pdf), [`paper.md`](./paper.md)
- [`../../003.async_runtime_2026-03-28.md`](../../003.async_runtime_2026-03-28.md)
- [`../../002.top_tier_index_2026-03-28.md`](../../002.top_tier_index_2026-03-28.md)
- [`../../006.question_correlation_matrix_2026-03-28.md`](../../006.question_correlation_matrix_2026-03-28.md)
- [`../../../../../current.md`](../../../../../current.md)
