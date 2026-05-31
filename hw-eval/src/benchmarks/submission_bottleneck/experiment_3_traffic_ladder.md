# Experiment 3: traffic-class ladder

## Selector and module

- CLI selector: `traffic-class-ladder`
- Result row: `traffic_class_ladder`
- Implementation: `experiment_3_traffic_ladder.rs`

## Question

Which added traffic class first changes the submit/completion curve?

## Shape

```text
SubmitOnly
   |
   v
NoopCompletion  (+ completion writes)
   |
   v
Memmove64       (+ tiny payload DMA)
   |
   v
Memmove4K       (+ larger payload DMA)
```

The benchmark keeps the logical window shape comparable while adding hardware traffic one class at a time. Batch size remains one logical operation per MMIO submission for completion-bearing classes.

## Main controls

- `--traffic-windows <LIST>`: logical windows to test.
- `--traffic-classes <submit-only|noop-completion|memmove64|memmove4k>`: traffic classes to include.
- `--iterations <N>`: repeated samples per row.

Typical focused command:

```text
launch ./target/release/hw-eval \
  --accel dsa \
  --benchmark traffic-class-ladder \
  --device /dev/dsa/wq0.0 \
  --json \
  --iterations 1000 \
  --traffic-windows 1,16,32,64,96,112,120,128,144,160 \
  --traffic-classes submit-only,noop-completion,memmove64,memmove4k \
  --pin-core 0
```

## Measured signal

The result separates submit-only pressure from completion-write pressure and payload-DMA pressure. A change that appears at `noop-completion` points at completion-write/coherence cost; a change that appears only at `memmove64` or `memmove4k` points at payload DMA traffic.

## Notes

Use this experiment to avoid attributing all slowdown to MMIO admission when completion writes or DMA payload traffic are the first changing variable.
