# Arrakis: The Operating System is the Control Plane

## Paper identity

- **Canonical title**: Arrakis: The Operating System is the Control Plane
- **Venue / year**: OSDI 2014
- **Repo pass confidence**: first-pass folder backfill from the curated literature report set
- **Source URL**: https://www.usenix.org/system/files/conference/osdi14/osdi14-paper-peter_simon.pdf

## Why it matters here

Arrakis splits control and data-plane responsibilities so applications can access virtualized I/O devices directly while the kernel retains protection and resource control. For this repo, it is a strong “generality without full mediation” baseline and a reminder that abstraction cost depends on where the control boundary is drawn.

## Problem

This paper is part of the broader literature-report corpus already summarized in `003.async_runtime_2026-03-28.md`. This folder backfill makes the paper itself locally navigable through `paper.pdf` and searchable through `paper.md`, so the repo no longer depends only on the cross-paper report for grounding.

## Method / mechanism

- **Control-plane / data-plane split**: keep protection and resource management in the kernel while moving fast-path data operations out of it.
- **Direct access to virtualized I/O devices**: expose hardware more directly to applications without full kernel mediation per operation.
- **Hot-path mediation removal**: retain safety guarantees while taking the kernel out of each fast-path operation.
- **Virtualization for safe direct access**: use virtualization to reconcile direct hardware access with isolation.

## Evaluation setup

- local source PDF copied from `papers/top_tier_pdfs/` into this paper folder
- first-pass curated interpretation lives in `003.async_runtime_2026-03-28.md`
- this pass prioritizes a stable local paper folder and searchable markdown extraction over a fresh deep reread

## Key findings

- Arrakis splits control and data-plane responsibilities so applications can access virtualized I/O devices directly while the kernel retains protection and resource control. For this repo, it is a strong “generality without full mediation” baseline and a reminder that abstraction cost depends on where the control boundary is drawn.
- **RQ2**: medium evidence for redesigning the control boundary around the fast path
- **RQ4**: medium evidence that the same tension appears outside the DSA setting

## Limits and confidence

- first-pass backfill from curated literature reports rather than a fully rewritten paper note
- local `paper.md` is intended as searchable grounding and may retain PDF extraction artifacts
- Arrakis is not about accelerator submission or async framework overhead on batched hardware. It is more useful as a positioning paper than as a detailed mechanism source.

## Repo takeaways

- this folder is the stable paper-specific home; the numbered literature reports remain the cross-paper synthesis layer
- use `paper.md` for direct textual grounding and the source literature report for the current repo-oriented interpretation

## Source links within repo

- local paper artifacts: [`paper.pdf`](./paper.pdf), [`paper.md`](./paper.md)
- [`../../003.async_runtime_2026-03-28.md`](../../003.async_runtime_2026-03-28.md)
- [`../../002.top_tier_index_2026-03-28.md`](../../002.top_tier_index_2026-03-28.md)
- [`../../006.question_correlation_matrix_2026-03-28.md`](../../006.question_correlation_matrix_2026-03-28.md)
- [`../../../../../current.md`](../../../../../current.md)
