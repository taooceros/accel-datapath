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
- **Strategy taxonomy**: Check `benchmark/dsa/strategies/README.md` before adding or modifying benchmark strategies.

## Project Overview

C++ sender/receiver (stdexec) bindings for Intel Data Streaming Accelerator (DSA). Primary goal: **maximize message rate** (ops/sec) for small transfers using inline polling. xmake build system; TOML-based benchmark config. Always run `xmake build` to verify changes compile.

## Build

```bash
devenv shell                                    # Nix development shell
xmake                                           # Build all targets
xmake build dsa_benchmark                       # Build specific target
xmake f -m release && xmake                     # Build modes: debug/release/profile
xmake f --policies=build.sanitizer.address && xmake  # ASan
run                                             # Run benchmarks (auto dsa_launcher + build mode)
run -- --help                                   # Benchmark CLI help
dsa_launcher ./build/.../example_data_move      # Run any binary with CAP_SYS_RAWIO
```

C++23, GCC 15, mold linker. Flags `-menqcmd` and `-mmovdir64b` required for DSA intrinsics.

### Build Targets

| Target | Description |
|--------|-------------|
| `dsa-stdexec` | Main executable (all `src/**/*.cpp`) |
| `dsa_benchmark` | Multi-dimensional benchmark suite |
| `task_queue_benchmark` | Task queue synchronization benchmarks |
| `dsa_launcher` | C11 capability launcher (see `tools/README.md`) |
| `example_<op>` | One per op: `data_move`, `mem_fill`, `compare`, `compare_value`, `dualcast`, `crc_gen`, `copy_crc`, `cache_flush` |

## Architecture

```
examples/ & src/main.cpp          Application layer
benchmark/dsa/                    Multi-dimensional benchmark suite
include/dsa_stdexec/              stdexec sender/receiver integration
src/dsa/                          Low-level DSA hardware interface
```

Per-module READMEs with file tables, design notes, and extension guides:

| README | Covers |
|--------|--------|
| `src/dsa/README.md` | DsaEngine, task queues, descriptor submitters, alignment |
| `include/dsa_stdexec/README.md` | stdexec integration, PollingRunLoop, senders |
| `include/dsa_stdexec/operations/README.md` | Per-operation sender pattern, how to add ops |
| `benchmark/dsa/README.md` | Benchmark framework, config, dispatch |
| `benchmark/dsa/strategies/README.md` | Strategy taxonomy, decision guide, perf reference |
| `examples/README.md` | Quick-start examples |
| `test/README.md` | Test suite coverage |
| `tools/README.md` | dsa_launcher capability model |
| `dsa-config/README.md` | accel-config device configurations |

Design decisions: `report/design_decisions.md`. Hardware spec: `dsa_architecture_spec.md`.

## Benchmark Rules

Always: (1) `--output <unique_filename>.csv` to avoid overwriting, (2) check `--help` before running, (3) preserve all CSV outputs. Visualize with `benchmark/visualize_interactive.py`.

## Dependencies

**Deps** (managed via Nix flake): stdexec, libaccel-config, fmt, proxy, tomlplusplus.

## Code Rules

- Match CODE to SPEC, not spec to code, unless told otherwise.
- In multi-agent setups, minimize idle messages. Only send updates on meaningful progress or when blocked.
