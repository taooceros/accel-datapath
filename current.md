# Current Focus

## Contents

- [x] Active — Google interview research slide draft (offset: 9)
- [ ] Paused — Mosaic integration for report visualization (offset: 31)
- [ ] Paused — tonic-only profiling and experiment design (offset: 49)
- [ ] Resume note (offset: 67)

## Active — Google interview research slide draft

**Overall goal:** draft a conversational ~15 minute slide that introduces the basic research idea, the current research plan, and the concrete progress already achieved, grounded in the existing presentation and plan/report artifacts.

### Active items

- review the newly drafted deck and tighten wording or visuals if requested
- adapt the deck to the final interview emphasis if the story needs rebalancing
- optionally add speaker notes or a shorter backup version

### Relevant artifacts

- `current.md`
- `docs/plan/2026-04-05/01.google_interview_research_slide_plan.in_progress.md`
- `presentation/2026-04-05/google_interview_research.typ`
- `presentation/2026-04-05/google_interview_research.pdf`
- `presentation/2026-03-31/progress_2026-03-31.typ`
- `presentation/2026-03-30/tonic_offloadability.typ`
- `docs/plan/2026-03-31/01.two_month_project_meeting_slides.done.md`
- `docs/plan/2026-03-30/02.tonic_offloadability_presentation.done.md`
- `docs/report/benchmarking/009.tonic_profile_bounded_matrix_results.md`
- `docs/report/architecture/002.tonic_component_analysis.md`
- `docs/report/architecture/003.tonic_interception_points.md`

### Completed items

- switched active focus from Mosaic work to interview-oriented slide drafting
- identified the most relevant existing decks, plans, and reports to reuse for the new slide
- synthesized a 7-slide conversational outline for a ~15 minute interview talk grounded in the latest presentation and report artifacts
- reframed the project premise around async API suitability for modern fast accelerators, batching, MMIO amortization, and control-path overhead
- drafted a concrete ~10-slide Typst deck at `presentation/2026-04-05/google_interview_research.typ`
- compiled the deck successfully to `presentation/2026-04-05/google_interview_research.pdf`

## Paused — Mosaic integration for report visualization

**Resume note:** the Observable Framework + Mosaic baseline is in place under `tools/mosaic-tonic-report/`; next likely work is dashboard polish or selective interaction reintroduction.

### Relevant artifacts

- `docs/plan/2026-04-03/01.observable_framework_mosaic_rewrite_plan.in_progress.md`
- `docs/report/benchmarking/009.tonic_profile_bounded_matrix_results.md`
- `docs/report/artifacts/003.tonic_bounded_matrix_mosaic/index.html`
- `tools/mosaic-tonic-report/src/index.md`
- `devenv.nix`

## Paused — tonic-only profiling and experiment design

**Resume note:** bounded first-pass matrix is done; next likely step is tightening matched-comparison claims and the next profiling matrix using `009.tonic_profile_bounded_matrix_results.md`.

### Relevant artifacts

- `docs/plan/2026-04-01/09.tonic_only_profiling_experiment_plan.in_progress.md`
- `docs/plan/2026-04-01/08.tonic_dsa_iax_experiment_plan.in_progress.md`
- `docs/report/benchmarking/007.tonic_profile_split_localhost_results.md`
- `docs/report/benchmarking/008.tonic_profile_split_core_localhost_results.md`
- `docs/report/benchmarking/009.tonic_profile_bounded_matrix_results.md`
- `results/tonic/2026-04-01-loop2/`

### Completed items

- split-core localhost profiling rerun completed; results at `docs/report/benchmarking/008.tonic_profile_split_core_localhost_results.md`
- bounded first-pass tonic matrix executed with representative perf/flamegraph captures; results at `docs/report/benchmarking/009.tonic_profile_bounded_matrix_results.md`

## Resume note

When resuming work, read this file together with the latest relevant plan under `docs/plan/` before proposing new work.
