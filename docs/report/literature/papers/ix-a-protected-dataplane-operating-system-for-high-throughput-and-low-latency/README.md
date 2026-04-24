# IX: A Protected Dataplane Operating System for High Throughput and Low Latency

## Paper identity

- **Canonical title**: IX: A Protected Dataplane Operating System for High Throughput and Low Latency
- **Venue / year**: OSDI 2014
- **Repo pass confidence**: first-pass folder backfill from the curated literature report set
- **Source URL**: https://www.usenix.org/system/files/conference/osdi14/osdi14-paper-belay.pdf

## Why it matters here

IX is a protected user-space dataplane design that gets high throughput and low latency by giving dedicated threads and queues to dataplane instances, processing bounded batches to completion, and removing coherence-heavy cross-core interaction. For this repo, it is a direct fast-path analogue for poll-mode execution, queue ownership, and bounded work on the hot path.

## Problem

This paper is part of the broader literature-report corpus already summarized in `003.async_runtime_2026-03-28.md`. This folder backfill makes the paper itself locally navigable through `paper.pdf` and searchable through `paper.md`, so the repo no longer depends only on the cross-paper report for grounding.

## Method / mechanism

- **Protected user-space dataplane**: move the fast path into user space while retaining isolation and protection.
- **Dedicated threads and queues**: assign hardware threads and NIC queues to each dataplane instance.
- **Bounded batching to completion**: process bounded batches, then run them to completion on the hot path.
- **Coherence avoidance**: minimize cross-core sharing and synchronization so coherence traffic does not dominate.

## Evaluation setup

- local source PDF copied from `papers/top_tier_pdfs/` into this paper folder
- first-pass curated interpretation lives in `003.async_runtime_2026-03-28.md`
- this pass prioritizes a stable local paper folder and searchable markdown extraction over a fresh deep reread

## Key findings

- IX is a protected user-space dataplane design that gets high throughput and low latency by giving dedicated threads and queues to dataplane instances, processing bounded batches to completion, and removing coherence-heavy cross-core interaction. For this repo, it is a direct fast-path analogue for poll-mode execution, queue ownership, and bounded work on the hot path.
- **RQ2**: strong fast-path design evidence for the nanosecond regime
- **RQ4**: strong network-domain analogue for queue ownership and bounded batching

## Limits and confidence

- first-pass backfill from curated literature reports rather than a fully rewritten paper note
- local `paper.md` is intended as searchable grounding and may retain PDF extraction artifacts
- IX is a NIC dataplane paper, not an accelerator framework paper. It says little about structured async composition, operation-state overhead, or accelerator-specific completion mechanisms.

## Repo takeaways

- this folder is the stable paper-specific home; the numbered literature reports remain the cross-paper synthesis layer
- use `paper.md` for direct textual grounding and the source literature report for the current repo-oriented interpretation

## Source links within repo

- local paper artifacts: [`paper.pdf`](./paper.pdf), [`paper.md`](./paper.md)
- [`../../003.async_runtime_2026-03-28.md`](../../003.async_runtime_2026-03-28.md)
- [`../../002.top_tier_index_2026-03-28.md`](../../002.top_tier_index_2026-03-28.md)
- [`../../006.question_correlation_matrix_2026-03-28.md`](../../006.question_correlation_matrix_2026-03-28.md)
- [`../../../../../current.md`](../../../../../current.md)
