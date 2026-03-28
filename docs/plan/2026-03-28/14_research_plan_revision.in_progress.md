# Research Plan Revision

**Date**: 2026-03-28
**Status**: in progress

## Goal

Revise `docs/research_plan.md` so it reflects what the repo's literature review now supports most strongly.

## Why this revision is needed

The current plan already captures the batching-regime thesis, but it overweights cross-domain characterization and RPC transport ambition relative to the strength of current repo evidence. The latest literature-review synthesis and repo remarks point to a better near-term ordering:

1. Immediate systems work on cache footprint and completion-path behavior
2. Transferable framework-design lessons for the nanosecond regime
3. Production-facing RPC decomposition and crossover analysis before full transport construction
4. Cross-domain validation as a next major validation step rather than the first execution priority
5. CXL/MMIO complementarity as positioning support, not the main workstream

## Inputs

- `docs/report/003.repo_grounded_literature_review_2026-03-28.md`
- `docs/research_plan.md`
- `remark/001_cache_working_set_vs_throughput.md`
- `remark/003_bistable_throughput_regime.md`
- `remark/010_stdexec_is_case_study_not_target.md`
- `remark/011_mmio_bottleneck_software_vs_hardware_solutions.md`

## Planned edits

1. Update the executive summary to foreground the strongest established results and narrow the forward-looking claims.
2. Revise preliminary results to make cache and completion-path findings more central.
3. Reorder research questions and proposed thrusts to put immediate bottleneck work and ns-regime framework design ahead of broader RPC and cross-domain ambitions.
4. Add caveats that stdexec is the case study, not the deployment target, and that tonic/gRPC accelerator literature is still sparse.
5. Adjust timeline and impact sections to match the revised priority order.

## Verification

1. Read the revised `docs/research_plan.md` for structure and claim alignment.
2. Run diagnostics on the modified markdown file if available.
3. Validate the revised ordering against an Oracle review grounded in repo evidence.
