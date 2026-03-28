# benchmark/dsa AGENTS

Inherits `../../AGENTS.md`.

## OVERVIEW
Benchmark framework for DSA scheduling patterns, polling modes, queue types, message sizes, and batch sizes.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| CLI entry and CSV export | `main.cpp` | Benchmark runner. |
| Config enums and parsing | `config.hpp`, `config.cpp` | TOML + CLI override wiring. |
| Dispatch | `strategies.hpp` | `strategy_table` and `dispatch_run`. |
| Common helpers | `helpers.hpp`, `strategy_common.hpp` | Slot and receiver machinery. |
| Strategy taxonomy | `strategies/README.md` | Read before changing strategies. |
| Sweep defaults | `../benchmark_config.toml` | Source of default dimensions. |

## CONVENTIONS
- Keep `strategy_table` order aligned with the `SchedulingPattern` enum.
- Treat `benchmark/benchmark_config.toml` plus CLI overrides as the user-facing configuration surface.
- Preserve the batch-size column and CSV output discipline.
- Use the taxonomy in `strategies/README.md` when naming or adding strategies.

## ANTI-PATTERNS
- Do not add a strategy without wiring config, dispatch, and documentation together.
- Do not overwrite previous CSV outputs; require unique filenames.
- Do not change inline vs threaded semantics casually; they are a core comparison axis.

## COMMANDS
```bash
cd dsa-stdexec && run -- --help
cd dsa-stdexec && xmake build dsa_benchmark
```
