# mTCP: a Highly Scalable User-level TCP Stack for Multicore Systems

## Paper identity

- **Canonical title**: mTCP: a Highly Scalable User-level TCP Stack for Multicore Systems
- **Venue / year**: NSDI 2014
- **Repo pass confidence**: first-pass folder backfill from the curated literature report set
- **Source URL**: https://www.usenix.org/system/files/conference/nsdi14/nsdi14-paper-jeong.pdf

## Why it matters here

mTCP argues that short-connection workloads are dominated by software-stack cost and attacks that problem with user-level networking, event aggregation, and batched I/O. The paper is useful here as evidence that high-rate request workloads expose software overhead before raw hardware limits are reached.

## Problem

This paper is part of the broader literature-report corpus already summarized in `003.async_runtime_2026-03-28.md`. This folder backfill makes the paper itself locally navigable through `paper.pdf` and searchable through `paper.md`, so the repo no longer depends only on the cross-paper report for grounding.

## Method / mechanism

- **User-level TCP stack**: remove much of kernel TCP processing from the hot path.
- **Shared-memory application interface**: replace many expensive syscalls with shared-memory interactions.
- **Flow-level event aggregation**: aggregate notification-path work across flows.
- **Batched packet I/O**: send and receive packets in batches to improve multicore I/O efficiency.

## Evaluation setup

- local source PDF copied from `papers/top_tier_pdfs/` into this paper folder
- first-pass curated interpretation lives in `003.async_runtime_2026-03-28.md`
- this pass prioritizes a stable local paper folder and searchable markdown extraction over a fresh deep reread

## Key findings

- mTCP argues that short-connection workloads are dominated by software-stack cost and attacks that problem with user-level networking, event aggregation, and batched I/O. The paper is useful here as evidence that high-rate request workloads expose software overhead before raw hardware limits are reached.
- **RQ2**: medium support for reducing aggregation and event-path overhead
- **RQ3**: medium support for why request-heavy RPC decomposition matters
- **RQ4**: strong cross-domain evidence that software costs dominate in short-request regimes

## Limits and confidence

- first-pass backfill from curated literature reports rather than a fully rewritten paper note
- local `paper.md` is intended as searchable grounding and may retain PDF extraction artifacts
- The paper is about a multicore TCP stack, not composable async accelerators or offload-policy crossover analysis. Its conclusions transfer as motivation and analogy more than as a drop-in design.

## Repo takeaways

- this folder is the stable paper-specific home; the numbered literature reports remain the cross-paper synthesis layer
- use `paper.md` for direct textual grounding and the source literature report for the current repo-oriented interpretation

## Source links within repo

- local paper artifacts: [`paper.pdf`](./paper.pdf), [`paper.md`](./paper.md)
- [`../../003.async_runtime_2026-03-28.md`](../../003.async_runtime_2026-03-28.md)
- [`../../002.top_tier_index_2026-03-28.md`](../../002.top_tier_index_2026-03-28.md)
- [`../../006.question_correlation_matrix_2026-03-28.md`](../../006.question_correlation_matrix_2026-03-28.md)
- [`../../../../../current.md`](../../../../../current.md)
