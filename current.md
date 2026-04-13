# Current Focus

## Contents

- [x] Active — tonic profiling: matched-comparison refinement + internal phase timers + software variants (offset: 9)
- [ ] Paused — Mosaic integration for report visualization (offset: 147)
- [ ] Resume note (offset: 178)

## Active — tonic profiling: matched-comparison refinement + internal phase timers + software variants

**Overall goal:** tighten attribution claims from the bounded first-pass matrix into a literature-grounded Tonic characterization pass: use matched comparisons, add internal phase timers to `tonic-profile`, run pooled-buffer / copy-minimized / instrumentation-off variants, and turn the result into a regime-based offload-readiness ranking.

### Literature context just completed

- the gRPC cost-breakdown literature scan is now captured in `docs/report/literature/007.grpc_cost_breakdown_2026-04-12.md`
- strongest end-to-end cost-decomposition citation: **A Cloud-Scale Characterization of Remote Procedure Calls** (SOSP 2023)
- strongest protobuf-specific citation: **A Hardware Accelerator for Protocol Buffers** (MICRO 2021)
- key gap retained for repo positioning: no strong modern paper jointly gives gRPC/tonic-preserving, stage-by-stage decomposition across serialization, copies, framing, scheduling, compression, and tail latency
- characterization implication carried into planning: Tonic should be measured by bucket and by workload regime, not by a single averaged baseline
- paper-folder convention added for the active Tonic literature flow: `docs/report/literature/009.paper_folder_convention_and_seed_index_2026-04-13.md`
- seeded paper folders now live under `docs/report/literature/papers/`, so the review can move from synthesis notes to per-paper explanation pages without relying on filename search

### Active planning refinement

- the detailed characterization update is captured in `docs/plan/2026-04-12/02.tonic_characterization_plan.in_progress.md`
- this refinement keeps `docs/plan/2026-04-01/09.tonic_only_profiling_experiment_plan.in_progress.md` as the broad harness plan, but tightens the next execution pass around regime-based decomposition, control variants, and offload-readiness ranking

### Current run — execute modified characterization experiment

- User asked to conduct the new experiment based on the modified experiment plan.
- Execution target is Phase A from `docs/plan/2026-04-12/02.tonic_characterization_plan.in_progress.md`: verify/finish `tonic-profile` instrumentation and controls, run a matched unary subset, and write a benchmarking report with instrumentation-overhead/stage-attribution results.
- Current execution completed for the first Phase-A characterization subset: built `tonic-profile`, ran matched unary selftest regimes with instrumentation on/off, pooled, copy-minimized, and 64 KiB compression controls, plus representative `perf stat -d` captures.
- New results are under `results/tonic/2026-04-12-characterization/`; report is `docs/report/benchmarking/012.tonic_characterization_refinement_results.md`.
- Main outcome: internal timers now separate fixed tiny-RPC codec work, 4 KiB buffer-policy sensitivity, large-message body/encode/decode movement, and compression transform cost; instrumentation overhead is high outside the tiny single-thread point, so the next pass should reduce timer overhead and split client/server snapshots before offload-readiness thresholds.


### FleetBench RPC characterization intake

- New plan drafted for two-level FleetBench-inspired Tonic characterization with explicit high-level-to-low-level mapping: `docs/plan/2026-04-13/01.fleetbench_inspired_two_level_tonic_characterization.in_progress.md`.
- New task: inspect Google FleetBench (`https://github.com/google/fleetbench/tree/main`) for RPC workload/characterization mechanisms and decide whether its RPC characteristics should be incorporated into the repo's Tonic characterization mechanism.
- Completed: inspected FleetBench RPC source/docs and wrote `docs/report/benchmarking/013.fleetbench_rpc_characterization_intake.md`.
- Refined decision: use a two-level characterization approach — FleetBench-style CPU/code-path characterization for lower-level understanding of realistic gRPC/protobuf instruction behavior, plus Tonic stage decomposition for higher-level attribution across encode/decode, copy/buffer lifecycle, compression, framing, runtime, and tails. Add a Tonic-native `unary-proto-shape` mode with FleetBench-like proto program shapes, asymmetric request/response shape selection, closed-loop outstanding-RPC/connection knobs, and client/server delay distributions. Add an explicit mapping layer from high-level stage buckets to low-level CPU evidence so perf/flamegraph/topdown data can validate or qualify decomposition claims.

### New presentation task

- create a literature-grounded presentation deck that explains how the reviewed RPC / gRPC / serialization literature guides Tonic characterization
- keep the deck aligned with the repo's recent Typst presentation style and connect the literature buckets to the repo's profiling and planning direction
- deck completed: `presentation/2026-04-12/tonic_literature_characterization.typ` compiled to `presentation/2026-04-12/tonic_literature_characterization.pdf`
- revision requested: make the deck more explicit about introducing the key literature items themselves and stating what each one teaches the Tonic characterization effort
- revision completed: Slides 3–5 now introduce the key papers more directly and state the lesson each one contributes to the Tonic characterization story
- stronger revision requested: the deck should also reflect paper details — how each paper achieves its result (mechanism / method) and the main results that matter for this repo's characterization story
- stronger revision completed: the literature section now introduces the main papers with mechanism-level detail and key results, not just topic-level lessons
- further direction: it is acceptable for the deck to be longer if that produces a clearer paper-by-paper literature introduction with enough mechanism and result detail
- mode change: this artifact is mainly for offline learning about what the papers are doing, so it does not need to stay presentation-short; it can be longer as long as it explains the papers clearly and uses better illustration than a direct report
- mode change completed: the literature deck has been expanded into a longer self-study walkthrough with paper-by-paper explanation, methodology flow, result interpretation, and repo-specific takeaways
- review requested: ask Oracle to judge whether the current learning deck is actually sufficient for learning the papers, especially whether it has enough mechanism, detail, and quantitative results to teach the reader something beyond the written report
- next revision target: write a detailed per-paper analysis artifact first, then use that analysis to rewrite the slide plan and rebuild the paper modules around actual mechanism, evaluation setup, concrete findings, and repo-specific interpretation
- revision completed: wrote `docs/report/literature/008.paper_module_rebuild_analysis.md`, rewrote the slide plan around it, rebuilt the deck into paper-first teaching modules, and recompiled the deck cleanly
- new revision request: strengthen results coverage inside each paper module so the deck shows concrete experiment or characterization results, not just the idea and interpretation
- results-density revision completed: strengthened the paper modules so result slides now carry more explicit experiment or characterization findings with context/meaning, then recompiled the deck cleanly
- current literature follow-up: directly read the Cloud-Scale RPC paper sources to pull more detailed characterization data and findings that can be taught in the deck, rather than relying only on secondary summary-level notes
- new revision request: the original Cloud-Scale RPC paper PDF is now available locally, so the next deck polish pass should use the paper text itself to strengthen the corresponding presentation module
- paper-grounded deck polish completed: the Cloud-Scale RPC slide module now cites the paper's actual methodology and quantitative findings (Monarch/Dapper/GWP, 700-day window, 10K+ methods, 722B sampled RPCs, latency-tax skew) and the deck recompiles cleanly
- new deck direction: revise all paper modules with a broader learning lens so each module explicitly teaches the interesting technique/mechanism the paper uses, not just the repo-facing result or takeaway

### Literature paper-folder convention and backfill

- new plan drafted for a paper-oriented literature layout under `docs/report/literature/papers/`: `docs/plan/2026-04-13/02.tonic_literature_paper_folder_convention.in_progress.md`
- new convention and seed index written: `docs/report/literature/009.paper_folder_convention_and_seed_index_2026-04-13.md`
- backfilled the currently active six-paper Tonic set into dedicated folders with structured `README.md` pages for Cloud-Scale RPC Characterization, Hardware Accelerator for Protocol Buffers, TF-gRPC-Bench, RPCAcc, Cornflakes, and RR-Compound
- active literature artifacts now link into the paper folders where it fits naturally, so the flow can move from review note or topic note to paper folder
- RR-Compound remains explicitly lower-confidence and metadata-grounded in this pass
- follow-up task started: download original PDFs and extract plain text for the active review paper set, storing `paper.pdf` and `paper.txt` inside each paper folder when directly obtainable
- PDF/text ingestion completed for the active review set: all six paper folders now have `paper.pdf`; five papers have Ghostscript-derived `paper.txt`, and RR-Compound has a parser-derived partial/incomplete fallback `paper.txt` because the available CLI extractors repeatedly timed out on that IEEE manuscript
- new follow-up requested: re-extract the active paper PDFs via the repo PDF workflow, refreshing each folder's `paper.txt` and checking whether RR-Compound still requires the partial fallback path
- maintenance task started: verify whether the PDF workflow dependencies are available in the repo's Nix/devenv environment and, if missing, add the minimal package set to `devenv.nix`
- current subtask: re-extract the local Cloud-Scale RPC paper into repo-tracked Markdown so the paper text itself is KB-searchable rather than only available through `paper.txt`
- Cloud-Scale paper refresh completed for KB ingestion: extracted `docs/report/literature/papers/cloud-scale-characterization-of-remote-procedure-calls/paper.md` from the local `paper.pdf` via `pdftotext -layout`, retained the existing `paper.txt`, and linked the new Markdown artifact from the paper-folder `README.md`
- follow-up fix completed: replaced the initial layout-preserving Markdown extraction with a column-aware `pdftotext` pass because the SOSP two-column layout was interleaving sentence order; the refreshed `paper.md` now reads the abstract/introduction in left-column then right-column order with cleaner paragraph text
- new documentation task: record the paper-text extraction pipeline in a local `docs/report/literature/papers/AGENTS.md` so future PDF ingestion reuses the column-aware Markdown workflow instead of the broken layout-preserving pass
- papers-folder extraction guidance completed: added `docs/report/literature/papers/AGENTS.md` documenting required paper artifacts, preferred `paper.md` output, extraction method order, two-column handling, normalization expectations, verification checks, and failure-reporting rules for future PDF ingestion
- new follow-up requested: make the Cloud-Scale `paper.md` more Markdown-native by splitting it into sections rather than page blocks; figure-caption cleanup remains explicitly deferred
- extended follow-up requested: after the section-based rewrite, update the papers-folder `AGENTS.md` again and add reusable scripts under `tools/` so future paper extraction and sectionization use the same workflow
- section-based paper pass completed for the Cloud-Scale paper: regenerated `paper.md` from `paper.pdf` with the reusable `tools/paper-text/extract_paper_text.py --columns two` flow, converted it into section-based Markdown with `tools/paper-text/sectionize_markdown.py`, and verified it with `tools/paper-text/verify_paper_text.py`
- reusable paper-text workflow added under `tools/paper-text/` (`extract_paper_text.py`, `sectionize_markdown.py`, `verify_paper_text.py`, `README.md`) and `docs/report/literature/papers/AGENTS.md` now points future paper ingestion at that extract → sectionize → verify pipeline, with figure-caption cleanup still explicitly deferred
- new follow-up requested: apply the reusable paper-text extraction pipeline to the other active paper folders so the remaining local `paper.pdf` artifacts also gain searchable `paper.md` outputs
- batch paper extraction completed for the other five active folders: `cornflakes-zero-copy-serialization-for-microsecond-scale-networking`, `designing-a-micro-benchmark-suite-to-evaluate-grpc-for-tensorflow-early-experiences`, `hardware-accelerator-for-protocol-buffers`, `rpcacc-a-high-performance-and-reconfigurable-pcie-attached-rpc-accelerator`, and `rr-compound-rdma-fused-grpc-for-low-latency-high-throughput-and-easy-interface` now each have a generated `paper.md`
- extractor hard dependencies were reduced during the batch pass: `tools/paper-text/extract_paper_text.py` now relies on Poppler tooling (`pdftotext`, `pdfinfo`) instead of Python PDF libraries so it can run in the current shell environment
- batch note: the sectionizer/verifier remains tuned to papers whose headings are recoverable cleanly, so the five newly refreshed papers were left in the safe page-based Markdown form after extraction; all paper-folder `README.md` files now link `paper.md` alongside `paper.pdf` / `paper.txt`
- quality note: RR-Compound now has a searchable `paper.md`, but its IEEE source remains visibly noisier than the other papers, so it should still be treated as lower-confidence raw extraction until a stronger cleanup pass is done
- broader literature inventory check completed: under `docs/report/literature/`, the only local per-paper pipeline targets remain the six folders in `docs/report/literature/papers/`; glob checks show 6 `paper.pdf` files and the same 6 `paper.md` files, so the request to run the same pipeline across all recorded literature papers is already satisfied for the current repo state
- scope correction requested: expand the paper-text pipeline target set beyond the seeded six folders to include any additional papers referenced anywhere in the numbered literature reports under `docs/report/literature/`, then determine which of those references already have local paper folders versus which still need per-paper artifacts
- broader literature expansion completed for the locally downloaded corpus: 15 additional papers from `papers/top_tier_pdfs/` now have stable folders under `docs/report/literature/papers/`, each with `paper.pdf`, searchable `paper.md`, and a `README.md` grounded in the existing literature reports
- new literature index added: `docs/report/literature/010.expanded_paper_folder_index_2026-04-13.md` records the expanded folder set and separates the still-blocked citation-only papers that lack local PDFs from the papers that are now fully foldered
- paper-folder inventory is now 21 folders total (6 seeded + 15 expanded-corpus folders); the remaining un-foldered literature references are citation-only cases such as `IOctopus`, `Telepathic Datacenters`, `OffRAC`, and similar papers that need a later acquisition pass before extraction
- new cleanup requested: in the literature reports, the reader-facing paper link should point to the per-paper `README.md` entry point rather than raw `papers/top_tier_pdfs/*.pdf` provenance paths; preserve provenance separately in historical index-style docs where useful
- literature report link cleanup completed: narrative reports `003.async_runtime_2026-03-28.md`, `004.rpc_transport_2026-03-28.md`, and `005.accelerator_hostpath_2026-03-28.md` now use `**Paper folder**` links to `docs/report/literature/papers/<slug>/README.md`, while `002.top_tier_index_2026-03-28.md` keeps the raw `Local PDF` provenance column and adds a separate `Paper folder` column for navigation
- new deck revision request: regenerate the Cloud-Scale paper module in `presentation/2026-04-12/tonic_literature_characterization.typ` directly from the extracted paper text, using `paper.md` details to strengthen the methodology and quantitative-results slides
- Cloud-Scale paper-text regeneration completed: Slides 5–7 in `presentation/2026-04-12/tonic_literature_characterization.typ` now reflect the extracted `paper.md` more directly by naming the measurement pipeline and component model more explicitly and by adding the fleet-wide RPC cycle-tax result; the deck recompiles cleanly
- current subtask: rebuild the remaining paper modules so each slide sequence foregrounds notable techniques such as ProtoBuf accelerator placement/design choices, TF-gRPC-Bench benchmark-construction controls, RPCAcc's PCIe-aware co-design, Cornflakes's hybrid field-level copy/scatter-gather policy, and RR-Compound's compatibility-first fast-path design constraints
- broader-learning revision completed: the paper modules now explicitly teach notable techniques beyond the repo-facing result (for example near-core instruction-dispatched protobuf acceleration, benchmark-construction controls in TF-gRPC-Bench, PCIe-aware batching/placement in RPCAcc, and hybrid CFPtr/RcBuf API design in Cornflakes); Oracle review passed overall and the final clarity fixes were applied before recompiling cleanly
- Cloud-Scale deck verification is now complete: slides 5–7 in `presentation/2026-04-12/tonic_literature_characterization.typ` compile cleanly with `typst compile --root .`, and `lsp_diagnostics` reports no issues on the edited `.typ` file

### Key findings from bounded matrix (report 009)

- Runtime crossover is workload-dependent: tiny RPCs prefer single-thread, medium prefer multi-thread, large payloads fall back to movement-dominated behavior
- Medium/large uncompressed runs are ruled by `memmove`, allocator paths, `BytesMut`/`RawVec` growth — not protobuf or scheduler work
- Compression is disastrous on incompressible payloads; structured payloads show throughput/latency trade-off
- Strongest next-step lane: buffer lifecycle and copy behavior
- First characterization refinement report (012) finds current timer instrumentation is diagnostic-only for larger/high-concurrency regimes; pooled helps the 4 KiB matched point, copy-minimized helps larger selftest points, and split endpoint-local lower-overhead timers are the next measurement fix

### Active items

- [x] create `presentation/2026-04-08/tonic_research_story.typ` from plan `docs/plan/2026-04-08/02.research_story_deck_plan.in_progress.md` and compile clean — done 2026-04-08 (`/tmp/tonic_research_story.pdf`)
- [x] rerun representative tonic perf/flamegraph captures after frame-pointer enablement, including server-side profiles — done 2026-04-08 (`results/tonic/2026-04-08-frameptr/`, report `docs/report/benchmarking/010.frame_pointer_rerun_client_server_results.md`)
- [x] rerun the representative profiling lane with release debuginfo enabled, keeping frame pointers and client/server capture symmetry — done 2026-04-08 (`results/tonic/2026-04-08-frameptr-debuginfo/`, report `docs/report/benchmarking/011.debug_symbol_rerun_client_server_results.md`)
- [x] revise `presentation/2026-04-08/tonic_flamegraph_analysis.typ` to reflect frame-pointer attribution: split large-message copy cost across client/server `run_phase`, prost encode/decode, tonic body buffering, and realloc instead of presenting a prost-ambiguous `memmove` monolith — done 2026-04-08 (plan `docs/plan/2026-04-08/03.frameptr_flamegraph_slide_revision.done.md`, compiled `presentation/2026-04-08/tonic_flamegraph_analysis.pdf`)
- [x] enrich `presentation/2026-04-08/tonic_flamegraph_analysis.typ` using debuginfo rerun: map hot copy paths back to concrete source files/lines and annotate the slide with source-backed caller locations — done 2026-04-08 (plan `docs/plan/2026-04-08/04.debuginfo_source_mapped_slide_revision.done.md`, rebuilt `presentation/2026-04-08/tonic_flamegraph_analysis.pdf`)
- tighten matched-comparison claims: compare across matched size/concurrency/runtime regimes, not cherry-picked points
- add internal phase timers to `tonic-profile` (encode/decode/compress/buffer stages)
- implement and run software variants: pooled-buffer, copy-minimized, instrumentation-off control
- run async microbenchmarks expansion (tokio spawn/join, oneshot/mpsc, wakeup overhead)
- expand to streaming modes (server, client, bidi) when the unary refinement is stable
- use the new characterization plan as the execution checklist for the unary refinement pass
- create a literature-based presentation artifact that can be used to explain the characterization framing and experimental direction
- [x] create the literature characterization deck from plan `docs/plan/2026-04-12/03.tonic_literature_characterization_deck_plan.in_progress.md` and compile clean — done 2026-04-12 (`presentation/2026-04-12/tonic_literature_characterization.typ`, `presentation/2026-04-12/tonic_literature_characterization.pdf`)
- [x] revise the literature characterization deck with paper-level mechanism and result details — done 2026-04-12 (updated `presentation/2026-04-12/tonic_literature_characterization.typ`, recompiled `presentation/2026-04-12/tonic_literature_characterization.pdf`)
- [x] expand the literature characterization deck into a longer offline-learning walkthrough — done 2026-04-12 (updated `presentation/2026-04-12/tonic_literature_characterization.typ`, recompiled `presentation/2026-04-12/tonic_literature_characterization.pdf`)
- revise the literature characterization deck so the key papers are introduced more directly and each slide states the lesson each paper contributes
- [x] revise the literature characterization deck so the key papers are introduced more directly and each slide states the lesson each paper contributes — done 2026-04-12 (updated `presentation/2026-04-12/tonic_literature_characterization.typ`, recompiled `presentation/2026-04-12/tonic_literature_characterization.pdf`)
- retrieve and incorporate paper-level mechanism and result details for the literature characterization deck before the next revision pass
- [x] refresh the Cloud-Scale RPC slides in the literature characterization deck using the local `paper.pdf` / `paper.txt` source rather than summary-level notes alone — done 2026-04-13 (`presentation/2026-04-12/tonic_literature_characterization.typ`, recompiled `presentation/2026-04-12/tonic_literature_characterization.pdf`)
- [x] add a paper-folder convention under `docs/report/literature/` and backfill the active six-paper Tonic set — done 2026-04-13 (`docs/report/literature/009.paper_folder_convention_and_seed_index_2026-04-13.md`, `docs/report/literature/papers/`)
- [x] write updated profiling report with stage attribution tables and next-step shortlist
- [x] run first Phase-A characterization subset and write report 012 — done 2026-04-12 (`results/tonic/2026-04-12-characterization/`, `docs/report/benchmarking/012.tonic_characterization_refinement_results.md`)
- [x] inspect FleetBench RPC characterization mechanisms and write incorporation recommendation — done 2026-04-12 (`docs/report/benchmarking/013.fleetbench_rpc_characterization_intake.md`)
- [x] fix flamegraph analysis slide (offload-potential column in table, split offload vs software-only cards, takeaway; added Slide 1b stack+crate-map context and Slide 5 architecture validation) — done 2026-04-08

### Relevant artifacts

- `current.md`
- `docs/plan/2026-04-12/01.grpc_cost_breakdown_note.in_progress.md`
- `docs/plan/2026-04-12/02.tonic_characterization_plan.in_progress.md`
- `docs/plan/2026-04-12/03.tonic_literature_characterization_deck_plan.in_progress.md`
- `docs/report/benchmarking/013.fleetbench_rpc_characterization_intake.md`
- `docs/plan/2026-04-13/01.fleetbench_inspired_two_level_tonic_characterization.in_progress.md`
- `docs/plan/2026-04-12/04.fleetbench_rpc_characterization_intake.done.md`
- `docs/report/literature/007.grpc_cost_breakdown_2026-04-12.md`
- `docs/report/literature/008.paper_module_rebuild_analysis.md`
- `docs/report/literature/009.paper_folder_convention_and_seed_index_2026-04-13.md`
- `docs/plan/2026-04-13/03.paper_pdf_reextraction.in_progress.md`
- `docs/report/literature/papers/`
- `presentation/template.typ`
- `docs/related_work/04_rpc_acceleration_transports.md`
- `docs/related_work/06_zero_copy_serialization_compression.md`
- `docs/plan/2026-04-01/09.tonic_only_profiling_experiment_plan.in_progress.md`
- `docs/plan/2026-04-01/08.tonic_dsa_iax_experiment_plan.in_progress.md`
- `docs/report/benchmarking/007.tonic_profile_split_localhost_results.md`
- `docs/report/benchmarking/008.tonic_profile_split_core_localhost_results.md`
- `docs/report/benchmarking/009.tonic_profile_bounded_matrix_results.md`
- `docs/report/benchmarking/010.frame_pointer_rerun_client_server_results.md`
- `docs/report/benchmarking/011.debug_symbol_rerun_client_server_results.md`
- `results/tonic/2026-04-12-characterization/`
- `docs/report/benchmarking/012.tonic_characterization_refinement_results.md`
- `docs/plan/2026-04-08/04.debug_symbol_profile_rerun.in_progress.md`
- `docs/plan/2026-04-08/03.frame_pointer_profile_rerun.in_progress.md`
- `accel-rpc/tonic-profile/src/main.rs`
- `accel-rpc/async-bench/benches/async_overhead.rs`
- `results/tonic/2026-04-01-loop2/`
- `results/tonic/2026-04-08-frameptr/`
- `results/tonic/2026-04-08-frameptr-debuginfo/`
- `tools/mosaic-tonic-report/`
- `presentation/2026-04-08/tonic_flamegraph_analysis.typ`
- `presentation/2026-04-08/tonic_flamegraph_analysis.pdf`
- `presentation/2026-04-08/tonic_research_story.typ`
- `presentation/2026-04-12/tonic_literature_characterization.typ`
- `presentation/2026-04-12/tonic_literature_characterization.pdf`
- `docs/plan/2026-04-08/01.tonic_flamegraph_offload_reframe_plan.done.md`
- `docs/plan/2026-04-08/02.research_story_deck_plan.done.md`
- `docs/plan/2026-04-08/03.frameptr_flamegraph_slide_revision.done.md`
- `docs/plan/2026-04-08/04.debuginfo_source_mapped_slide_revision.done.md`

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
