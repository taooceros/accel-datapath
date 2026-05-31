# Experiment 5: submit-admission probe

## Selector and module

- CLI selector: `submit-admission`
- Result row: admission result rows
- Implementation: `experiment_5_submit_admission_probe.rs`

## Question

Does pushing past nominal WQ depth lose descriptors, report descriptor errors, or simply backpressure the submit loop?

## Shape

```text
blind push phase:       post-submit accounting:
[1][2][3] ... [N]  ->   completed / missing / errors
   no polling while submitting
```

The benchmark pushes `N` unique completion-bearing descriptors without software in-flight accounting during submission. It then scans completions afterward.

## Main controls

- `--submit-bursts <LIST>`: logical burst sizes to push.
- `--iterations <N>`: repeated samples per burst.

Typical focused command:

```text
launch ./target/release/hw-eval \
  --accel dsa \
  --benchmark submit-admission \
  --device /dev/dsa/wq0.0 \
  --json \
  --iterations 1000 \
  --submit-bursts 64,96,112,116,117,120,124,128,132,144,160 \
  --pin-core 0
```

## Measured signal

The result records submit timing plus post-submit completed, missing, and error counts. This is a correctness and backpressure probe rather than a sustained-throughput policy test.

## Notes

Use this experiment as the admission correctness gate. If descriptors are missing or erroring, later overlap or throughput interpretations are suspect. If completions are intact while submit latency bends, the likely effect is WQ admission backpressure rather than descriptor loss.
