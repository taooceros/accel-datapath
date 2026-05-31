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

The benchmark pre-fills `K` completion-bearing logical operations, then times exactly one additional submit. Batch size is one logical operation per MMIO submission.

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

For each occupancy, the benchmark records the latency of the one measured submit and completion accounting after the burst drains.

The primary signal is the submit-latency curve as `K` approaches and exceeds the practical admission/backpressure point.

## Notes

This experiment isolates submit admission pressure from active polling by measuring a single submit after prefill, then draining afterward.
