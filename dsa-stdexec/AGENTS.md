# dsa-stdexec AGENTS

Inherits `../AGENTS.md`. Use the nearest child file for `src/dsa/`, `benchmark/dsa/`, or `include/dsa_stdexec/operations/`.

## OVERVIEW
C++23/xmake DSA framework focused on maximizing small-message throughput with inline polling.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Low-level engine, queues, submitters | `src/dsa/AGENTS.md` | Hardware-facing invariants live there. |
| stdexec integration | `include/dsa_stdexec/README.md` | `PollingRunLoop`, scheduler, type erasure. |
| Adding a new operation | `include/dsa_stdexec/operations/AGENTS.md` | Use the local checklist. |
| Benchmarks and strategies | `benchmark/dsa/AGENTS.md` | Config, dispatch, CSV discipline. |
| Examples | `examples/README.md` | `example_<op>` naming and run flow. |
| Tests | `test/README.md` | Coverage and hardware-capability notes. |
| Build truth | `xmake.lua` | Targets and hooks are authoritative. |

## CONVENTIONS
- Build from `dsa-stdexec/` with `xmake`; use `run` / `launch` for hardware execution.
- Treat inline polling as the primary path. Threaded polling exists for comparison, not as the default design target.
- Read the nearest module README before changing `src/dsa/`, `include/dsa_stdexec/`, `benchmark/dsa/`, `examples/`, or `test/`.
- Preserve benchmark outputs; use unique CSV filenames.

## ANTI-PATTERNS
- Do not change strategy behavior without checking `benchmark/dsa/strategies/README.md`.
- Do not add examples, tests, or ops without updating the matching build registration in `xmake.lua`.
- Do not describe `dsa_launcher` behavior from memory; use `../tools/README.md`.

## COMMANDS
```bash
cd dsa-stdexec && xmake
cd dsa-stdexec && xmake build dsa_benchmark
cd dsa-stdexec && xmake f -m release && xmake
cd dsa-stdexec && run -- --help
cd dsa-stdexec && xmake build example_data_move
```
