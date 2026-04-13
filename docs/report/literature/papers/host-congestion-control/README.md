# Host Congestion Control

## Paper identity

- **Canonical title**: Host Congestion Control
- **Venue / year**: SIGCOMM 2023
- **Repo pass confidence**: first-pass folder backfill from the curated literature report set
- **Source URL**: https://www.cs.cornell.edu/~ragarwal/pubs/hostcc.pdf

## Why it matters here

hostCC argues that congestion is not only a network-fabric phenomenon and introduces host-local congestion signals plus a host-local response policy. In this repo, it matters as further evidence that control-path policy inside the host can dominate observed performance.

## Problem

This paper is part of the broader literature-report corpus already summarized in `005.accelerator_hostpath_2026-03-28.md`. This folder backfill makes the paper itself locally navigable through `paper.pdf` and searchable through `paper.md`, so the repo no longer depends only on the cross-paper report for grounding.

## Method / mechanism

- **Host-local congestion signals**: measure congestion inside the host network, not just in the external fabric.
- **Sub-RTT host-local response**: react faster than RTT-scale network control when host resources are congested.
- **Two-level control architecture**: combine host-resource allocation with network-resource allocation.
- **Protocol-compatible integration**: insert host-congestion handling into the Linux stack without changing applications or NIC hardware.

## Evaluation setup

- local source PDF copied from `papers/top_tier_pdfs/` into this paper folder
- first-pass curated interpretation lives in `005.accelerator_hostpath_2026-03-28.md`
- this pass prioritizes a stable local paper folder and searchable markdown extraction over a fresh deep reread

## Key findings

- hostCC argues that congestion is not only a network-fabric phenomenon and introduces host-local congestion signals plus a host-local response policy. In this repo, it matters as further evidence that control-path policy inside the host can dominate observed performance.
- **RQ1**: medium support for host-path bottleneck awareness
- **RQ4**: medium support for the repo's broader host-to-accelerator framing

## Limits and confidence

- first-pass backfill from curated literature reports rather than a fully rewritten paper note
- local `paper.md` is intended as searchable grounding and may retain PDF extraction artifacts
- hostCC is about congestion response rather than accelerator abstraction or framework cost. It is therefore a positioning and systems-context paper more than a direct mechanism source.

## Repo takeaways

- this folder is the stable paper-specific home; the numbered literature reports remain the cross-paper synthesis layer
- use `paper.md` for direct textual grounding and the source literature report for the current repo-oriented interpretation

## Source links within repo

- local paper artifacts: [`paper.pdf`](./paper.pdf), [`paper.md`](./paper.md)
- [`../../005.accelerator_hostpath_2026-03-28.md`](../../005.accelerator_hostpath_2026-03-28.md)
- [`../../002.top_tier_index_2026-03-28.md`](../../002.top_tier_index_2026-03-28.md)
- [`../../006.question_correlation_matrix_2026-03-28.md`](../../006.question_correlation_matrix_2026-03-28.md)
- [`../../../../../current.md`](../../../../../current.md)
