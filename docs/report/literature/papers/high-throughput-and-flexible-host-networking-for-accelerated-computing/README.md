# High-throughput and Flexible Host Networking for Accelerated Computing (ZeroNIC)

## Paper identity

- **Canonical title**: High-throughput and Flexible Host Networking for Accelerated Computing (ZeroNIC)
- **Venue / year**: OSDI 2024
- **Repo pass confidence**: first-pass folder backfill from the curated literature report set
- **Source URL**: https://www.usenix.org/system/files/osdi24-skiadopoulos.pdf

## Why it matters here

ZeroNIC separates data and control paths to deliver both throughput and flexibility in host networking for accelerated computing. For this repo, the paper is valuable less as a direct DSA reference than as an architectural analogy for keeping the hot data path lean while reserving richer control logic for a colder path.

## Problem

This paper is part of the broader literature-report corpus already summarized in `005.accelerator_hostpath_2026-03-28.md`. This folder backfill makes the paper itself locally navigable through `paper.pdf` and searchable through `paper.md`, so the repo no longer depends only on the cross-paper report for grounding.

## Method / mechanism

- **Physical separation of data and control paths**: keep payload transfer and protocol logic on different paths.
- **Header/payload split-merge pipeline**: DMA payloads directly while processing headers separately.
- **Per-flow memory-segment bookkeeping**: maintain enough mapping state to place data correctly under reordering and retransmission.
- **Protocol-agnostic zero-copy endpoint design**: keep the fast path independent of whether buffers live in CPU, GPU, FPGA, or another memory domain.

## Evaluation setup

- local source PDF copied from `papers/top_tier_pdfs/` into this paper folder
- first-pass curated interpretation lives in `005.accelerator_hostpath_2026-03-28.md`
- this pass prioritizes a stable local paper folder and searchable markdown extraction over a fresh deep reread

## Key findings

- ZeroNIC separates data and control paths to deliver both throughput and flexibility in host networking for accelerated computing. For this repo, the paper is valuable less as a direct DSA reference than as an architectural analogy for keeping the hot data path lean while reserving richer control logic for a colder path.
- **RQ2**: medium support for separating hot-path and control-path responsibilities
- **RQ3**: medium relevance to accelerator-aware RPC deployment
- **RQ5**: strong architectural analogue for deployment-facing accelerator transports

## Limits and confidence

- first-pass backfill from curated literature reports rather than a fully rewritten paper note
- local `paper.md` is intended as searchable grounding and may retain PDF extraction artifacts
- ZeroNIC uses custom NIC co-design rather than commodity on-die accelerators. It is therefore adjacent architecture rather than a direct implementation template.

## Repo takeaways

- this folder is the stable paper-specific home; the numbered literature reports remain the cross-paper synthesis layer
- use `paper.md` for direct textual grounding and the source literature report for the current repo-oriented interpretation

## Source links within repo

- local paper artifacts: [`paper.pdf`](./paper.pdf), [`paper.md`](./paper.md)
- [`../../005.accelerator_hostpath_2026-03-28.md`](../../005.accelerator_hostpath_2026-03-28.md)
- [`../../002.top_tier_index_2026-03-28.md`](../../002.top_tier_index_2026-03-28.md)
- [`../../006.question_correlation_matrix_2026-03-28.md`](../../006.question_correlation_matrix_2026-03-28.md)
- [`../../../../../current.md`](../../../../../current.md)
