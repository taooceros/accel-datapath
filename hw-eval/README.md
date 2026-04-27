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
--sw-only                Software baselines only (no hardware)
--pin-core <N>           Pin benchmark thread to CPU core
--cold                   Flush caches between iterations (cold-cache DMA)
--json                   Machine-readable JSON output
```

## Benchmarks

| Benchmark | What it measures |
|-----------|-----------------|
| **noop** | Pure submission + completion overhead (no data movement) |
| **memmove** | Single-op DMA copy latency (rdtscp, per size) |
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
- `iax` descriptor/completion layouts are sourced through the sibling
  `idxd-sys` crate, which runs bindgen against the local kernel
  `linux/idxd.h` at build time.

## Timing

- Latency benchmarks use **rdtscp** for cycle-accurate measurement (~7ns overhead vs ~30ns for Instant::now)
- Throughput benchmarks use **Instant::now** (amortized over many ops)
- TSC frequency auto-detected from /proc/cpuinfo

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
src/dsa.rs                        DSA-specific descriptors/completions/opcodes/helpers
src/iax.rs                        IAX-specific descriptors/completions/opcodes/helpers
src/sw.rs                         Software memcpy/CRC baselines
src/main.rs                       All benchmarks, CLI, JSON output
benches/dsa_raw.rs                Criterion benchmarks (SW baselines only)
plot_results.py                   Matplotlib graphing script
Cargo.toml
```

## Dependencies

Managed via Cargo: libc, clap, serde, serde_json, criterion (dev), and the
root-level `idxd-sys` crate for IAX UAPI access.
Build-time for IAX bindings: `idxd-sys` uses `bindgen` plus a working
`libclang`, reading `/usr/include/linux/idxd.h` by default (override with
`IDXD_HEADER`).
Graphing: matplotlib, numpy (Python 3).
