# Caladan: Mitigating Interference at Microsecond Timescales

## Paper identity

- **Canonical title**: Caladan: Mitigating Interference at Microsecond Timescales
- **Venue / year**: OSDI 2020
- **Repo pass confidence**: first-pass folder backfill from the curated literature report set
- **Source URL**: https://www.usenix.org/system/files/osdi20-fried.pdf

## Why it matters here

Caladan argues that isolation at microsecond scales is better achieved with fast control signals and core allocation than with static partitioning. For this repo, the paper is useful because it keeps the literature honest: once fast I/O exists, scheduling, interference, and control policy still shape real performance.

## Problem

This paper is part of the broader literature-report corpus already summarized in `003.async_runtime_2026-03-28.md`. This folder backfill makes the paper itself locally navigable through `paper.pdf` and searchable through `paper.md`, so the repo no longer depends only on the cross-paper report for grounding.

## Method / mechanism

- **Fast core allocation instead of static partitioning**: treat isolation as a rapid allocation problem, not a permanent slice.
- **Control-signal-driven scheduling**: use runtime signals to react to interference at microsecond timescales.
- **Microsecond-scale interference management**: design scheduling decisions around very short QoS disruptions.
- **Dynamic isolation policy**: make isolation a runtime control loop rather than a fixed provisioning choice.

## Evaluation setup

- local source PDF copied from `papers/top_tier_pdfs/` into this paper folder
- first-pass curated interpretation lives in `003.async_runtime_2026-03-28.md`
- this pass prioritizes a stable local paper folder and searchable markdown extraction over a fresh deep reread

## Key findings

- Caladan argues that isolation at microsecond scales is better achieved with fast control signals and core allocation than with static partitioning. For this repo, the paper is useful because it keeps the literature honest: once fast I/O exists, scheduling, interference, and control policy still shape real performance.
- **RQ2**: medium evidence that framework redesign cannot ignore control policy
- **RQ3**: medium relevance to microsecond-scale RPC evaluation
- **RQ4**: medium support for the repo's generalization story

## Limits and confidence

- first-pass backfill from curated literature reports rather than a fully rewritten paper note
- local `paper.md` is intended as searchable grounding and may retain PDF extraction artifacts
- Caladan does not address accelerator offload or per-operation framework cost directly. It is a runtime-policy baseline rather than a mechanism paper for the DSA path.

## Repo takeaways

- this folder is the stable paper-specific home; the numbered literature reports remain the cross-paper synthesis layer
- use `paper.md` for direct textual grounding and the source literature report for the current repo-oriented interpretation

## Source links within repo

- local paper artifacts: [`paper.pdf`](./paper.pdf), [`paper.md`](./paper.md)
- [`../../003.async_runtime_2026-03-28.md`](../../003.async_runtime_2026-03-28.md)
- [`../../002.top_tier_index_2026-03-28.md`](../../002.top_tier_index_2026-03-28.md)
- [`../../006.question_correlation_matrix_2026-03-28.md`](../../006.question_correlation_matrix_2026-03-28.md)
- [`../../../../../current.md`](../../../../../current.md)
