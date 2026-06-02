# hw-eval

Raw hardware performance evaluation (Rust). Zero-framework-overhead DSA/IAX benchmarks. Calls hardware directly via inline asm (MOVDIR64B/ENQCMD). Establishes true hardware floor.

## Build

```bash
cd hw-eval
cargo build --release
# Run via launch script for CAP_SYS_RAWIO:
launch ./target/release/hw-eval
# Software baselines only (no hardware needed):
cargo run --release -- --sw-only
# Criterion benchmarks (SW baselines only):
cargo bench
```

## CLI Options

```
--accel <dsa|iax>       Accelerator backend (default: dsa)
--device, -d <PATH>      WQ device path (default: /dev/dsa/wq0.0 for dsa,
                         /dev/iax/wq1.0 for iax)
--sizes, -s <LIST>       Message sizes in bytes, comma-separated
--iterations, -i <N>     Iterations per measurement (default: 10000)
--max-concurrency, -m <N> Max sliding window concurrency (default: 128)
--benchmark <all|submit-only|submit-admission|submit-occupancy|submit-marker-overlap|submit-marker-mechanism|traffic-class-ladder|completion-reuse-policy>
                         Benchmark subset to run (default: all)
--submit-mode <all|unloaded|sustained|mfence>
                         Submit-only workload variant (default: all)
--submit-bursts <LIST>     Submit-only burst sizes (default:
                         1,2,4,8,16,32,64,128,256,512)
--submit-occupancies <LIST>
                         Prefill occupancies for submit-occupancy, zero allowed
                         (default: 0,32,64,96,112,120,124,126,127,128,129,132,136,144,160)
--marker-bursts <LIST>  Burst lengths for Experiment 2 overlap and mechanism rows
--marker-positions <LIST>
                         Marker positions: first,half,last
--marker-poll-cadences <LIST>
                         Legacy compatibility option; traced mode uses poll step 1
--marker-poll-offsets <LIST>
                         First zero-based submit indexes where Experiment 2 tracing/probes start polling
--marker-poll-submit-batches <LIST>
                         Submit-side poll intervals for Experiment 2 mechanism probes:
                         poll after every N submitted logical requests
--traffic-windows <LIST>
                         Windows for traffic-class-ladder
--traffic-classes <LIST>
                         Traffic classes: submit-only,noop-completion,memmove64,memmove4k
--completion-reuse-policies <LIST>
                         Completion policies: packed-scan,padded-round-robin,poll-only,delayed-reset,batch-harvest
--completion-reuse-window <N>
                         Fixed logical window for completion-reuse-policy
--dsa-op <noop|memmove64|memmove4k>
                         DSA operation class for bottleneck experiments
--dsa-memmove-bytes <N>
                         Override memmove byte count for bottleneck experiments
--submit-occupancy-trace-until <N>
                         For submit-occupancy, time every post-prefill submit until total submitted reaches N
--sw-only                Software baselines only (no hardware)
--pin-core <N>           Pin benchmark thread to CPU core
--cold                   Flush caches between iterations (cold-cache DMA)
--json                   Machine-readable JSON output
```

## Benchmarks

| Benchmark | What it measures |
|-----------|-----------------|
| **noop** | Pure submission + completion overhead (no data movement) |
| **submit_only_empty** | Empty submit-only calibration path; same timer wrapper, no MMIO submission |
| **submit_only_unloaded** | DSA NOOP submit burst only; drains with an out-of-region sentinel between samples |
| **submit_only_pressure_ramp** | DSA NOOP submit burst without per-sample draining; selected by `--submit-mode sustained` and exposes stateful backpressure |
| **submit_only_mfence** | DSA NOOP submit burst with `mfence` between submissions; probes posted-write serialization |
| **submit_admission_distinct** | Distinct completion-bearing NOOP burst with no software inflight gate; counts missing completions to test dedicated-WQ admission behavior |
| **memmove** | Single-op DMA copy latency (rdtscp, per size) |
| **submit_occupancy_one_extra** | Prefill K completion-bearing descriptors, time one extra submit, then record first-old completion visibility and drain counts |
| **submit_marker_overlap** | Default Experiment 2 suite: baseline overlap traces plus all mechanism-probe rows for the configured marker bursts and poll offsets |
| **submit_marker_mechanism** | Mechanism-only Experiment 2 sub-experiments for completion-record layout, cacheline-pair, prefetch, measurement-method, and cache-state probes |
| **traffic_class_ladder** | Compare submit-only, NOOP+completion, 64 B memmove, and 4 KiB memmove at identical windows |
| **completion_reuse_policy** | Compare packed scan, padded round-robin, poll-only, delayed reset, and batch-harvest completion reuse policies |
| **crc_gen** | Single-op CRC-32C generation latency |
| **copy_crc** | Single-op fused copy+CRC latency |
| **batch** | Batch descriptor latency (sweep batch_n=1..1024) |
| **sliding window** | Pipelined throughput (per-op buffers, concurrency 1..128) |
| **pipelined batch** | Sliding window of batch descriptors (batch_n × concurrency sweep) |
| **burst** | Submit N ops, wait all, repeat (no pipelining overlap) |
| **sw_memcpy** | Software memcpy baseline |
| **sw_crc32c** | Software CRC-32C (SSE4.2) baseline |

Backend notes:
- `dsa`: runs the full suite above.
- `iax`: runs `noop` plus `crc64` latency, burst throughput, and sliding-window
  throughput. The IAX path does not use the old `memmove` benchmark anymore.
- `dsa` and `iax` descriptor/completion layouts are sourced through the
  root-level `idxd-sys` crate, which runs bindgen against the local kernel
  `linux/idxd.h` at build time. `hw-eval` keeps benchmark-specific helper
  APIs, but the hardware ABI boundary is owned by `idxd-sys`.

## Timing

- Latency benchmarks use **rdtscp** by default for low-overhead TSC timing.
- Submit-only runs measure each workload four independent ways: TSC ticks,
  wall-clock nanoseconds, PMU core cycles via per-sample `perf_event_open`
  ioctl/read, and PMU core cycles via low-overhead `rdpmc`. Each timer source
  wraps a separate run of the same burst so the timers do not interfere with
  each other. JSON rows include an explicit `timer` field.
- Throughput benchmarks use **Instant::now** (amortized over many ops)
- TSC frequency auto-detected from /proc/cpuinfo

## DSA submission bottleneck experiments

DSA submission bottleneck runners are selected explicitly so their JSON rows are easy to find:

```bash
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

For payload-saturation probes, use `--dsa-op memmove64 --dsa-memmove-bytes <N>` to keep the memmove operation path while overriding the byte count.

JSON reports include skipped-when-empty arrays for these modes:
- `admission`: rows use `benchmark: "submit_admission_distinct"` and record burst size, submitted operations, completion counts, missing counts, descriptor errors, and submit timing for the Experiment 5 correctness gate.
- `submit_occupancy`: rows use `benchmark: "submit_occupancy_trace"` and record `operation_class`, `k_prefill`, `submitted`, `trace_until`, raw per-submit `extra_submit_trace` points, and per-iteration `trace_outcomes`. Percentiles and ranges are computed by post-processing scripts, not by the runner.
- `submit_marker_overlap`: rows use `benchmark: "submit_marker_overlap"` and record `n`, zero-based marker position, fixed poll step, zero-based poll offset, submit-tail timing, marker-visible timing when observed, completion counts, and a `trace` list keyed by zero-based `submit_index`. Each trace point records submit latency, visible prefix/count, and per-read poll latency/status stats for plotting.
- `traffic_class_ladder`: rows use `benchmark: "traffic_class_ladder"` and record traffic class, operation size, window, submit timing, completion-visible timing where applicable, completion counts where applicable, and ops/sec.
- `completion_reuse_policy`: rows use `benchmark: "completion_reuse_policy"` and record operation class, window, policy, operations completed, ops/sec, polls/completion, harvest timing, reset-to-submit timing where applicable, and completion counts.

## Graphing

Generate benchmark graphs from JSON output:

```bash
# Run benchmarks with JSON output
launch ./target/release/hw-eval --json --iterations 3000 \
  --sizes 64,256,1024,4096,16384,65536 > results.json

# Generate PNG graphs
python3 plot_results.py results.json --outdir graphs/
```

Produces 5 graphs in `graphs/`:
- **latency_vs_size.png** — Single-op latency + effective bandwidth vs message size
- **throughput_vs_concurrency.png** — Mops/sec vs concurrency for sliding window, burst, pipelined batch
- **batch_amortization.png** — Per-op latency vs batch size (shows submission overhead amortization)
- **pipelined_batch_heatmap.png** — batch_size × concurrency heatmap of Mops/sec
- **strategy_comparison.png** — Peak Mops/sec bar chart comparing all three strategies

Requires: `pip install matplotlib numpy`

## Structure

```
src/submit.rs                     Shared WQ submission and low-level polling/timing/topology helpers
src/dsa.rs                        DSA helper façade over root-level idxd-sys descriptor/completion ABI
src/iax.rs                        IAX helper façade over root-level idxd-sys descriptor/completion ABI
src/sw.rs                         Software memcpy/CRC baselines
src/benchmarks.rs                 Benchmark implementation module root
src/benchmarks/dsa.rs             DSA hardware benchmark suite and dispatcher
src/benchmarks/iax.rs             IAX hardware benchmark suite
src/benchmarks/software.rs        Software baseline benchmark suite
src/benchmarks/submission_bottleneck.rs  Submission bottleneck experiment module root
src/benchmarks/submission_bottleneck/experiment_N_*.rs
                                  Numbered submission bottleneck experiment modules
src/benchmarks/submission_bottleneck/common.rs
                                  Shared submission bottleneck helpers
src/main.rs                       CLI parsing and top-level benchmark orchestration
benches/dsa_raw.rs                Criterion benchmarks (SW baselines only)
plot_results.py                   Matplotlib graphing script
Cargo.toml
```

## Dependencies

Managed via Cargo: libc, clap, serde, serde_json, criterion (dev), and the
root-level `idxd-sys` crate for DSA/IAX UAPI access.
Build-time for DSA/IAX bindings: `idxd-sys` uses `bindgen` plus a working
`libclang`, reading `/usr/include/linux/idxd.h` by default (override with
`IDXD_HEADER`).
Graphing: matplotlib, numpy (Python 3).
