# RPCAcc: A High-Performance and Reconfigurable PCIe-attached RPC Accelerator

## Paper identity

- **Canonical title**: RPCAcc: A High-Performance and Reconfigurable PCIe-attached RPC Accelerator
- **Venue / year**: arXiv 2024
- **Repo pass confidence**: first-pass folder backfill from the curated literature report set
- **Source URL**: https://arxiv.org/pdf/2411.07632

## Why it matters here

RPCAcc is the most direct hardware-assisted RPC comparison in the current paper set: it co-designs parts of the RPC stack with a PCIe-attached accelerator and explicitly targets serialization and traversal overhead inside the RPC path. For this repo, it is the clearest evidence that the RPC stack itself is now a plausible hardware/software co-design target.

## Problem

This paper is part of the broader literature-report corpus already summarized in `004.rpc_transport_2026-03-28.md`. This folder backfill makes the paper itself locally navigable through `paper.pdf` and searchable through `paper.md`, so the repo no longer depends only on the cross-paper report for grounding.

## Method / mechanism

- **PCIe-attached on-NIC RPC acceleration**: offload parts of the RPC stack to a deployable NIC/FPGA accelerator.
- **Target-aware deserializer**: batch field writes in accelerator SRAM and cross PCIe only when needed.
- **Memory-affinity collaborative serializer**: split serialization work between CPU and accelerator to avoid pointer-chasing over PCIe.
- **Automatic field-update / schema adaptation**: adapt field placement based on which RPC kernels are offloaded.

## Evaluation setup

- local source PDF copied from `papers/top_tier_pdfs/` into this paper folder
- first-pass curated interpretation lives in `004.rpc_transport_2026-03-28.md`
- this pass prioritizes a stable local paper folder and searchable markdown extraction over a fresh deep reread

## Key findings

- RPCAcc is the most direct hardware-assisted RPC comparison in the current paper set: it co-designs parts of the RPC stack with a PCIe-attached accelerator and explicitly targets serialization and traversal overhead inside the RPC path. For this repo, it is the clearest evidence that the RPC stack itself is now a plausible hardware/software co-design target.
- **RQ2**: medium support for redesigning framework boundaries around accelerator capabilities
- **RQ3**: strong direct comparison for RPC decomposition and offload benefit
- **RQ5**: strong precedent for deployment-facing accelerator-assisted RPC integration

## Limits and confidence

- first-pass backfill from curated literature reports rather than a fully rewritten paper note
- local `paper.md` is intended as searchable grounding and may retain PDF extraction artifacts
- RPCAcc is on-NIC and PCIe-attached rather than on-die, and it is not framed around Rust `tonic` or a composable async runtime. The transport and hardware assumptions are therefore close, but not identical.

## Repo takeaways

- this folder is the stable paper-specific home; the numbered literature reports remain the cross-paper synthesis layer
- use `paper.md` for direct textual grounding and the source literature report for the current repo-oriented interpretation

## Source links within repo

- local paper artifacts: [`paper.pdf`](./paper.pdf), [`paper.md`](./paper.md)
- [`../../004.rpc_transport_2026-03-28.md`](../../004.rpc_transport_2026-03-28.md)
- [`../../002.top_tier_index_2026-03-28.md`](../../002.top_tier_index_2026-03-28.md)
- [`../../006.question_correlation_matrix_2026-03-28.md`](../../006.question_correlation_matrix_2026-03-28.md)
- [`../../../../../current.md`](../../../../../current.md)
