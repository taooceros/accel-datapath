# Project docs map

This directory is the human-facing research knowledge base for the project. It is part of the root Obsidian vault; open [[Home]] first if you want the top-level map.

## Primary entry points

- [[docs/research_plan|Research plan]] — thesis-level narrative and current research positioning.
- [[docs/design_document|Design document]] — system design record for the DSA/stdexec work.
- [[docs/specs/README|Specs]] — local DSA/IAX hardware specs and spec-reading guidance.
- [[docs/related_work/01_host_intra_host_datapath|Related work: host/intra-host datapath]]
- [[docs/related_work/02_batching_submission_regime|Related work: batching/submission regime]]
- [[docs/related_work/03_async_framework_completion_overhead|Related work: async framework completion overhead]]
- [[docs/related_work/04_rpc_acceleration_transports|Related work: RPC acceleration transports]]
- [[docs/related_work/05_intel_accelerators_data_movement_offload|Related work: Intel accelerators and data movement]]
- [[docs/related_work/06_zero_copy_serialization_compression|Related work: zero-copy serialization and compression]]

## Records by purpose

- `plan/YYYY-MM-DD/NN.topic.state.md` — human-readable project plans and completion records.
- `report/benchmarking/` — benchmark findings, plots, and measurement notes.
- `report/architecture/` — architecture decisions and integration history.
- `report/hw_eval/` — hardware evaluation findings.
- `report/literature/` — literature synthesis and paper folders.
- `report/research/` — broader research synthesis.
- `report/integration-review/` — integration risk and review notes.
- `report/api/` — API-facing reports.

## Boundary

Agent execution plans and workflow reports live in [[agents/README|agents/]]. Single-point insights live in [[remark/README|remark/]].
