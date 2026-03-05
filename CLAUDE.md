# CLAUDE.md

Guidance for Claude Code working in this repository.

## Workflow Rules

- **Plans**: Write to `plan/YYYY-MM-DD/<topic>.md` before non-trivial changes.
- **Reports**: Write findings to `report/<descriptive_name>.md`.
- **Remarks**: Write concise, standalone insights to `remark/NNN_<topic>.md`. Each remark captures one interesting finding with data, explanation, and implication. Number sequentially. Reference source report.
- **Early Hypotheses**: Deliver a preliminary analysis within the first 30 seconds. State what you've found so far if you need more time.
- **Read before modify**: Read the module's co-located README before changing any module. They document patterns, conventions, and extension steps.
- **Check prior work**: Scan `report/*.md` before starting analysis — previous sessions may have covered the topic.
- **Design decisions first**: Check `report/design_decisions.md` before proposing architectural changes to understand existing rationale.
- **Spec over guessing**: For hardware behavior questions (opcodes, completion records, page faults), consult `dsa_architecture_spec.md` rather than guessing.
- **Strategy taxonomy**: Check `dsa-stdexec/benchmark/dsa/strategies/README.md` before adding or modifying benchmark strategies.

## Project Overview

Research project for Intel hardware accelerator (DSA + IAX) integration into data path systems:

1. **dsa-stdexec/** — C++ stdexec sender/receiver framework for DSA. Maximize message rate (ops/sec) for small transfers using inline polling. xmake build; TOML-based benchmark config.
2. **accel-rpc/** — Accelerator-driven gRPC using Tonic (Rust). Offload gRPC data path (memcpy, CRC, compression) to DSA/IAX via the Rust async framework.
3. **hw-eval/** — Raw hardware performance evaluation (Rust). Zero-framework-overhead DSA/IAX benchmarks. Calls hardware directly via inline asm (MOVDIR64B/ENQCMD). Establishes true hardware floor.

## Repository Structure

```
dsa-stdexec/                        C++ stdexec framework
  src/dsa/                          Low-level DSA hardware interface
  include/dsa_stdexec/              stdexec sender/receiver integration
  benchmark/dsa/                    Multi-dimensional benchmark suite
  examples/                         Per-operation examples
  test/                             Unit + integration tests
  tools/                            dsa_launcher capability wrapper
  xmake.lua                         Build configuration

accel-rpc/                          Accelerator-driven gRPC (Rust)
  tonic/                            Submodule: taooceros/tonic fork
  accel-codec/                      Custom Tonic Codec with pooled buffers
  accel-middleware/                  Tower CRC/compression middleware
  dsa-ffi/                          FFI bridge to C++ DSA
  iax-ffi/                          FFI bridge to IAX
  async-bench/                      Async framework overhead characterization
  tonic-profile/                    Tonic profiling harness
  Cargo.toml                        Workspace root

hw-eval/                            Raw hardware evaluation (Rust)
  src/dsa.rs                        DSA descriptors, WQ portal, inline asm
  src/main.rs                       Latency/throughput benchmarks + SW baselines
  benches/dsa_raw.rs                Criterion benchmarks
  Cargo.toml

Shared (repo root):
  dsa_architecture_spec.md          Hardware spec
  dsa-config/                       accel-config device configurations
  plan/                             Plans
  report/                           Reports
  remark/                           Insight remarks
  docs/                             Design documents
  devenv.nix                        Nix dev environment (C++ + Rust)
```

## Build

### C++ (dsa-stdexec/)

```bash
devenv shell                                    # Nix development shell
cd dsa-stdexec
xmake                                           # Build all targets
xmake build dsa_benchmark                       # Build specific target
xmake f -m release && xmake                     # Build modes: debug/release/profile
run                                             # Run benchmarks (auto dsa_launcher + build mode)
```

C++23, GCC 15, mold linker. Flags `-menqcmd` and `-mmovdir64b` required for DSA intrinsics.

### Rust (accel-rpc/)

```bash
cd accel-rpc
cargo build                                     # Build all crates
cargo check                                     # Type-check workspace
```

### Raw Hardware Eval (hw-eval/)

```bash
cd hw-eval
cargo build --release
# Run via dsa_launcher for CAP_SYS_RAWIO:
dsa_launcher ./target/release/hw-eval
# Software baselines only (no hardware needed):
cargo run --release -- --sw-only
# Criterion benchmarks:
dsa_launcher cargo bench
```

### C++ Build Targets

| Target | Description |
|--------|-------------|
| `dsa-stdexec` | Main executable (all `dsa-stdexec/src/**/*.cpp`) |
| `dsa_benchmark` | Multi-dimensional benchmark suite |
| `task_queue_benchmark` | Task queue synchronization benchmarks |
| `dsa_launcher` | C11 capability launcher (see `dsa-stdexec/tools/README.md`) |
| `example_<op>` | One per op: `data_move`, `mem_fill`, `compare`, `compare_value`, `dualcast`, `crc_gen`, `copy_crc`, `cache_flush` |

## C++ Architecture

Per-module READMEs:

| README | Covers |
|--------|--------|
| `dsa-stdexec/src/dsa/README.md` | DsaEngine, task queues, descriptor submitters, alignment |
| `dsa-stdexec/include/dsa_stdexec/README.md` | stdexec integration, PollingRunLoop, senders |
| `dsa-stdexec/include/dsa_stdexec/operations/README.md` | Per-operation sender pattern, how to add ops |
| `dsa-stdexec/benchmark/dsa/README.md` | Benchmark framework, config, dispatch |
| `dsa-stdexec/benchmark/dsa/strategies/README.md` | Strategy taxonomy, decision guide, perf reference |
| `dsa-stdexec/examples/README.md` | Quick-start examples |
| `dsa-stdexec/test/README.md` | Test suite coverage |
| `dsa-stdexec/tools/README.md` | dsa_launcher capability model |
| `dsa-config/README.md` | accel-config device configurations |

Design decisions: `report/design_decisions.md`. Hardware spec: `dsa_architecture_spec.md`.

## Benchmark Rules

Always: (1) `--output <unique_filename>.csv` to avoid overwriting, (2) check `--help` before running, (3) preserve all CSV outputs. Visualize with `dsa-stdexec/benchmark/visualize_interactive.py`.

## Dependencies

**C++ Deps** (managed via Nix flake): stdexec, libaccel-config, fmt, proxy, tomlplusplus.
**Rust Deps** (managed via Cargo): tonic, tokio, prost, bytes, tower, libc, clap, criterion.

## Code Rules

- Match CODE to SPEC, not spec to code, unless told otherwise.
- In multi-agent setups, minimize idle messages. Only send updates on meaningful progress or when blocked.
