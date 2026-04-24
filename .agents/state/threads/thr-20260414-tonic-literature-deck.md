# Thread State: Tonic literature, paper folders, and characterization deck

```yaml
thread_id: thr-20260414-tonic-literature-deck
title: Tonic literature, paper folders, and characterization deck
status: paused
owner_agent: legacy-current-md-unrecorded-literature-deck
owner_session_id: legacy-current-md-unrecorded-literature-deck
previous_owner_session_id: null
lease_acquired_at: 2026-04-14T12:00:00Z
lease_expires_at: 2026-04-14T16:00:00Z
last_updated: 2026-04-14T12:00:00Z
handoff_to: null
handoff_reason: null
resume_allowed: true
match_hints:
  - tonic literature review
  - paper folders
  - gRPC cost breakdown
  - paper ingestion
  - characterization deck
  - typst presentation
  - missing citations
  - Hermes paper
superseded_by: null
source_of_truth_scope: .agents/state/threads/ canonical mutable thread state for this thread
index_label: Tonic literature, paper folders, and characterization deck
summary: Keep the literature review, paper-folder ingestion flow, and offline-learning deck ready for future paper acquisition, extraction cleanup, or deck revisions without mixing this thread into active measurement execution.
next_actions:
  - Acquire and ingest still-missing cited papers such as IOctopus, Telepathic Datacenters, OffRAC, and similar references that still lack local PDFs.
  - Improve lower-confidence paper cleanup where extraction quality blocks reliable reuse, especially RR-Compound.
  - Continue deck revisions only when the user explicitly asks for more paper-teaching depth or presentation-design work.
blocked_by: []
related_artifacts:
  - docs/report/literature/007.grpc_cost_breakdown_2026-04-12.md
  - docs/report/literature/008.paper_module_rebuild_analysis.md
  - docs/report/literature/009.paper_folder_convention_and_seed_index_2026-04-13.md
  - docs/report/literature/010.expanded_paper_folder_index_2026-04-13.md
  - docs/report/literature/papers/
  - docs/report/literature/papers/hermes-enhancing-layer-7-cloud-load-balancers-with-userspace-directed-i-o-event-notification/
  - docs/report/literature/papers/AGENTS.md
  - presentation/template.typ
  - presentation/AGENTS.md
  - presentation/2026-04-12/tonic_literature_characterization.typ
  - presentation/2026-04-12/tonic_literature_characterization.pdf
  - presentation/2026-04-14/tonic_progress_since_2026-04-09.typ
  - presentation/2026-04-14/tonic_progress_since_2026-04-09.pdf
  - tools/paper-text/
  - docs/related_work/04_rpc_acceleration_transports.md
  - docs/related_work/06_zero_copy_serialization_compression.md
```

## Detailed state

- The gRPC cost-breakdown literature scan lives in `docs/report/literature/007.grpc_cost_breakdown_2026-04-12.md`.
- The strongest end-to-end cost-decomposition citation is still *A Cloud-Scale Characterization of Remote Procedure Calls*.
- The strongest protobuf-focused citation is still *A Hardware Accelerator for Protocol Buffers*.
- The key retained positioning gap is unchanged: there is still no strong modern paper that jointly gives a gRPC or Tonic-preserving, stage-by-stage decomposition across serialization, copies, framing, scheduling, compression, and tail latency.

## Paper-folder and deck carryover

- Paper-folder convention and seed indexing were captured in report `009`, then expanded in report `010`.
- The paper-folder inventory sits at 22 folders under `docs/report/literature/papers/`.
- Reusable extraction workflow lives under `tools/paper-text/`, with local guidance in `docs/report/literature/papers/AGENTS.md`.
- Five active papers have cleaner extraction quality than RR-Compound, which is still the lowest-confidence reusable extraction.
- Hermes was added as a local paper folder with a tracked PDF and verified `paper.md`; it strengthens the event-notification and worker-scheduling side of the active Tonic literature stack.
- The literature-grounded deck lives at `presentation/2026-04-12/tonic_literature_characterization.typ`, with a compiled PDF beside it.
- The Cloud-Scale module was refreshed from local paper text, and reusable presentation design guidance plus a review checklist now live in `presentation/AGENTS.md`.
- A dated short progress deck now lives at `presentation/2026-04-14/tonic_progress_since_2026-04-09.typ` and `.pdf`, grounded in benchmarking reports `012` and `013`, literature reports `009` and `010`, and the pre-advisor priorities plan.
