# Current Dashboard

Read this dashboard first, match on the per-entry dashboard metadata, then open the matching canonical thread file, then the linked plan and report artifacts.

`current.md` is a dashboard/index only. Canonical mutable thread state lives under `.agents/state/threads/`.

## Live threads

- `thr-20260414-tonic-characterization`
  - index_label: Tonic characterization execution and next measurement pass
  - summary: Tighten bounded-matrix attribution into a regime-based unary Tonic characterization pass, reduce instrumentation distortion, connect higher-level buckets to lower-level CPU evidence, and turn the result into an offload-readiness ranking.
  - owner_agent: legacy-active-owner
  - owner_session_id: legacy-current-md-unrecorded-active
  - status: active
  - lease_expires_at: 2026-04-14T16:00:00Z
  - last_updated: 2026-04-14T12:00:00Z
  - canonical_thread_file: `.agents/state/threads/thr-20260414-tonic-characterization.md`
  - next_action: Execute the pre-advisor characterization priorities plan and tighten the matched-comparison lane with lower-overhead split client/server timers.
  - match_hints: tonic characterization, unary RPC regime map, matched comparison, split endpoint-local timers, advisor-ready taxonomy, offload-readiness ranking
  - related_artifacts: `docs/plan/2026-04-13/05.pre_advisor_tonic_characterization_priorities.in_progress.md`, `docs/report/benchmarking/012.tonic_characterization_refinement_results.md`, `docs/report/benchmarking/013.fleetbench_rpc_characterization_intake.md`, `results/tonic/2026-04-12-characterization/`

- `thr-20260414-tonic-literature-deck`
  - index_label: Tonic literature, paper folders, and characterization deck
  - summary: Keep the literature review, paper-folder ingestion flow, and offline-learning deck ready for future paper acquisition, extraction cleanup, or deck revisions without mixing this thread into active measurement execution.
  - owner_agent: legacy-current-md-unrecorded-literature-deck
  - owner_session_id: legacy-current-md-unrecorded-literature-deck
  - status: paused
  - lease_expires_at: 2026-04-14T16:00:00Z
  - last_updated: 2026-04-14T12:00:00Z
  - canonical_thread_file: `.agents/state/threads/thr-20260414-tonic-literature-deck.md`
  - next_action: Acquire missing cited papers, clean low-confidence paper extractions, or revise the literature deck only when the request is explicitly about literature or presentation work.
  - match_hints: tonic literature review, paper folders, gRPC cost breakdown, paper ingestion, characterization deck, typst presentation, missing citations
  - related_artifacts: `docs/report/literature/007.grpc_cost_breakdown_2026-04-12.md`, `docs/report/literature/010.expanded_paper_folder_index_2026-04-13.md`, `presentation/2026-04-12/tonic_literature_characterization.typ`, `presentation/2026-04-12/tonic_literature_characterization.pdf`

- `thr-20260414-mosaic-visualization`
  - index_label: Mosaic integration for report visualization
  - summary: Keep the Observable Framework plus Mosaic baseline ready for future dashboard polish or selective interaction reintroduction in the report visualization app.
  - owner_agent: legacy-current-md-unrecorded-mosaic-visualization
  - owner_session_id: legacy-current-md-unrecorded-mosaic-visualization
  - status: paused
  - lease_expires_at: 2026-04-14T16:00:00Z
  - last_updated: 2026-04-14T12:00:00Z
  - canonical_thread_file: `.agents/state/threads/thr-20260414-mosaic-visualization.md`
  - next_action: Resume with dashboard polish or selective interaction reintroduction in the Observable Framework plus Mosaic report app.
  - match_hints: Mosaic dashboard, Observable Framework, report visualization, bounded matrix app, tools/mosaic-tonic-report, dashboard polish, interaction reintroduction
  - related_artifacts: `docs/plan/2026-04-03/01.observable_framework_mosaic_rewrite_plan.in_progress.md`, `docs/report/artifacts/003.tonic_bounded_matrix_mosaic/index.html`, `tools/mosaic-tonic-report/src/index.md`

## Resume note

Match the incoming request against each thread's `index_label`, `summary`, `match_hints`, and `related_artifacts` in this dashboard. If there is one live match, resume it by opening the canonical thread file next. If nothing fits, create a new thread. Completed and archived work stays historical and does not belong in this live dashboard.
