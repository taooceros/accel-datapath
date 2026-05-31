# Experiment 2: marker overlap

## Selector and module

- CLI selector: `submit-marker-overlap`
- Result row: `submit_marker_overlap`
- Implementation: `experiment_2_marker_overlap.rs`

## Question

While the submit loop approaches the admission wall, when do early completions become visible and what does each completion-status read cost?

## Shape

```text
submit order:
[0: marker] [1] [2] [3] ... [poll_offset] [poll_offset+1] ... [N-1]
    ^ trace submit latency at every index   ^ poll next unfinished completion
```

The benchmark submits `N` logical operations one per MMIO. After `poll_offset`, it polls the next unfinished completion record while the submit loop continues. The current traced path uses fixed poll step 1.

## Main controls

- `--marker-bursts <LIST>`: burst lengths `N`.
- `--marker-positions <first|half|last>`: marker position in CPU submit order.
- `--marker-poll-offsets <LIST>`: first zero-based submit indices where polling begins.
- `--dsa-op <noop|memmove64|memmove4k>`: logical operation class.
- `--iterations <N>`: repeated samples per configuration.

Latest focused command:

```text
launch ./target/release/hw-eval \
  --accel dsa \
  --benchmark submit-marker-overlap \
  --device /dev/dsa/wq0.0 \
  --json \
  --iterations 1000 \
  --marker-bursts 160 \
  --marker-positions first \
  --marker-poll-offsets 1 \
  --dsa-op noop \
  --pin-core 0
```

## Current artifacts

- No-`black_box` JSON: `docs/report/benchmarking/submission_bottleneck_2026-05-31/marker_trace_no_black_box_offset1_noop.json`
- Nanosecond latency-order plot: `docs/report/benchmarking/submission_bottleneck_2026-05-31/submit_poll_trace_no_black_box_offset1_noop_latency_order_ns.png`
- Interpretation report: `docs/report/benchmarking/033.experiment2_latency_ns_interpretation_2026-05-31.md`

## Nanosecond latency result

The no-`black_box` run reported `tsc_freq_hz = 2,202,533,683`, so `1 TSC tick = 0.454 ns`.

From all 1000 per-iteration traces:

| class | median | p90 | p99 |
|---|---:|---:|---:|
| `NONE` status reads | 10.9 ns | 10.9 ns | 11.8 ns |
| visible status reads | 104.4 ns | 247.0 ns | 359.6 ns |
| visible read order 1 | 105.3 ns | 126.2 ns | 301.5 ns |
| visible read order 2 | 125.3 ns | 313.3 ns | 382.3 ns |
| visible read order 4 | 188.0 ns | 239.7 ns | 289.7 ns |

The high visible-read latency remained after replacing `core::hint::black_box(poll.value)` with direct `poll.value` use, so `black_box` is not the source.

## Completion-record layout hypothesis

`OperationSlots` stores completion records in a `Vec<DsaCompletionRecord>`. `DsaCompletionRecord` wraps the 32-byte DSA UAPI completion record with `#[repr(C, align(32))]`, so the vector stores records contiguously at 32-byte stride.

A 64-byte cache line therefore usually contains two completion records. This run showed a strong pair pattern consistent with `comp[0]` being 64-byte aligned:

```text
line 0: comp[0], comp[1]
line 1: comp[2], comp[3]
line 2: comp[4], comp[5]
```

Adjacent visible reads in the same poll event, under that `(even, odd)` pairing:

| position in pair | median | p90 | p99 |
|---|---:|---:|---:|
| lower/even request | 133.5 ns | 329.6 ns | 388.6 ns |
| following odd request | 10.9 ns | 10.9 ns | 119.9 ns |

Examples from sample iteration 850:

```text
r22: 291 ns  -> r23: 9 ns
r40: 284 ns  -> r41: 9 ns
r78: 302 ns  -> r79: 11 ns
r84: 333 ns  -> r85: 10 ns
r116: 371 ns -> r117: 9 ns
r128: 302 ns -> r129: 9 ns
```

Working conclusion: the expensive visible read is usually the first CPU touch of a DSA-written 64-byte completion cacheline. The second completion record in the same line is often cheap because the line is already visible to the CPU.

## Page-alignment note

A page-alignment issue is possible but does not explain the dominant pattern. Since a 4 KiB page is divisible by 32 and 64, a 32-byte completion record does not straddle a page boundary when the vector base is 32-byte aligned. For `N = 160`, the completion array occupies `160 * 32 B = 5120 B`, so it spans more than one page, but page boundaries should cause localized anomalies. The observed every-other-record pattern is better explained by cacheline sharing.

## Mechanism-probe implementation

The follow-up checks are implemented as the `mechanism_probes` submodule of Experiment 2 (`experiment_2_marker_overlap/mechanism_probes.rs`) and documented in `experiment_2_marker_overlap/mechanism_probes.md`.

Use:

```text
launch ./target/release/hw-eval \
  --accel dsa \
  --benchmark submit-marker-mechanism \
  --device /dev/dsa/wq0.0 \
  --json \
  --iterations 1000 \
  --marker-bursts 160 \
  --marker-poll-offsets 1 \
  --dsa-op noop \
  --pin-core 0
```

The mechanism rows record:

```text
completions.as_ptr() % 64
completions.as_ptr() % 4096
```

They also compare packed 32-byte completion records against 64-byte-padded records, cacheline-position latency, prefetch distance, per-read vs batch-scan timing, and reset/pre-touch/clflush cache state.
