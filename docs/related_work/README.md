# Related Work

This directory organizes related work around the repository's actual research thesis rather than around individual papers. The central claim is that **batching can amortize hardware submission cost enough that software framework overhead becomes the dominant bottleneck**, and the notes here group prior work by how directly they support, qualify, or motivate that claim.

## How to read these notes

- **Direct matches** are the strongest papers for the repository's exact questions.
- **Adjacent baselines** matter because they frame the same bottlenecks from a nearby angle.
- Topic files emphasize why a paper matters to this repo, not just what the paper did.

## Topic map

- `01_host_intra_host_datapath.md` — host-network and intra-host bottlenecks that motivate the host-to-accelerator framing
- `02_batching_submission_regime.md` — batching, submission amortization, and fast-path API design
- `03_async_framework_completion_overhead.md` — poll-mode design, completion mechanisms, and runtime overhead
- `04_rpc_acceleration_transports.md` — fast RPC, SmartNIC RPC, and transport-aware RPC acceleration
- `05_intel_accelerators_data_movement_offload.md` — Intel DSA/IAA systems work and the closest data-movement-offload literature
- `06_zero_copy_serialization_compression.md` — zero-copy, serialization, and compression work adjacent to `accel-rpc`

## Main repo cross-links

- `docs/research_plan.md`
- `RESEARCH_PLAN.md`
- `docs/report/003.repo_grounded_literature_review_2026-03-28.md`
- `remark/006_research_positioning_notes.md`
- `remark/007_batching_regime_change_is_general.md`
- `remark/011_mmio_bottleneck_software_vs_hardware_solutions.md`
