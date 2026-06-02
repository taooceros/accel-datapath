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

The benchmark pre-fills `K` completion-bearing logical operations, then times exactly one additional submit. Batch size is one logical operation per MMIO submission. It also runs a separate prefill-only completion probe after the normal measured iteration so the primary occupancy signal is not changed.

## Main controls

- `--submit-occupancies <LIST>`: prefill occupancies, including zero.
- `--dsa-op <noop|memmove64|memmove4k>`: logical operation class.
- `--iterations <N>`: repeated samples per occupancy.

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
  --pin-core 0
```

## Measured signal

For each occupancy, the benchmark records:

- the latency of the measured submit trace;
- per-iteration prefill submit-loop time for the companion prefill-only probe;
- per-iteration elapsed time from the first prefill submit until all `K` prefill completions are observed;
- completion accounting after the burst drains.

The primary signal is the submit-latency curve as `K` approaches and exceeds the practical admission/backpressure point. The prefill completion trace is the companion signal for whether `K` requests can complete on a timescale comparable to the prefill submission loop. A small `post_submit_completion_*` value means the hardware was already close to, or at, completion by the time software finished submitting the prefill.

## Notes

This experiment isolates submit admission pressure from active polling by measuring a single submit after prefill, then draining afterward.
