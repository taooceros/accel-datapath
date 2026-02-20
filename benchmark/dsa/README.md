# DSA Benchmark Framework

Multi-dimensional benchmark framework for measuring Intel DSA performance across
scheduling patterns, submission strategies, queue types, concurrency levels,
message sizes, and batch sizes.

## Key Files

| File | Description |
|------|-------------|
| `main.cpp` | Entry point, `run_benchmark`, `make_dsa`, CSV export |
| `config.hpp` / `config.cpp` | Enums (`SchedulingPattern`, `PollingMode`, `QueueType`, `OperationType`, etc.), `BenchmarkConfig`, TOML + CLI parsing |
| `helpers.hpp` | `ProgressBar`, `LatencyCollector`, `BufferSet`, `OperationSlot`, `BasicSlotArena`, `SlotArena`, `SlotReceiver`, `ArenaReceiver`, `DirectBenchReceiver` |
| `strategies.hpp` | `StrategyParams`, `StrategyFn`, `strategy_table`, `dispatch_run` |
| `strategy_common.hpp` | `with_op_sender`, `spawn_op`, `CompletionRecord`, slot-size helpers |
| `static.cpp` | Legacy monolithic benchmark (separate build target) |
| `strategies/` | Strategy implementations (see [strategies/README.md](strategies/README.md)) |

## Configuration

Benchmarks are configured via `benchmark/benchmark_config.toml`. The TOML file
specifies which dimensions to sweep. CLI flags can override TOML values, e.g.
`--batch-size=1,4,16` (comma-separated lists).

Run `run -- --help` to see all available CLI options.

### Dimensions Swept

- **Scheduling pattern**: sliding window, batch, scoped workers
- **Submission strategy**: heap alloc, noalloc, arena, direct, reusable, raw batch
- **Queue type**: mutex, spinlock, TTAS, backoff, lock-free, single-thread, indexed
- **Concurrency**: number of operations in flight (e.g. 1, 4, 16, 64)
- **Message size**: transfer size in bytes (e.g. 64, 256, 4096)
- **Batch size**: descriptor batch size for batch submitters

## Strategy Interface

All strategies share a unified function signature via `StrategyParams`:

```cpp
struct StrategyParams {
  DsaProxy &dsa;
  exec::async_scope &scope;
  size_t concurrency, msg_size, total_bytes, batch_size;
  BufferSet &bufs;
  LatencyCollector &latency;
  OperationType op_type;
};
using StrategyFn = void(*)(const StrategyParams &);
```

Strategies destructure at the top: `auto &[dsa, scope, concurrency, ...] = params;`

## Dispatch

Strategy dispatch uses a 2D table indexed by scheduling pattern and polling mode:

```
strategy_table[SchedulingPattern][PollingMode] -> StrategyFn
```

`dispatch_run` selects the correct function pointer and invokes it with the
assembled `StrategyParams`. The table order must match the `SchedulingPattern`
enum defined in `config.hpp`.

## Output

Results are exported to CSV with one row per benchmark configuration, including
the batch size column. Use `--output <filename>.csv` to avoid overwriting
previous results.

Visualize results with `benchmark/visualize_interactive.py`, which generates
interactive HTML plots (heatmaps, dashboards) from the CSV data.

## Further Reading

- [strategies/README.md](strategies/README.md) -- detailed strategy taxonomy, decision guide, and performance reference
- [CLAUDE.md](../../CLAUDE.md) -- project-wide conventions, build commands, and architecture overview
