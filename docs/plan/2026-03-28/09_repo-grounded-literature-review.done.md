# Repo-Grounded Literature Review Report

## Goal

Write a literature review report that aligns external work with the repository's active research agenda rather than listing papers generically.

## Scope

- anchor the review to `docs/research_plan.md` and the measured DSA/stdexec evidence in `docs/report/`
- distinguish three layers of the repo agenda: mature evidence (`dsa-stdexec`), hardware-floor support (`hw-eval`), and forward-looking application path (`accel-rpc`)
- cover external work in the smallest set of themes that match the repo's claims:
  - batching and submission-path amortization
  - async/framework overhead in the nanosecond regime
  - Intel DSA/IAX and adjacent accelerator software stacks
  - RPC acceleration, zero-copy, and offload-adjacent transports
- explicitly call out where direct literature is sparse and where the repo is extrapolating from adjacent work

## Planned Output

- one report in `docs/report/` with:
  1. repo thesis and current evidence
  2. external literature grouped by relevance to that thesis
  3. gaps between prior work and this repo's proposed contribution
  4. concrete reading priorities / next papers to ground future implementation work

## Source Base

### Internal

- `docs/research_plan.md`
- `docs/report/progress_post_alignment_debug.md`
- `docs/report/stdexec_overhead_results.md`
- `docs/report/perf_analysis_20mpps.md`
- `docs/report/design_decisions.md`
- `docs/plan/2026-03-05/01_accelerator_driven_rpc.cancelled.md`
- `remark/002_layer_removal_measures_abstraction_cost.md`
- `remark/006_research_positioning_notes.md`
- `remark/007_batching_regime_change_is_general.md`
- `remark/008_composability_not_inherently_expensive.md`
- `remark/010_stdexec_is_case_study_not_target.md`
- `remark/011_mmio_bottleneck_software_vs_hardware_solutions.md`

### External

- official Intel / Linux / library documentation for DSA, IAX/IAA, idxd, DML/QPL, stdexec, tonic, and tokio
- peer-reviewed systems papers on host-network bottlenecks, RPC acceleration, zero-copy RPC, and accelerator-assisted data movement
- adjacent implementation references when direct academic matches are scarce

## Acceptance Criteria

- the report explains the repo's research thesis in terms consistent with the measured local evidence
- every non-trivial external claim is tied to a source category or a named work
- the report separates established evidence from proposed future work
- the review states clearly which literature directly matches the repo and which is only adjacent
