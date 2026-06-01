# Experiment 2 mechanism probes

## Role

This module contains runnable sub-experiments for Experiment 2. They are not a new top-level research question; they explain the mechanism behind the `submit-marker-overlap` result.

Primary design report: `docs/report/benchmarking/034.experiment2_mechanism_probes_design_2026-05-31.md`.

## Selector and result row

- Mechanism-only CLI selector: `submit-marker-mechanism`
- Result row: `submit_marker_mechanism`
- Implementation: private child module `experiment_2_marker_overlap::mechanism_probes` (`experiment_2_marker_overlap/mechanism_probes.rs`), exported through the outer Experiment 2 facade

The default Experiment 2 selector, `submit-marker-overlap`, also runs these mechanism rows after the baseline overlap rows. Use `submit-marker-mechanism` only for mechanism-only reruns.

## Sub-experiment matrix

| Sub-experiment | Variant examples | Question |
|---|---|---|
| `baseline` | `packed-32b` | What is the packed, reset-only, no-prefetch, per-read timing baseline? |
| `layout` | `padded-64b` | Does one-completion-per-cacheline close or widen the gap against baseline? |
| `prefetch` | `prefetch-1-lines`, `prefetch-2-lines`, `prefetch-4-lines` | Can CPU prefetch hide DSA-written line visibility cost? |
| `measurement` | `batch-scan-timing` | Does avoiding per-read `rdtscp` change scan cost against baseline? |
| `cache-state` | `pre-touch`, `clflush` | Does CPU cache state before DSA writes change first-visible cost? |

## Output interpretation

Each result row records completion-storage stride, nominal alignment, base pointer modulo 64 and 4096, completion outcome counts, and latency statistics split by status and cacheline position. Non-baseline rows also include `baseline_comparison`, which reports median nanosecond deltas and ratios against the single `baseline/packed-32b` row for the same burst and poll offset.

For packed 32-byte completion storage, line position is computed from the actual completion base address. Do not assume request parity alone; use the emitted alignment fields to interpret whether pairs are `(0,1)`, `(1,2)`, or shifted by allocation.

## Invariants

- Batch size remains one logical operation per MMIO submission.
- `n` remains the number of logical descriptors submitted in the burst.
- The poll loop still follows the Experiment 2 next-unfinished frontier model.
- Existing `submit_marker_overlap` row shape is not modified by this module; the default suite appends `submit_marker_mechanism` rows as a separate result family.
