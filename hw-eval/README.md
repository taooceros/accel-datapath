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
# Criterion benchmarks:
launch cargo bench
```

## Structure

```
src/dsa.rs                        DSA descriptors, WQ portal, inline asm
src/main.rs                       Latency/throughput benchmarks + SW baselines
benches/dsa_raw.rs                Criterion benchmarks
Cargo.toml
```

## Dependencies

Managed via Cargo: libc, clap, criterion.
