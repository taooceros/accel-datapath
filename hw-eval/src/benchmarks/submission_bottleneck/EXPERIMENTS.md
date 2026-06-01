# Submission bottleneck experiments

This folder contains the DSA submission-bottleneck experiment implementations. Each experiment keeps batch size fixed at one logical operation per MMIO submission unless explicitly stated otherwise; concurrency is the number of logical operations outstanding or pushed.

## Experiment index

| Experiment | Selector | Module | Local doc |
|---|---|---|---|
| 1. Submit occupancy | `submit-occupancy` | `experiment_1_submit_occupancy.rs` | `experiment_1_submit_occupancy.md` |
| 2. Marker overlap suite | `submit-marker-overlap` | `experiment_2_marker_overlap.rs` facade; private `overlap.rs` + `mechanism_probes.rs` implementations | `experiment_2_marker_overlap.md` |
| 2B-F. Marker mechanism probes only | `submit-marker-mechanism` | Exported by `experiment_2_marker_overlap.rs`; implemented in private `mechanism_probes.rs` | `experiment_2_marker_overlap/mechanism_probes.md` |
| 3. Traffic ladder | `traffic-class-ladder` | `experiment_3_traffic_ladder.rs` | `experiment_3_traffic_ladder.md` |
| 4. Completion reuse | `completion-reuse-policy` | `experiment_4_completion_reuse.rs` | `experiment_4_completion_reuse.md` |
| 5. Admission probe | `submit-admission` | `experiment_5_submit_admission_probe.rs` | `experiment_5_submit_admission_probe.md` |

## Reading order

- Start with Experiment 5 when checking whether a burst size is safe to push.
- Use Experiment 1 to locate the submit-admission knee under controlled prefill.
- Use Experiment 2's default suite to inspect overlap and run all per-status-read mechanism probes under the same marker-burst/poll-offset configuration.
- Use `submit-marker-mechanism` only when rerunning the cacheline/coherence probes without the baseline overlap rows.
- Use Experiment 3 to distinguish submit-only pressure from completion-write and payload-DMA traffic.
- Use Experiment 4 to study sustained completion discovery, reset, layout, and reuse policy effects.

## Current focused finding

The current detailed analysis is in `experiment_2_marker_overlap.md`: visible completion-status reads show a strong two-record/cacheline pattern. The working hypothesis is that the first CPU touch of a DSA-written 64-byte completion cacheline pays the coherence/visibility cost, while the second 32-byte completion record in the same line is often cheap. `experiment_2_marker_overlap::mechanism_probes` turns that hypothesis into runnable sub-experiments for layout, cacheline-pair, prefetch, measurement-method, and cache-state checks, each compared to one `baseline/packed-32b` row.
