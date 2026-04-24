# DSA-2LM: A CPU-Free Tiered Memory Architecture with Intel DSA

## Paper identity

- **Canonical title**: DSA-2LM: A CPU-Free Tiered Memory Architecture with Intel DSA
- **Venue / year**: USENIX ATC 2025
- **Repo pass confidence**: first-pass folder backfill from the curated literature report set
- **Source URL**: https://www.usenix.org/system/files/atc25-liu-ruili.pdf

## Why it matters here

DSA-2LM is one of the closest direct systems precedents for this repo because it embeds Intel DSA into a higher-level memory policy rather than treating DSA as a standalone primitive. Its main value here is to show that DSA can produce end-to-end system benefit when the surrounding control policy is designed around it.

## Problem

This paper is part of the broader literature-report corpus already summarized in `005.accelerator_hostpath_2026-03-28.md`. This folder backfill makes the paper itself locally navigable through `paper.pdf` and searchable through `paper.md`, so the repo no longer depends only on the cross-paper report for grounding.

## Method / mechanism

- **Direct in-kernel DSA page migration**: bypass the generic DMA interface and invoke DSA directly for page migration.
- **Adaptable concurrent migration**: handle both 4 KB and 2 MB migration in one mixed-page path.
- **Batch + multi-work-queue scheduling**: batch small-page copies and split huge pages across multiple DSA work queues.
- **Threshold-driven aggressive migration loop**: shorten migration intervals because copy work is offloaded.

## Evaluation setup

- local source PDF copied from `papers/top_tier_pdfs/` into this paper folder
- first-pass curated interpretation lives in `005.accelerator_hostpath_2026-03-28.md`
- this pass prioritizes a stable local paper folder and searchable markdown extraction over a fresh deep reread

## Key findings

- DSA-2LM is one of the closest direct systems precedents for this repo because it embeds Intel DSA into a higher-level memory policy rather than treating DSA as a standalone primitive. Its main value here is to show that DSA can produce end-to-end system benefit when the surrounding control policy is designed around it.
- **RQ1**: strong evidence that DSA matters in a real higher-level path
- **RQ2**: medium evidence that policy and framework structure must surround the hardware
- **RQ5**: medium support for deployment-facing accelerator integration

## Limits and confidence

- first-pass backfill from curated literature reports rather than a fully rewritten paper note
- local `paper.md` is intended as searchable grounding and may retain PDF extraction artifacts
- DSA-2LM is about tiered-memory page migration, not composable async abstractions or per-operation framework overhead. It is a direct accelerator paper, but not yet a direct framework paper.

## Repo takeaways

- this folder is the stable paper-specific home; the numbered literature reports remain the cross-paper synthesis layer
- use `paper.md` for direct textual grounding and the source literature report for the current repo-oriented interpretation

## Source links within repo

- local paper artifacts: [`paper.pdf`](./paper.pdf), [`paper.md`](./paper.md)
- [`../../005.accelerator_hostpath_2026-03-28.md`](../../005.accelerator_hostpath_2026-03-28.md)
- [`../../002.top_tier_index_2026-03-28.md`](../../002.top_tier_index_2026-03-28.md)
- [`../../006.question_correlation_matrix_2026-03-28.md`](../../006.question_correlation_matrix_2026-03-28.md)
- [`../../../../../current.md`](../../../../../current.md)
