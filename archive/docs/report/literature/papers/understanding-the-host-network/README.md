# Understanding the Host Network

## Paper identity

- **Canonical title**: Understanding the Host Network
- **Venue / year**: SIGCOMM 2024
- **Repo pass confidence**: first-pass folder backfill from the curated literature report set
- **Source URL**: https://www.cs.cornell.edu/~ragarwal/pubs/understanding-the-host-network.pdf

## Why it matters here

Understanding the Host Network reframes the host as a network of contention domains and gives a credit-based way to reason about data movement inside the machine. For this repo, it is the main external justification for treating the host-to-accelerator path as a first-class systems bottleneck rather than just a local implementation detail.

## Problem

This paper is part of the broader literature-report corpus already summarized in `005.accelerator_hostpath_2026-03-28.md`. This folder backfill makes the paper itself locally navigable through `paper.pdf` and searchable through `paper.md`, so the repo no longer depends only on the cross-paper report for grounding.

## Method / mechanism

- **Host-network abstraction**: model the host as a network of processor, memory, and peripheral interconnects.
- **Domain-by-domain credit-based flow control**: analyze the host through contention domains with their own credits and latency.
- **Blue/red contention regime analysis**: distinguish cases where host contention harms different traffic classes differently.
- **Latency-throughput domain model**: explain host-path limits using flow-control style reasoning instead of only raw bandwidth numbers.

## Evaluation setup

- local source PDF copied from `papers/top_tier_pdfs/` into this paper folder
- first-pass curated interpretation lives in `005.accelerator_hostpath_2026-03-28.md`
- this pass prioritizes a stable local paper folder and searchable markdown extraction over a fresh deep reread

## Key findings

- Understanding the Host Network reframes the host as a network of contention domains and gives a credit-based way to reason about data movement inside the machine. For this repo, it is the main external justification for treating the host-to-accelerator path as a first-class systems bottleneck rather than just a local implementation detail.
- **RQ1**: medium support for why host-path bottlenecks matter to measured DSA throughput
- **RQ4**: strong support for the repo's host-path generalization and positioning

## Limits and confidence

- first-pass backfill from curated literature reports rather than a fully rewritten paper note
- local `paper.md` is intended as searchable grounding and may retain PDF extraction artifacts
- It is primarily a characterization and conceptual-model paper. It does not directly propose a DSA-oriented framework or RPC data path.

## Repo takeaways

- this folder is the stable paper-specific home; the numbered literature reports remain the cross-paper synthesis layer
- use `paper.md` for direct textual grounding and the source literature report for the current repo-oriented interpretation

## Source links within repo

- local paper artifacts: [`paper.pdf`](./paper.pdf), [`paper.md`](./paper.md)
- [`../../005.accelerator_hostpath_2026-03-28.md`](../../005.accelerator_hostpath_2026-03-28.md)
- [`../../002.top_tier_index_2026-03-28.md`](../../002.top_tier_index_2026-03-28.md)
- [`../../006.question_correlation_matrix_2026-03-28.md`](../../006.question_correlation_matrix_2026-03-28.md)
- [`../../../../../current.md`](../../../../../current.md)
