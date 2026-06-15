# FaSST: Fast, Scalable and Simple Distributed Transactions with Two-Sided (RDMA) Datagram RPCs

## Paper identity

- **Canonical title**: FaSST: Fast, Scalable and Simple Distributed Transactions with Two-Sided (RDMA) Datagram RPCs
- **Venue / year**: OSDI 2016
- **Repo pass confidence**: first-pass folder backfill from the curated literature report set
- **Source URL**: https://www.usenix.org/system/files/conference/osdi16/osdi16-kalia.pdf

## Why it matters here

FaSST makes the case that fast RPCs over modern RDMA networks should favor two-sided datagram RPCs instead of more complex one-sided reliability machinery. In this repo's literature stack, it is the strongest RDMA-side reminder that batching and a simple request/response model can outperform more elaborate transport semantics.

## Problem

This paper is part of the broader literature-report corpus already summarized in `004.rpc_transport_2026-03-28.md`. This folder backfill makes the paper itself locally navigable through `paper.pdf` and searchable through `paper.md`, so the repo no longer depends only on the cross-paper report for grounding.

## Method / mechanism

- **Two-sided unreliable-datagram RDMA RPCs**: use RPCs over RDMA datagrams instead of one-sided remote-memory access.
- **Server-side traversal instead of client flattening**: let the server CPU traverse remote data structures rather than forcing client-side one-sided access patterns.
- **Datagram transport for scalability**: avoid the queue-pair scaling issues of connection-oriented one-sided designs.
- **Doorbell batching**: batch multiple RPC requests or responses behind fewer NIC doorbells.

## Evaluation setup

- local source PDF copied from `papers/top_tier_pdfs/` into this paper folder
- first-pass curated interpretation lives in `004.rpc_transport_2026-03-28.md`
- this pass prioritizes a stable local paper folder and searchable markdown extraction over a fresh deep reread

## Key findings

- FaSST makes the case that fast RPCs over modern RDMA networks should favor two-sided datagram RPCs instead of more complex one-sided reliability machinery. In this repo's literature stack, it is the strongest RDMA-side reminder that batching and a simple request/response model can outperform more elaborate transport semantics.
- **RQ3**: strong RDMA-side baseline for fast RPC decomposition
- **RQ4**: medium evidence that batching and simple request semantics generalize beyond Ethernet software stacks

## Limits and confidence

- first-pass backfill from curated literature reports rather than a fully rewritten paper note
- local `paper.md` is intended as searchable grounding and may retain PDF extraction artifacts
- FaSST is still a transport paper rather than an async-runtime-plus-accelerator paper. It informs the transport edge of the design space, not the on-die offload machinery.

## Repo takeaways

- this folder is the stable paper-specific home; the numbered literature reports remain the cross-paper synthesis layer
- use `paper.md` for direct textual grounding and the source literature report for the current repo-oriented interpretation

## Source links within repo

- local paper artifacts: [`paper.pdf`](./paper.pdf), [`paper.md`](./paper.md)
- [`../../004.rpc_transport_2026-03-28.md`](../../004.rpc_transport_2026-03-28.md)
- [`../../002.top_tier_index_2026-03-28.md`](../../002.top_tier_index_2026-03-28.md)
- [`../../006.question_correlation_matrix_2026-03-28.md`](../../006.question_correlation_matrix_2026-03-28.md)
- [`../../../../../current.md`](../../../../../current.md)
