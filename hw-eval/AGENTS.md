# hw-eval AGENTS

Inherits `../AGENTS.md`.

## OVERVIEW
Raw DSA/IAX benchmark crate. Measures hardware submission/completion costs with minimal framework overhead.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| CLI and benchmark matrix | `README.md`, `src/main.rs` | Entry point for modes and output. |
| Shared submission helpers | `src/submit.rs` | Portal, polling, timing, topology. |
| DSA path | `src/dsa.rs` | DSA descriptors and helpers. |
| IAX path | `src/iax.rs` | IAX descriptors, completions, CRC64 flow. |
| Software baselines | `src/sw.rs` | Non-hardware fallback path. |
| Criterion bench | `benches/dsa_raw.rs` | Software-only bench target. |
| Bindings dependency | `../idxd-sys/Cargo.toml` | Root-level canonical raw IDXD/UAPI/MMIO crate shared with `idxd-rust`. |

## CONVENTIONS
- Use `launch` for hardware-facing runs; use `--sw-only` when hardware is not required.
- Build and measure release binaries only; debug builds are not performance data.
- Keep DSA and IAX benchmark matrices distinct; the two paths are intentionally different.
- Preserve JSON output and graphing compatibility when changing benchmark result shapes.

## HARDWARE RUNS
- From the repo root, build with `cargo build --release -p hw-eval`; the workspace binary is `./target/release/hw-eval`. If your cwd is `hw-eval/`, use `../target/release/hw-eval`.
- For hardware performance tests, run the built binary through `launch`, not `cargo run`, so the benchmark child receives `CAP_SYS_RAWIO`. Use `cargo run --release -p hw-eval -- --sw-only` only for software baselines.
- Use explicit `--accel`, `--device`, `--iterations`, `--sizes`, and `--json` for comparable hardware results. Add `--pin-core <N>` when comparing runs; add `--cold` only when evaluating cold-cache DMA behavior.
- If `launch` or device setup fails, stop and inspect `tools/README.md` / local DSA-IAX setup instead of running the hardware binary directly.

## BATCHING, CONCURRENCY, AND CLI TRIGGERS
- There is no `--strategy`, `--benchmark`, or `--batch-size` CLI flag today. Strategy selection is by `--accel`; the binary runs the hard-coded suite for that accelerator.
- In terminology, batch size is operations per MMIO submission; concurrency is maximum logical operations outstanding. Keep those dimensions separate when describing results.
- Direct/no-MMIO-batch paths submit one operation per MMIO (`batch_size = 1` conceptually). In JSON these rows use `batch_size: null`; examples are `memmove`, `copy_crc`, and `burst_memmove`.
- DSA hardware-batch paths submit one DSA `BATCH` descriptor per MMIO; that descriptor points to `batch_n` sub-descriptors. Examples are `batch_memmove`, `pipelined_batch_b<N>`, and `burst_batch_b<N>`.
- `--accel dsa` triggers the DSA suite, including direct/no-batch throughput and DSA hardware-batch measurements.
- `--accel iax` triggers the IAX CRC64 suite, including `burst_crc64` and sliding-window `crc64`; it does not run DSA hardware batch descriptor strategies.
- To focus a batch experiment from the command line, narrow `--sizes`, set `--iterations`, cap the sweep with `--max-concurrency`, write `--json`, and filter rows by JSON `benchmark` name.
- `--max-concurrency` caps the benchmark's outstanding submission slots. For direct/no-batch rows, logical concurrency equals the JSON `concurrency` value. For `pipelined_batch_b<N>` and `burst_batch_b<N>`, logical concurrency is `N * concurrency` because each outstanding batch descriptor contains `N` operations.

## HW-EVAL STRATEGY MEANINGS
- Direct sliding window (`memmove`, `copy_crc`): keep up to `concurrency` independent descriptors in flight, submitting a replacement as each completion arrives. Batch size is 1 operation/MMIO.
- Direct burst (`burst_memmove`): submit `burst_size` independent descriptors first, then wait for all completions. Batch size is 1 operation/MMIO; maximum outstanding operations equals `burst_size`.
- Batch latency (`batch_memmove`): submit one DSA `BATCH` descriptor and measure completion latency for its `batch_n` memmove sub-descriptors. Batch size is `batch_n`; there is one MMIO submission for the whole batch.
- Pipelined batch (`pipelined_batch_b<N>`): sliding window where each in-flight submission is a DSA hardware batch descriptor with `N` operations. Batch size is `N`; logical outstanding operations are `N * window_slots`.
- Burst batch (`burst_batch_b<N>`): burst where each submitted item is a DSA hardware batch descriptor with `N` operations. Batch size is `N`; logical outstanding operations are `N * burst_size`.

## ANTI-PATTERNS
- Do not run the hardware binary directly when the documented flow requires `launch`.
- Do not treat debug-build measurements as hardware performance data.

## COMMANDS
```bash
# Build the release binary in the workspace target directory.
cargo build --release -p hw-eval

# Software-only smoke test; no hardware or launch required.
cargo run --release -p hw-eval -- --sw-only

# Hardware smoke test from the repo root.
launch ./target/release/hw-eval --accel dsa --device /dev/dsa/wq0.0 --json --iterations 100 --sizes 64 --max-concurrency 4

# Hardware performance sweeps from the repo root.
launch ./target/release/hw-eval --accel dsa --device /dev/dsa/wq0.0 --json --iterations 3000 --sizes 64,256,1024,4096,16384,65536 > results/hw-eval-dsa.json
launch ./target/release/hw-eval --accel iax --device /dev/iax/wq1.0 --json --iterations 3000 --sizes 64,256,1024,4096,16384,65536 > results/hw-eval-iax.json

# Focus DSA batch/throughput strategies: this runs the DSA suite, including batch_memmove, pipelined_batch_b<N>, burst_batch_b<N>, burst_memmove, and sliding-window memmove/copy_crc.
launch ./target/release/hw-eval --accel dsa --device /dev/dsa/wq0.0 --json --iterations 3000 --sizes 4096 --max-concurrency 32 > results/hw-eval-dsa-strategies.json

# Optional JSON filters for individual strategy families.
jq '.latency[] | select(.benchmark == "batch_memmove")' results/hw-eval-dsa-strategies.json
jq '.throughput[] | select(.benchmark | startswith("pipelined_batch_b"))' results/hw-eval-dsa-strategies.json
jq '.throughput[] | select(.benchmark | startswith("burst_batch_b"))' results/hw-eval-dsa-strategies.json
jq '.throughput[] | select(.benchmark == "burst_memmove" or .benchmark == "memmove" or .benchmark == "copy_crc")' results/hw-eval-dsa-strategies.json

# Criterion benchmarks cover software baselines only.
cargo bench -p hw-eval
```
