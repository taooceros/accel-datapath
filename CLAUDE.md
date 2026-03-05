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
- **Strategy taxonomy**: Check `cpp/benchmark/dsa/strategies/README.md` before adding or modifying benchmark strategies.

## Project Overview

Two-track research project for Intel Data Streaming Accelerator (DSA) and IAX integration:

1. **C++ track** (`cpp/`): stdexec sender/receiver bindings for DSA. Primary goal: maximize message rate (ops/sec) for small transfers using inline polling. xmake build system; TOML-based benchmark config.
2. **Rust track** (`rust/`): Accelerator-driven gRPC using Tonic. Goal: offload gRPC data path (memcpy, CRC, compression) to DSA and IAX hardware via the Rust async framework.

## Repository Structure

```
cpp/                                C++ stdexec track
  src/dsa/                          Low-level DSA hardware interface
  include/dsa_stdexec/              stdexec sender/receiver integration
  benchmark/dsa/                    Multi-dimensional benchmark suite
  examples/                         Per-operation examples
  test/                             Unit + integration tests
  tools/                            dsa_launcher capability wrapper
  xmake.lua                         Build configuration

rust/                               Rust async framework track
  tonic/                            Submodule: taooceros/tonic fork
  accel-codec/                      Custom Tonic Codec with pooled buffers
  accel-middleware/                  Tower CRC/compression middleware
  dsa-ffi/                          FFI bridge to C++ DSA
  iax-ffi/                          FFI bridge to IAX
  async-bench/                      Async framework overhead characterization
  tonic-profile/                    Tonic profiling harness
  Cargo.toml                        Workspace root

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

### C++ Track

```bash
devenv shell                                    # Nix development shell
cd cpp
xmake                                           # Build all targets
xmake build dsa_benchmark                       # Build specific target
xmake f -m release && xmake                     # Build modes: debug/release/profile
xmake f --policies=build.sanitizer.address && xmake  # ASan
run                                             # Run benchmarks (auto dsa_launcher + build mode)
run -- --help                                   # Benchmark CLI help
dsa_launcher ./build/.../example_data_move      # Run any binary with CAP_SYS_RAWIO
```

C++23, GCC 15, mold linker. Flags `-menqcmd` and `-mmovdir64b` required for DSA intrinsics.

### Rust Track

```bash
cd rust
cargo build                                     # Build all crates
cargo check                                     # Type-check workspace
cargo bench                                     # Run benchmarks (async-bench)
```

### C++ Build Targets

| Target | Description |
|--------|-------------|
| `dsa-stdexec` | Main executable (all `cpp/src/**/*.cpp`) |
| `dsa_benchmark` | Multi-dimensional benchmark suite |
| `task_queue_benchmark` | Task queue synchronization benchmarks |
| `dsa_launcher` | C11 capability launcher (see `cpp/tools/README.md`) |
| `example_<op>` | One per op: `data_move`, `mem_fill`, `compare`, `compare_value`, `dualcast`, `crc_gen`, `copy_crc`, `cache_flush` |

## C++ Architecture

Per-module READMEs with file tables, design notes, and extension guides:

| README | Covers |
|--------|--------|
| `cpp/src/dsa/README.md` | DsaEngine, task queues, descriptor submitters, alignment |
| `cpp/include/dsa_stdexec/README.md` | stdexec integration, PollingRunLoop, senders |
| `cpp/include/dsa_stdexec/operations/README.md` | Per-operation sender pattern, how to add ops |
| `cpp/benchmark/dsa/README.md` | Benchmark framework, config, dispatch |
| `cpp/benchmark/dsa/strategies/README.md` | Strategy taxonomy, decision guide, perf reference |
| `cpp/examples/README.md` | Quick-start examples |
| `cpp/test/README.md` | Test suite coverage |
| `cpp/tools/README.md` | dsa_launcher capability model |
| `dsa-config/README.md` | accel-config device configurations |

Design decisions: `report/design_decisions.md`. Hardware spec: `dsa_architecture_spec.md`.

## Benchmark Rules

Always: (1) `--output <unique_filename>.csv` to avoid overwriting, (2) check `--help` before running, (3) preserve all CSV outputs. Visualize with `cpp/benchmark/visualize_interactive.py`.

## Dependencies

**C++ Deps** (managed via Nix flake): stdexec, libaccel-config, fmt, proxy, tomlplusplus.
**Rust Deps** (managed via Cargo): tonic, tokio, prost, bytes, tower.

## Code Rules

- Match CODE to SPEC, not spec to code, unless told otherwise.
- In multi-agent setups, minimize idle messages. Only send updates on meaningful progress or when blocked.
