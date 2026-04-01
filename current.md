# Current Focus

## Contents

- [x] Active — tonic-only profiling and experiment design (offset: 9)
- [ ] Resume note (offset: 29)

## Active — tonic-only profiling and experiment design

**Overall goal:** profile tonic-only workloads and tighten the experiment design so the report supports matched quantitative claims.

### Active items

- revise the profiling matrix to avoid confounded comparisons across size, concurrency, payload kind, and runtime
- decide what metrics must be collected for every run versus representative profiling points only

### Relevant artifacts

- `docs/plan/2026-04-01/09.tonic_only_profiling_experiment_plan.in_progress.md`
- `docs/plan/2026-04-01/08.tonic_dsa_iax_experiment_plan.in_progress.md`
- `docs/report/benchmarking/007.tonic_profile_split_localhost_results.md`
- `docs/report/benchmarking/008.tonic_profile_split_core_localhost_results.md`

### Completed items

- split-core localhost profiling rerun completed; results at `docs/report/benchmarking/008.tonic_profile_split_core_localhost_results.md`

## Resume note

When resuming work, read this file together with the latest relevant plan under `docs/plan/` before proposing new work.
