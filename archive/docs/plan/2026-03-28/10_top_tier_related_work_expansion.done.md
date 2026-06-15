# Expand related work with top-tier conference papers

## Goal

Add a structured related-work corpus under `docs/related_work/` that captures top-tier conference papers most relevant to the repository's research plan.

## Scope

- use `docs/research_plan.md`, `RESEARCH_PLAN.md`, recent reports, and remarks to define the topical split
- query and prioritize top-tier venues such as SIGCOMM, NSDI, OSDI, SOSP, ASPLOS, EuroSys, MICRO, HPCA, ATC, and closely related strong venues where necessary
- separate direct matches from adjacent-but-important baselines
- add the results into `docs/related_work/`, not only into a report

## Planned structure

- `docs/related_work/README.md` — overview and map
- one note per major topic aligned to the repo thesis:
  1. host and intra-host datapath work
  2. batching and submission regime
  3. async framework and completion overhead
  4. RPC acceleration and transports
  5. Intel accelerators and data-movement offload
  6. zero-copy, serialization, and compression

## Acceptance criteria

- each note contains named top-tier papers with venue and year
- each note explains why the paper matters to this repo specifically
- the notes clearly distinguish direct evidence from adjacent background
- the overview file points readers from the repo thesis to the right related-work note
