# Experiment 1: submit occupancy

## Selector and module

- CLI selector: `submit-occupancy`
- Result row: `submit_occupancy_one_extra`
- Implementation: `experiment_1_submit_occupancy.rs`

## Question

Does one more MMIO submit stay cheap, or bend as prefilled outstanding occupancy consumes the DSA admission/credit domain?

## Shape

```text
WQ before measured submit:
[op 0][op 1] ... [op K-1]  +  [extra op]
     outstanding K              measured submit
```

The benchmark pre-fills `K` completion-bearing logical operations, then times the post-prefill submit trace. Batch size is one logical operation per MMIO submission. Optional `--submit-occupancy-gap-tsc` waits for a target number of TSC ticks between submissions, outside the timed submit call, so the submit loop can be manually paced when comparing `noop` against large `memmove` work.

## Main controls

- `--submit-occupancies <LIST>`: prefill occupancies, including zero.
- `--dsa-op <noop|memmove64|memmove4k>`: logical operation class.
- `--iterations <N>`: repeated samples per occupancy.
- `--submit-occupancy-spin-iters <N>`: fixed `for n in 0..N { black_box(n); }` loop iterations inserted between submissions; default `0`.
- `--submit-occupancy-gap-tsc <N>`: preferred pacing knob; wait until about `N` TSC ticks have elapsed between submissions; default `0`. Mutually exclusive with `--submit-occupancy-spin-iters`.
- `--submit-occupancy-shared-payload`: for large memmove payload sweeps, reuse one source/destination payload buffer across descriptors while keeping completions distinct; default `false`.

Typical focused command:

```text
launch ./target/release/hw-eval \
  --accel dsa \
  --benchmark submit-occupancy \
  --device /dev/dsa/wq0.0 \
  --json \
  --iterations 1000 \
  --submit-occupancies 0,32,64,96,112,120,124,126,127,128,129,132,136,144,160 \
  --dsa-op noop \
  --pin-core 0 \
  --submit-occupancy-gap-tsc 0
```

## Measured signal

For each occupancy, the benchmark records:

- the latency of the measured submit trace, with `spin_loop_iters`, `gap_tsc_ticks`, and `shared_payload` recorded for plot grouping;
- per-iteration prefill submit-loop time, including configured inter-submit pacing gaps;
- per-iteration elapsed time from the first prefill submit until all `K` prefill completions are observed;
- completion accounting after the burst drain attempt, including `hardware_observed`, `drain_sentinel_completed`, and `drain_sentinel_status` for rows where dedicated-WQ submissions appear to have been dropped or cannot be drained.

The primary signal is the submit-latency curve as `K` approaches and exceeds the practical admission/backpressure point. The prefill completion trace is the companion signal for whether `K` requests can complete on a timescale comparable to the prefill submission loop. A small `post_submit_completion_*` value means the hardware was already close to, or at, completion by the time software finished submitting the prefill.

## Notes

This experiment isolates submit admission pressure from active polling by measuring submit calls after prefill, then draining afterward. The manual pacing options change the time available for hardware progress between submissions; they deliberately do not add the gap cost to each recorded `submit_tsc_ticks` sample.
