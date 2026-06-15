# Create repo-grounded related-work review docs

## Goal

Create a concise, repo-grounded related-work literature review under `docs/related work/`, organized by the themes already used in the repository's research and design documents.

## Scope

- add an overview file that maps the related-work groups
- add grouped notes for host and intra-host datapath work, batching and submission regime, async framework overhead, RPC acceleration and transports, Intel accelerators, and zero-copy or serialization or compression
- keep all claims grounded in existing repo documents and remarks

## Result

- added `docs/related work/README.md` as the overview and group map
- added six grouped related-work notes aligned with the repository's stated themes and citations
- kept the writing tied to the repo's existing positioning, especially the batching regime change and the host-to-accelerator offload path
