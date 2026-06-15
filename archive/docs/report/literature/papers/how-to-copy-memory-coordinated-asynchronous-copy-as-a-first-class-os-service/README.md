# How to Copy Memory? Coordinated Asynchronous Copy as a First-Class OS Service

## Paper identity

- **Canonical title**: How to Copy Memory? Coordinated Asynchronous Copy as a First-Class OS Service
- **Venue / year**: SOSP 2025
- **Repo pass confidence**: first-pass folder backfill from the curated literature report set
- **Source URL**: https://ipads.se.sjtu.edu.cn/_media/pub/sosp25-copier-preprint.pdf

## Why it matters here

Copier treats copy as a first-class asynchronous OS service with overlap, hardware-capability use, and global optimization opportunities. In this repo's literature stack, it is a strong abstraction precedent for elevating copy offload above a narrow device API.

## Problem

This paper is part of the broader literature-report corpus already summarized in `005.accelerator_hostpath_2026-03-28.md`. This folder backfill makes the paper itself locally navigable through `paper.pdf` and searchable through `paper.md`, so the repo no longer depends only on the cross-paper report for grounding.

## Method / mechanism

- **Copy as a first-class OS service**: make copy a coordinated system service rather than just a library call.
- **Asynchronous copy with `amemcpy` / `csync`**: overlap computation with copy while preserving use-point synchronization semantics.
- **Queue-based segmented copy pipeline**: track per-segment completion and allow early consumption of ready pieces.
- **Piggybacked heterogeneous dispatcher + copy absorption**: coordinate AVX and DMA copy units and eliminate redundant intermediate copies.

## Evaluation setup

- local source PDF copied from `papers/top_tier_pdfs/` into this paper folder
- first-pass curated interpretation lives in `005.accelerator_hostpath_2026-03-28.md`
- this pass prioritizes a stable local paper folder and searchable markdown extraction over a fresh deep reread

## Key findings

- Copier treats copy as a first-class asynchronous OS service with overlap, hardware-capability use, and global optimization opportunities. In this repo's literature stack, it is a strong abstraction precedent for elevating copy offload above a narrow device API.
- **RQ1**: medium support for improving the practical DSA path
- **RQ2**: strong support for designing higher-level async copy abstractions
- **RQ3**: medium relevance where copy dominates RPC paths
- **RQ5**: medium support for integrating copy into larger deployment-facing systems

## Limits and confidence

- first-pass backfill from curated literature reports rather than a fully rewritten paper note
- local `paper.md` is intended as searchable grounding and may retain PDF extraction artifacts
- Copier is OS-service-centric and copy-specific. It does not address the broader multi-stage async pipeline of copy, checksum, compression, and transport integration that the repo is after.

## Repo takeaways

- this folder is the stable paper-specific home; the numbered literature reports remain the cross-paper synthesis layer
- use `paper.md` for direct textual grounding and the source literature report for the current repo-oriented interpretation

## Source links within repo

- local paper artifacts: [`paper.pdf`](./paper.pdf), [`paper.md`](./paper.md)
- [`../../005.accelerator_hostpath_2026-03-28.md`](../../005.accelerator_hostpath_2026-03-28.md)
- [`../../002.top_tier_index_2026-03-28.md`](../../002.top_tier_index_2026-03-28.md)
- [`../../006.question_correlation_matrix_2026-03-28.md`](../../006.question_correlation_matrix_2026-03-28.md)
- [`../../../../../current.md`](../../../../../current.md)
