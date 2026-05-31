# Experiment 4: completion reuse policy

## Selector and module

- CLI selector: `completion-reuse-policy`
- Result row: `completion_reuse_policy`
- Implementation: `experiment_4_completion_reuse.rs`

## Question

Is sustained batch-size-1 throughput limited by CPU completion discovery, reset timing, cacheline layout, or resubmit policy?

## Shape

```text
fill window
   |
   v
[poll / harvest completions] -> [reset completion records] -> [resubmit]
   ^                                                            |
   |---------------- closed-loop steady window -----------------|
```

The benchmark holds a fixed logical window open, then varies how software discovers, resets, and resubmits completed slots.

## Main controls

- `--completion-reuse-policies <LIST>`: policy selector list (`packed-scan`, `padded-round-robin`, `poll-only`, `delayed-reset`, `batch-harvest`).
- `--completion-reuse-window <N>`: fixed logical window.
- `--dsa-op <noop|memmove64|memmove4k>`: logical operation class.
- `--iterations <N>`: repeated steady-window samples.

Typical focused command:

```text
launch ./target/release/hw-eval \
  --accel dsa \
  --benchmark completion-reuse-policy \
  --device /dev/dsa/wq0.0 \
  --json \
  --iterations 1000 \
  --completion-reuse-window 128 \
  --completion-reuse-policies packed-scan,padded-round-robin,poll-only,delayed-reset,batch-harvest \
  --dsa-op noop \
  --pin-core 0
```

## Measured signal

This experiment is for sustained closed-loop throughput after initial fill. Compare policies to isolate whether packed completion scanning, padded completion layout, reset timing, or round-robin slot reuse changes the steady-state rate.

## Notes

The cacheline-layout dimension in this experiment is directly relevant to Experiment 2's completion-record cacheline hypothesis. If padding completion records changes sustained throughput or poll latency, it strengthens the argument that completion cacheline ownership/visibility is load-bearing.
