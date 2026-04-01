# Current Focus

## Contents

- [x] Active — workflow and run-tracking policy (offset: 9)
- [ ] Paused — tonic-only profiling and experiment design (offset: 34)
- [ ] Resume note (offset: 54)

## Active — workflow and run-tracking policy

**Overall goal:** document and package the `current.md` workflow changes, including a workflow report and a focused commit.

### Active items

- no open items in this thread right now

### Current planning artifacts

- `docs/plan/2026-04-01/11.current_md_toc_active_sections.done.md`
- `docs/plan/2026-04-01/10.current_md_run_tracking.done.md`
- `docs/plan/2026-04-01/14.current_md_section_offset_toc.done.md`
- `docs/plan/2026-04-01/15.current_md_workflow_report_and_commit.done.md`

### Completed items

- root `AGENTS.md` updated to require explicit `current.md` run tracking and immediate completion updates; plan at `docs/plan/2026-04-01/10.current_md_run_tracking.done.md`
- root `AGENTS.md` and `current.md` updated to use a TOC-based active/paused/completed ledger; plan at `docs/plan/2026-04-01/11.current_md_toc_active_sections.done.md`
- root `AGENTS.md` and `current.md` updated so the TOC uses Markdown task-list bullets; plan at `docs/plan/2026-04-01/12.current_md_checkbox_toc.done.md`
- root `AGENTS.md` and `current.md` updated so each thread keeps its own completed items; plan at `docs/plan/2026-04-01/13.current_md_per_section_completion.done.md`
- root `AGENTS.md` and `current.md` updated so the TOC lists section status, section name, and line offset only; plan at `docs/plan/2026-04-01/14.current_md_section_offset_toc.done.md`
- workflow report written at `docs/report/workflow/003.current_md_run_ledger_policy.md`
- focused `current.md` workflow change set prepared for commit; plan at `docs/plan/2026-04-01/15.current_md_workflow_report_and_commit.done.md`

## Paused — tonic-only profiling and experiment design

**Last known goal:** profile tonic-only workloads and tighten the experiment design so the report supports matched quantitative claims.

### Paused items

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
