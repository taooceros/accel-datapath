# My CXL Pool Obviates Your PCIe Switch

## Paper identity

- **Canonical title**: My CXL Pool Obviates Your PCIe Switch
- **Venue / year**: HotOS 2025
- **Repo pass confidence**: first-pass folder backfill from the curated literature report set
- **Source URL**: https://arxiv.org/pdf/2503.23611

## Why it matters here

This paper broadens the story from accelerator micro-mechanisms to interconnect organization by arguing that PCIe-device pooling can be implemented in software using CXL memory pools. In the repo's literature review, it serves as a bridge to the MMIO, interconnect, and disaggregation side of the argument.

## Problem

This paper is part of the broader literature-report corpus already summarized in `005.accelerator_hostpath_2026-03-28.md`. This folder backfill makes the paper itself locally navigable through `paper.pdf` and searchable through `paper.md`, so the repo no longer depends only on the cross-paper report for grounding.

## Method / mechanism

- **Software PCIe pooling on top of CXL memory pools**: use a CXL pod as the substrate for PCIe device pooling.
- **CXL-resident I/O buffers**: place TX/RX or device I/O buffers in shared CXL memory.
- **Software coherence for non-coherent shared CXL memory**: manage coherence explicitly because current pooled memory is not cross-host coherent.
- **Pooling orchestrator + agents**: use a control plane to map devices to hosts, monitor health, and rebalance or fail over usage.

## Evaluation setup

- local source PDF copied from `papers/top_tier_pdfs/` into this paper folder
- first-pass curated interpretation lives in `005.accelerator_hostpath_2026-03-28.md`
- this pass prioritizes a stable local paper folder and searchable markdown extraction over a fresh deep reread

## Key findings

- This paper broadens the story from accelerator micro-mechanisms to interconnect organization by arguing that PCIe-device pooling can be implemented in software using CXL memory pools. In the repo's literature review, it serves as a bridge to the MMIO, interconnect, and disaggregation side of the argument.
- **RQ4**: medium support for the broader generalization and positioning story
- **RQ5**: medium relevance to deployment paths involving disaggregated or pooled devices

## Limits and confidence

- first-pass backfill from curated literature reports rather than a fully rewritten paper note
- local `paper.md` is intended as searchable grounding and may retain PDF extraction artifacts
- It is not directly a DSA, RPC, or accelerator-framework paper, and the title remains only a best-match candidate for the earlier CXL-NIC thread. Its role here is therefore contextual rather than central.

## Repo takeaways

- this folder is the stable paper-specific home; the numbered literature reports remain the cross-paper synthesis layer
- use `paper.md` for direct textual grounding and the source literature report for the current repo-oriented interpretation

## Source links within repo

- local paper artifacts: [`paper.pdf`](./paper.pdf), [`paper.md`](./paper.md)
- [`../../005.accelerator_hostpath_2026-03-28.md`](../../005.accelerator_hostpath_2026-03-28.md)
- [`../../002.top_tier_index_2026-03-28.md`](../../002.top_tier_index_2026-03-28.md)
- [`../../006.question_correlation_matrix_2026-03-28.md`](../../006.question_correlation_matrix_2026-03-28.md)
- [`../../../../../current.md`](../../../../../current.md)
