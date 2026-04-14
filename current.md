# Current Focus

## Contents

- [x] Active — Tonic characterization execution and next measurement pass (offset: 9)
- [ ] Paused — Tonic literature, paper folders, and characterization deck (offset: 61)
- [ ] Paused — Mosaic integration for report visualization (offset: 115)
- [ ] Completed — Google interview research slide draft (offset: 127)
- [ ] Resume note (offset: 146)

## Active — Tonic characterization execution and next measurement pass

**Overall goal:** tighten attribution claims from the bounded first-pass matrix into a regime-based Tonic characterization pass: finish the matched-comparison measurement lane, reduce instrumentation distortion, connect higher-level stage buckets to lower-level CPU evidence, and turn the result into an offload-readiness ranking by workload regime.

### Current state

- the detailed execution update is captured in `docs/plan/2026-04-12/02.tonic_characterization_plan.in_progress.md`
- the first Phase-A characterization subset is complete in `results/tonic/2026-04-12-characterization/`, with report `docs/report/benchmarking/012.tonic_characterization_refinement_results.md`
- current measurements already separate fixed tiny-RPC codec work, 4 KiB buffer-policy sensitivity, large-message body/encode/decode movement, and compression transform cost
- instrumentation overhead is still too high outside the tiny single-thread point, so the next measurement pass needs lower-overhead endpoint-local timers before making stronger offload-readiness claims

### Characterization planning and intake

- `docs/plan/2026-04-12/02.tonic_characterization_plan.in_progress.md` remains the main execution checklist
- `docs/plan/2026-04-13/01.fleetbench_inspired_two_level_tonic_characterization.in_progress.md` adds the lower-level CPU/code-path lane that should be mapped back to the stage buckets
- `docs/plan/2026-04-13/05.pre_advisor_tonic_characterization_priorities.in_progress.md` is the new **high-priority** pre-next-meeting companion plan that turns the active thread into an advisor-facing taxonomy, workload-model, measurement, and controller-question package
- FleetBench RPC intake is complete in `docs/report/benchmarking/013.fleetbench_rpc_characterization_intake.md`
- refined decision: use a two-level characterization approach — FleetBench-style CPU/code-path characterization for lower-level understanding of realistic gRPC/protobuf instruction behavior, plus Tonic stage decomposition for higher-level attribution across encode/decode, copy/buffer lifecycle, compression, framing, runtime, and tails

### Key findings from bounded matrix (report 009)

- Runtime crossover is workload-dependent: tiny RPCs prefer single-thread, medium prefer multi-thread, large payloads fall back to movement-dominated behavior
- Medium/large uncompressed runs are ruled by `memmove`, allocator paths, `BytesMut`/`RawVec` growth — not protobuf or scheduler work
- Compression is disastrous on incompressible payloads; structured payloads show throughput/latency trade-off
- Strongest next-step lane: buffer lifecycle and copy behavior
- First characterization refinement report (012) finds current timer instrumentation is diagnostic-only for larger/high-concurrency regimes; pooled helps the 4 KiB matched point, copy-minimized helps larger selftest points, and split endpoint-local lower-overhead timers are the next measurement fix

### Next actions

- execute the new high-priority pre-advisor plan so the next meeting is framed around taxonomy, workload dimensions, evidence thresholds, and the controller-model question rather than raw profiling alone
- tighten matched-comparison claims across size/concurrency/runtime regimes rather than cherry-picked points
- reduce timer overhead and split client/server snapshots before using the timers as evidence in larger or higher-concurrency regimes
- implement and run the remaining software variants and async microbenchmark expansion where they still inform the regime map
- extend the characterization lane toward streaming only after the unary refinement is stable

### Relevant artifacts

- `current.md`
- `docs/plan/2026-04-12/02.tonic_characterization_plan.in_progress.md`
- `docs/plan/2026-04-13/01.fleetbench_inspired_two_level_tonic_characterization.in_progress.md`
- `docs/plan/2026-04-13/05.pre_advisor_tonic_characterization_priorities.in_progress.md`
- `docs/plan/2026-04-01/09.tonic_only_profiling_experiment_plan.in_progress.md`
- `docs/plan/2026-04-01/08.tonic_dsa_iax_experiment_plan.in_progress.md`
- `docs/report/benchmarking/009.tonic_profile_bounded_matrix_results.md`
- `docs/report/benchmarking/012.tonic_characterization_refinement_results.md`
- `docs/report/benchmarking/013.fleetbench_rpc_characterization_intake.md`
- `results/tonic/2026-04-12-characterization/`
- `results/tonic/2026-04-01-loop2/`
- `results/tonic/2026-04-08-frameptr/`
- `results/tonic/2026-04-08-frameptr-debuginfo/`
- `accel-rpc/tonic-profile/src/main.rs`
- `accel-rpc/async-bench/benches/async_overhead.rs`

## Paused — Tonic literature, paper folders, and characterization deck

**Resume note:** the literature review, paper-folder ingestion flow, and offline-learning deck are in a good intermediate state. Resume here when the goal is to deepen paper extraction quality, acquire still-missing cited papers, or revise the literature deck again; do not mix this thread into the active measurement-execution thread unless the user explicitly asks for literature or deck work.

### Current state

- the gRPC cost-breakdown literature scan is captured in `docs/report/literature/007.grpc_cost_breakdown_2026-04-12.md`
- strongest end-to-end cost-decomposition citation: **A Cloud-Scale Characterization of Remote Procedure Calls** (SOSP 2023)
- strongest protobuf-specific citation: **A Hardware Accelerator for Protocol Buffers** (MICRO 2021)
- key retained positioning gap: no strong modern paper jointly gives a gRPC/tonic-preserving, stage-by-stage decomposition across serialization, copies, framing, scheduling, compression, and tail latency
- the literature-grounded deck exists at `presentation/2026-04-12/tonic_literature_characterization.typ` and has already been rebuilt into a paper-first self-study walkthrough with mechanism and result detail

### Literature and paper-folder status

- paper-folder convention and seed index: `docs/report/literature/009.paper_folder_convention_and_seed_index_2026-04-13.md`
- expanded paper-folder index: `docs/report/literature/010.expanded_paper_folder_index_2026-04-13.md`
- paper-folder inventory is now 21 folders total under `docs/report/literature/papers/`
- reusable extraction workflow lives under `tools/paper-text/` and is documented in `docs/report/literature/papers/AGENTS.md`
- five of the active papers have cleaner extraction quality than RR-Compound, which remains lower-confidence raw extraction

### Deck and presentation status

- deck completed: `presentation/2026-04-12/tonic_literature_characterization.typ` compiled to `presentation/2026-04-12/tonic_literature_characterization.pdf`
- per-paper rebuild analysis: `docs/report/literature/008.paper_module_rebuild_analysis.md`
- Cloud-Scale paper module was refreshed directly from local paper text and deck verification passed
- reusable presentation design guidance and review checklist for future deck revisions are now recorded in `presentation/AGENTS.md`
- presentation planning policy now explicitly requires slide plans to map every included slide element to a written plan description rather than relying on implicit deck-side additions (`presentation/AGENTS.md`)
- deck plan rewrite completed: `docs/plan/2026-04-12/03.tonic_literature_characterization_deck_plan.in_progress.md` now uses a progressive, module-by-module rewrite structure with explicit slide-visible mapping, per-slide traceability fields, and staged completion waves instead of one monolithic rewrite pass
- presentation planning policy now also requires the plan itself to be authored progressively, with each section or module completed in enough detail before moving on, rather than leaving shallow placeholder sections (`presentation/AGENTS.md`)

### Next actions when this thread resumes

- acquire and ingest still-missing citation-only papers such as `IOctopus`, `Telepathic Datacenters`, `OffRAC`, and similar references that lack local PDFs
- improve lower-confidence paper cleanup where extraction quality still blocks reliable reuse, especially RR-Compound
- continue literature-deck revisions only if the user explicitly wants more paper-teaching depth or a stronger presentation design pass

### Relevant artifacts

- `current.md`
- `docs/plan/2026-04-12/01.grpc_cost_breakdown_note.in_progress.md`
- `docs/plan/2026-04-12/03.tonic_literature_characterization_deck_plan.in_progress.md`
- `docs/plan/2026-04-13/02.tonic_literature_paper_folder_convention.in_progress.md`
- `docs/plan/2026-04-13/03.paper_pdf_reextraction.in_progress.md`
- `docs/report/literature/007.grpc_cost_breakdown_2026-04-12.md`
- `docs/report/literature/008.paper_module_rebuild_analysis.md`
- `docs/report/literature/009.paper_folder_convention_and_seed_index_2026-04-13.md`
- `docs/report/literature/010.expanded_paper_folder_index_2026-04-13.md`
- `docs/report/literature/papers/`
- `docs/report/literature/papers/AGENTS.md`
- `presentation/template.typ`
- `presentation/AGENTS.md`
- `presentation/2026-04-12/tonic_literature_characterization.typ`
- `presentation/2026-04-12/tonic_literature_characterization.pdf`
- `tools/paper-text/`
- `docs/related_work/04_rpc_acceleration_transports.md`
- `docs/related_work/06_zero_copy_serialization_compression.md`

## Paused — Mosaic integration for report visualization

**Resume note:** the Observable Framework + Mosaic baseline is in place under `tools/mosaic-tonic-report/`; next likely work is dashboard polish or selective interaction reintroduction.

### Relevant artifacts

- `docs/plan/2026-04-03/01.observable_framework_mosaic_rewrite_plan.in_progress.md`
- `docs/report/benchmarking/009.tonic_profile_bounded_matrix_results.md`
- `docs/report/artifacts/003.tonic_bounded_matrix_mosaic/index.html`
- `tools/mosaic-tonic-report/src/index.md`
- `devenv.nix`

## Completed — Google interview research slide draft

Touying migration complete across all decks. Interview deck updated with fresh DSA hardware-floor numbers. Decks compiled and verified.

### Relevant artifacts

- `presentation/template.typ`
- `presentation/2026-04-05/google_interview_research.typ`
- `presentation/2026-04-05/google_interview_research.pdf`
- `presentation/2026-03-31/progress_2026-03-31.typ`
- `presentation/2026-03-31/progress_2026-03-31.pdf`
- `presentation/2026-03-30/tonic_offloadability.typ`
- `presentation/2026-02-23/progress_2026-02-23.typ`
- `docs/plan/2026-03-31/01.two_month_project_meeting_slides.done.md`
- `docs/plan/2026-03-30/02.tonic_offloadability_presentation.done.md`
- `docs/report/hw_eval/010.dsa_hw_eval_smoke_numbers_2026-04-06.md`
- `docs/report/architecture/002.tonic_component_analysis.md`
- `docs/report/architecture/003.tonic_interception_points.md`

## Resume note

When resuming work, read this file together with the latest relevant plan under `docs/plan/` before proposing new work.
