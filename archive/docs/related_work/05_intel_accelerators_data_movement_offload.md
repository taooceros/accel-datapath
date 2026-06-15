# Intel accelerators and data-movement offload

This note captures the most directly relevant literature for `dsa-stdexec`, `hw-eval`, and the DSA/IAA parts of `accel-rpc`.

## Direct matches

- **DSA-2LM: A CPU-Free Tiered Memory Architecture with Intel DSA** — 2025, **USENIX ATC**. The strongest top-tier Intel DSA systems paper identified in this search pass.
- **How to Copy Memory? Coordinated Asynchronous Copy as a First-Class OS Service** — 2025, **SOSP**. Not Intel-specific, but extremely relevant to any argument that copy/offload should be treated as a managed async service.

## Important adjacent movement papers

- **IOctopus: Outsmarting Nonuniform DMA** — 2020, **ASPLOS**. Useful for topology, placement, and DMA policy effects.
- **True IOMMU Protection from DMA Attacks: When Copy is Faster than Zero Copy** — 2016. A useful caution against naive assumptions that zero-copy or DMA offload automatically wins.

## Primary non-paper sources that still matter

- Intel DSA Architecture Specification
- Intel DSA User Guide
- Intel DSA Enabling Guide
- Intel IAA Architecture Specification
- Intel IAA User Guide
- Intel DML
- Intel QPL

These are not top-tier papers, but they are the main ground truth for the hardware and software stacks the repo actually implements against.

## Why these matter here

The repository does not only need proof that DSA or IAA can accelerate something. It needs support for a more specific question: under what queue, batching, and runtime conditions does offload remain worthwhile once software overhead is included? DSA-2LM and Copier are the best top-tier anchors for that broader question.

## Gap relative to this repo

Top-tier **IAA-specific** papers remain sparse, and direct literature on composable async abstractions over DSA/IAA is even thinner. The repo therefore has to combine Intel primary docs with a small set of strong systems papers.
