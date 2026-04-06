# Current Focus

## Contents

- [x] Active — Google interview research slide draft (offset: 9)
- [ ] Paused — Mosaic integration for report visualization (offset: 31)
- [ ] Paused — tonic-only profiling and experiment design (offset: 49)
- [ ] Resume note (offset: 67)

## Active — Google interview research slide draft

**Overall goal:** keep the presentation set Touying-native while simplifying the shared template so future slide edits spend less effort on formatting and more on content.

### Active items

- optionally do a quick visual pass on the migrated older decks if any slide now feels crowded under Touying
- keep any color aliases or one-off layout helpers local unless a true shared-template blocker appears
- only add shared helpers later if the same pattern repeats across future decks

### Relevant artifacts

- `current.md`
- `docs/plan/2026-04-05/01.google_interview_research_slide_plan.in_progress.md`
- `docs/plan/2026-04-06/01.touying_migrate_older_presentations.in_progress.md`
- `presentation/template.typ`
- `presentation/2026-04-05/google_interview_research.typ`
- `presentation/2026-04-05/google_interview_research.pdf`
- `presentation/2026-03-31/progress_2026-03-31.typ`
- `presentation/2026-03-31/progress_2026-03-31.pdf`
- `presentation/2026-03-30/tonic_offloadability.typ`
- `presentation/2026-02-23/progress_2026-02-23.typ`
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
- started revising the deck/template to use Touying instead of the hand-crafted presentation scaffolding
- migrated `presentation/template.typ` to a Touying-based slide wrapper while preserving the existing palette and helper components
- ported `presentation/2026-04-05/google_interview_research.typ` from manual page breaks to Touying `title-slide` / `slide` wrappers
- recompiled the interview deck successfully after the Touying migration and minor spacing trims
- started a second pass to make the deck more Touying-native and move shared styling into the Touying theme itself
- updated `presentation/template.typ` so the shared Touying wrapper carries more of the deck chrome (simple theme setup, palette-driven primary color, shared header/footer controls)
- rewrote `presentation/2026-04-05/google_interview_research.typ` into a more Touying-native heading-driven source using `=` / `==` slide structure
- recompiled the heading-driven Touying deck successfully and trimmed repeated-slide titles to read as continuations
- started a simplification pass to reduce custom formatting logic so future slide edits can stay content-first
- simplified `presentation/template.typ` into a thinner Touying wrapper plus only the reusable content helpers (`callout`, `card`, `panel`, `stage-card`, `fit-badge`)
- kept the heading-driven deck source unchanged under the simplified template and recompiled successfully
- migrated `presentation/2026-03-31/progress_2026-03-31.typ` from manual title/`#pagebreak()` structure to shared Touying heading-driven slides with local table striping and palette aliases
- recompiled `presentation/2026-03-31/progress_2026-03-31.typ` successfully to `presentation/2026-03-31/progress_2026-03-31.pdf`
- migrated `presentation/2026-02-23/progress_2026-02-23.typ` from manual page breaks to the shared Touying `#show: deck.with(...)` style while keeping its story and measurements intact
- recompiled `presentation/2026-02-23/progress_2026-02-23.typ` successfully with `typst compile --root ...`
- migrated `presentation/2026-03-30/tonic_offloadability.typ` from manual `#pagebreak()` / `#slide-title` structure to shared Touying `#show: deck.with(...)` plus heading-driven slides
- kept the deck-local palette aliases local and recompiled `presentation/2026-03-30/tonic_offloadability.pdf` successfully with `typst compile --root ...`
- verified the three migrated older decks compile cleanly together; Typst LSP still reports import-root access limits on `../template.typ`, so compile remains the reliable validation path
- cleaned one rough wording spot in the 2026-03-31 deck and removed unused aliases from the 2026-02-23 deck after review

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
