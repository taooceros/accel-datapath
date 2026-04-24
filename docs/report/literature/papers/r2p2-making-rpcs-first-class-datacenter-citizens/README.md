# R2P2: Making RPCs first-class datacenter citizens

## Paper identity

- **Canonical title**: R2P2: Making RPCs first-class datacenter citizens
- **Venue / year**: USENIX ATC 2019
- **Repo pass confidence**: first-pass folder backfill from the curated literature report set
- **Source URL**: https://www.usenix.org/system/files/atc19-kogias-r2p2_0.pdf

## Why it matters here

R2P2 argues that the transport should optimize directly for RPC request/response behavior instead of treating RPC as an afterthought on top of a generic byte stream. The key value here is conceptual: it treats RPC as the first-class unit that scheduling, routing, and transport design should expose.

## Problem

This paper is part of the broader literature-report corpus already summarized in `004.rpc_transport_2026-03-28.md`. This folder backfill makes the paper itself locally navigable through `paper.pdf` and searchable through `paper.md`, so the repo no longer depends only on the cross-paper report for grounding.

## Method / mechanism

- **Request-Response Pair Protocol**: make the request/response pair the first-class transport abstraction.
- **Separation of target selection from streaming**: choose the RPC target independently from request and reply streaming.
- **In-network RPC routing**: let a router or switch participate directly in RPC scheduling and load balancing.
- **JBSQ split-queue scheduling**: centralize pending RPCs while bounding per-server outstanding work.

## Evaluation setup

- local source PDF copied from `papers/top_tier_pdfs/` into this paper folder
- first-pass curated interpretation lives in `004.rpc_transport_2026-03-28.md`
- this pass prioritizes a stable local paper folder and searchable markdown extraction over a fresh deep reread

## Key findings

- R2P2 argues that the transport should optimize directly for RPC request/response behavior instead of treating RPC as an afterthought on top of a generic byte stream. The key value here is conceptual: it treats RPC as the first-class unit that scheduling, routing, and transport design should expose.
- **RQ3**: strong relevance to end-to-end RPC decomposition
- **RQ4**: medium support for the idea that control structure should match the operation being optimized

## Limits and confidence

- first-pass backfill from curated literature reports rather than a fully rewritten paper note
- local `paper.md` is intended as searchable grounding and may retain PDF extraction artifacts
- R2P2 is a transport redesign paper, not a drop-in accelerator-assisted runtime. It is more directly relevant to transport semantics than to the repo's current on-die offload path.

## Repo takeaways

- this folder is the stable paper-specific home; the numbered literature reports remain the cross-paper synthesis layer
- use `paper.md` for direct textual grounding and the source literature report for the current repo-oriented interpretation

## Source links within repo

- local paper artifacts: [`paper.pdf`](./paper.pdf), [`paper.md`](./paper.md)
- [`../../004.rpc_transport_2026-03-28.md`](../../004.rpc_transport_2026-03-28.md)
- [`../../002.top_tier_index_2026-03-28.md`](../../002.top_tier_index_2026-03-28.md)
- [`../../006.question_correlation_matrix_2026-03-28.md`](../../006.question_correlation_matrix_2026-03-28.md)
- [`../../../../../current.md`](../../../../../current.md)
