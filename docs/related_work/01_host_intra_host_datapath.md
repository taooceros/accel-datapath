# Host and intra-host datapath work

This topic provides the broadest external motivation for the repository. These papers are not mainly about Intel DSA or stdexec, but they explain why host-side data movement, MMIO cost, queue behavior, and cache effects deserve first-class attention.

## Direct matches for the repo's positioning

- **Understanding the Host Network** — 2024, **SIGCOMM**. Best high-level anchor for the claim that intra-host movement and copies are already major bottlenecks at modern network rates.
- **hostCC** — 2023, **SIGCOMM**. Useful for framing the host interconnect as a resource that can be saturated and must be managed explicitly.
- **ZeroNIC** — 2024, **OSDI**. Strong reference for separating data and control paths and reducing CPU cost in the host datapath.
- **CXL-NIC** — 2025, **MICRO**. Most important MMIO-related comparison point: it treats MMIO as the bottleneck and attacks it in hardware, which complements this repo's software batching story.

## Why these matter here

The repository studies the **host-to-accelerator** path, while these papers mostly study NIC-to-host or broader intra-host movement. That difference is exactly why they are useful: together they suggest that the host's internal data path should be treated as a systems problem in its own right. In the repo's framing, DSA/IAA offload is one way to optimize that path, while batching and framework redesign explain how to avoid giving back those gains in software.

## Gap relative to this repo

These papers establish that intra-host bottlenecks matter, but they do not directly quantify the cost of **software framework layers sitting above batched on-die accelerators**. That is the gap the repository is trying to fill.
