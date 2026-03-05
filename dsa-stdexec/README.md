# dsa-stdexec

C++ stdexec sender/receiver framework for Intel DSA. Maximizes message rate (ops/sec) for small transfers using inline polling.

## Build

```bash
devenv shell                                    # Nix development shell
cd dsa-stdexec
xmake                                           # Build all targets
xmake build dsa_benchmark                       # Build specific target
xmake f -m release && xmake                     # Build modes: debug/release/profile
run                                             # Run benchmarks (auto dsa_launcher + build mode)
launch <cmd> [args...]                          # Run any command with CAP_SYS_RAWIO
```

C++23, GCC 15, mold linker. Flags `-menqcmd` and `-mmovdir64b` required for DSA intrinsics.

## Build Targets

| Target | Description |
|--------|-------------|
| `dsa-stdexec` | Main executable (all `src/**/*.cpp`) |
| `dsa_benchmark` | Multi-dimensional benchmark suite |
| `task_queue_benchmark` | Task queue synchronization benchmarks |
| `example_<op>` | One per op: `data_move`, `mem_fill`, `compare`, `compare_value`, `dualcast`, `crc_gen`, `copy_crc`, `cache_flush` |

## Structure

```
src/dsa/                          Low-level DSA hardware interface
include/dsa_stdexec/              stdexec sender/receiver integration
benchmark/dsa/                    Multi-dimensional benchmark suite
examples/                         Per-operation examples
test/                             Unit + integration tests
xmake.lua                         Build configuration
```

## Module READMEs

| README | Covers |
|--------|--------|
| `src/dsa/README.md` | DsaEngine, task queues, descriptor submitters, alignment |
| `include/dsa_stdexec/README.md` | stdexec integration, PollingRunLoop, senders |
| `include/dsa_stdexec/operations/README.md` | Per-operation sender pattern, how to add ops |
| `benchmark/dsa/README.md` | Benchmark framework, config, dispatch |
| `benchmark/dsa/strategies/README.md` | Strategy taxonomy, decision guide, perf reference |
| `examples/README.md` | Quick-start examples |
| `test/README.md` | Test suite coverage |

## Dependencies

Managed via Nix flake: stdexec, libaccel-config, fmt, proxy, tomlplusplus.

## Benchmark Rules

Always: (1) `--output <unique_filename>.csv` to avoid overwriting, (2) check `--help` before running, (3) preserve all CSV outputs. Visualize with `benchmark/visualize_interactive.py`.
