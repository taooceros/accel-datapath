# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Workflow Rules

- **Plans**: Before implementing non-trivial changes, write a plan to `plan/YYYY-MM-DD/<topic>.md` first. This captures intent, alternatives considered, and expected outcomes before code is touched.
- **Reports**: When discovering interesting findings during analysis (performance insights, architectural observations, surprising benchmark results, etc.), write them to `report/<descriptive_name>.md`. These build a persistent knowledge base for the project.
- **Early Hypotheses**: When debugging performance issues or analyzing code, provide a preliminary hypothesis or summary within the first 5 minutes of investigation. Do NOT spend an entire session reading files without delivering any analysis. If you need more time, state what you've found so far and what you're still investigating.

## Project Overview

C++ sender/receiver (stdexec) bindings for Intel Data Streaming Accelerator (DSA). The primary goal is **maximizing message rate** (ops/sec) for small transfers using inline polling — the calling thread drives both submission and completion in a tight loop with no cross-thread coordination.

This is a DSA (Data Streaming Accelerator) benchmark project. Key architecture: C++ benchmark harness with strategies (batch, sliding_window, reusable), Python visualization (visualize_interactive.py), xmake build system. Strategies live in `benchmark/dsa/strategies/{batch,sliding_window}/` subfolders. Config is TOML-based via `benchmark/benchmark_config.toml`. Always run `xmake build` to verify changes compile.

## Hardware Reference

`dsa_architecture_spec.md` contains the full Intel DSA specification (~637KB).

## Build Commands

```bash
# Enter development shell (nix/devenv)
devenv shell

# Build all targets
xmake

# Build specific target
xmake build dsa-stdexec
xmake build dsa_benchmark
xmake build example_data_move   # or any example_<op> target

# Build modes
xmake f -m debug && xmake
xmake f -m release && xmake
xmake f -m profile && xmake

# AddressSanitizer
xmake f --policies=build.sanitizer.address && xmake

# Run benchmarks (uses dsa_launcher for CAP_SYS_RAWIO, auto-detects build mode)
run
```

### Running Binaries

DSA requires `CAP_SYS_RAWIO`. Instead of applying `setcap` to every executable, use the `dsa_launcher` tool — it has `setcap cap_sys_rawio+eip` applied once to itself and passes the capability to child processes via ambient capabilities.

- **`run`** (devenv script): Builds `dsa_launcher` and `dsa_benchmark` if needed, auto-detects the current xmake build mode, and runs the benchmark. Accepts extra args: `run -- --help`.
- **`dsa_launcher <binary> [args...]`**: Launch any DSA binary with the required capability.

### Build Targets

| Target | Description |
|--------|-------------|
| `dsa-stdexec` | Main executable (all `src/**/*.cpp`) |
| `dsa_benchmark` | Multi-dimensional benchmark suite |
| `task_queue_benchmark` | Task queue synchronization benchmarks |
| `dsa_launcher` | C11 utility for DSA capability setting |
| `example_<op>` | One per operation: `data_move`, `mem_fill`, `compare`, `compare_value`, `dualcast`, `crc_gen`, `copy_crc`, `cache_flush` |

### Compiler Flags

C++23 with GCC 15, mold linker. Hardware instruction flags `-menqcmd` and `-mmovdir64b` are required for DSA submission intrinsics.

## Architecture

### Layer Overview

```
examples/ & src/main.cpp          Application layer
benchmark/dsa/                    Multi-dimensional benchmark suite
include/dsa_stdexec/              stdexec sender/receiver integration
src/dsa/                          Low-level DSA hardware interface
```

### Low-Level DSA Interface (`src/dsa/`)

**`DsaHwContext`** (`dsa.hpp`): Manages work queue portal, descriptor submission (`_movdir64b`/`_enqcmd`), and completion checking. Satisfies the `HwContext` concept used by task queues.

**`DsaEngine<Submitter, QueueTemplate>`** (`dsa.hpp`/`dsa.ipp`): Generic DSA class templated on submitter and queue type. Handles device discovery via libaccel-config, work queue mapping, submit, and poll. `submit()` includes WQ backpressure for dedicated work queues — spins on `poll()` when inflight descriptors reach WQ depth, preventing silent drops from `_movdir64b`. `poll()` feeds completion count back to the submitter via `notify_complete()`. Explicit instantiations live in `dsa_instantiate.cpp` to avoid redundant compilation.

Type aliases (preferred for inline polling — no lock contention on the hot path):
- `DsaSingleThread` — `DsaBase<SingleThreadTaskQueue>` (no locks, best for single-thread inline)
- `DsaIndexed` — `DsaBase<IndexedTaskQueue>` (per-slot, no head contention, inline only)

Other queue types (for threaded polling or comparative benchmarks):
- `Dsa` — `DsaBase<MutexTaskQueue>`, `DsaTasSpinlock`, `DsaSpinlock` (TTAS), `DsaBackoffSpinlock`, `DsaLockFree`

**`MirroredRing`** (`mirrored_ring.hpp`): RAII double-mapped ring buffer via `memfd_create` + two `MAP_FIXED` mappings of the same physical pages. Eliminates wrap-around handling in ring-based batch submitters. Region size is page-aligned.

**Descriptor Submitters** (`descriptor_submitter.hpp`): Concept-based (`DescriptorSubmitter`). `DirectSubmitter` does immediate `_movdir64b`/`_enqcmd` and tracks inflight count + WQ depth for dedicated-mode backpressure. Batch submitters (`BatchAdaptiveSubmitter`, `MirroredRingBatchSubmitter`, etc.) stage descriptors and submit as hardware batch descriptors; they self-throttle via ring capacity and delegate `wq_capacity()`/`inflight()` to their inner `DirectSubmitter`.

**Task Queues** (`task_queue.hpp`): Concept-based design (`TaskQueue` concept). Intrusive linked list of `OperationBase*`. Implementations: `LockedTaskQueue<Lock, HwCtx>` (parameterized by lock type), `RingBufferTaskQueue`, `LockFreeTaskQueue`.

**`DsaOperationBase`** (`dsa_operation_base.hpp`): Base class for all hardware operations. Over-allocates buffers for 64-byte aligned descriptors and 32-byte aligned completion records (needed for coroutine compatibility instead of `alignas`).

### stdexec Integration (`include/dsa_stdexec/`)

**Operation Senders** (`operations/*.hpp`): Each DSA operation has a sender following the same pattern:
- `<Op>Operation` inherits `DsaOperationBase`, implements `start()` (fill descriptor + submit) and `notify()` (handle completion/page fault retry)
- `<Op>Sender` is the stdexec sender with completion signatures `set_value_t(...)` and `set_error_t(exception_ptr)`
- Free function `dsa_<op>(dsa, ...)` creates the sender

Supported operations: `data_move`, `mem_fill`, `compare`, `compare_value`, `dualcast`, `crc_gen`, `copy_crc`, `cache_flush`.

**`OperationBase`** (`operation_base.hpp`): Type-erased operation using `pro::proxy<OperationFacade>`. Stores intrusive `next` pointer for queue linking. Proxy dispatches `notify()` and `get_descriptor()`.

**`PollingRunLoop`** (`run_loop.hpp`): Custom run loop that interleaves stdexec task execution with DSA polling. Takes a poll function (typically `[&dsa] { dsa.poll(); }`). This is the primary execution model for maximizing message rate.

**`wait_start(sender, loop)`** (`sync_wait.hpp`): Runs the polling loop until the sender completes. Used by inline strategies.

### Key Design Decisions

- **Type erasure via proxy**: `pro::proxy<OperationFacade>` instead of virtual dispatch, enabling heterogeneous operation types in intrusive linked lists
- **Page fault handling**: `DSA_COMP_PAGE_FAULT_NOBOF` triggers automatic page touch + re-submit with adjusted byte offsets
- **Static dispatch for completion**: `HwContext::check_completion()` avoids virtual calls in the hot poll loop
- **Over-alignment**: Runtime alignment computation in `DsaOperationBase` instead of `alignas()` for coroutine frame compatibility

## Benchmark Architecture (`benchmark/dsa/`)

### Directory Layout

```
benchmark/dsa/
├── main.cpp                  Entry point, run_benchmark, make_dsa, CSV export
├── config.hpp / config.cpp   Enums, BenchmarkConfig, TOML + CLI parsing
├── helpers.hpp               ProgressBar, LatencyCollector, BufferSet,
│                               OperationSlot, BasicSlotArena, SlotArena,
│                               SlotReceiver, ArenaReceiver, DirectBenchReceiver
├── strategies.hpp            StrategyParams, StrategyFn, strategy_table, dispatch_run
├── strategy_common.hpp       with_op_sender, spawn_op, CompletionRecord, slot-size helpers
├── static.cpp                Legacy monolithic benchmark (separate target)
└── strategies/
    ├── README.md             Strategy docs, decision guide, perf reference
    ├── sliding_window/       Sustained throughput (N ops always in flight)
    │   ├── sliding_window.cpp  Baseline: heap alloc per op (~35 ns/op)
    │   ├── noalloc.cpp         Placement-new into pre-allocated slots
    │   ├── arena.cpp           Free-list SlotArena (O(1) recycle)
    │   ├── direct.cpp          Bypasses async_scope (~13 ns/op)
    │   └── reusable.cpp        Bypasses stdexec entirely (~8 ns/op)
    ├── batch/                Barrier-synchronized groups
    │   ├── heap_alloc.cpp      Baseline batch
    │   ├── noalloc.cpp         Pre-allocated batch slots
    │   └── raw.cpp             Hardware batch descriptor (dsa_batch sender)
    └── scoped_workers/       N coroutine workers, sequential processing
        └── scoped_workers.cpp
```

### StrategyParams

All strategy functions share a unified signature via `StrategyParams` struct (`strategies.hpp`):

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

### Strategy Taxonomy (inline polling focus)

```
├── SlidingWindow family       Sustained throughput, N ops always in flight
│   ├── sliding_window           stdexec baseline (~35 ns/op)
│   ├── noalloc / arena          zero-alloc variants
│   ├── direct                   bypass async_scope (~13 ns/op)
│   └── reusable                 bypass stdexec entirely (~8 ns/op)
├── Batch family               Barrier-synchronized groups
│   ├── heap_alloc / noalloc     stdexec-based batching
│   └── raw                      hardware batch descriptor
└── ScopedWorkers              N coroutine workers, sequential co_await
```

Threaded polling variants exist for comparison but are not the optimization target. The `strategy_table[SchedulingPattern][PollingMode]` dispatches via function pointer; order must match `SchedulingPattern` enum in `config.hpp`.

### Benchmark Configuration

Configured via `benchmark/benchmark_config.toml` with dimensions: scheduling pattern, submission strategy, queue type, concurrency levels, message sizes, and batch sizes (descriptor batching sweep). Results output to CSV with per-result batch size; visualize with `benchmark/visualize_interactive.py`. CLI: `--batch-size=N,...` (comma-separated list).

## Dependencies

- **stdexec**: NVIDIA's P2300 reference implementation (header-only)
- **libaccel-config**: Intel accelerator configuration library for DSA/IAX
- **fmt**: Formatting library
- **proxy**: Microsoft's proxy library for polymorphic programming
- **tomlplusplus**: TOML parsing for benchmark config

All dependencies are managed via Nix flake (`flake.nix` / `devenv.nix`).

## Hardware Requirements

- Intel CPU with DSA (4th Gen Xeon Scalable or later)
- DSA device configured and work queue enabled (via `accel-config`)
- `CAP_SYS_RAWIO` capability for user-space DSA access

## Benchmarking

When running benchmarks, always: (1) use `--output <unique_filename>.csv` to avoid overwriting previous results, (2) verify correct CLI flags before running (check `--help` first), (3) preserve all CSV outputs.

## Code Changes

When asked to fix or strengthen code: always match the CODE to the SPEC, not the spec to the code, unless explicitly told otherwise.

## Multi-Agent Conventions

When working in multi-agent team setups, minimize idle notification messages. Only send status updates when you have meaningful progress or are blocked on a dependency. Do not send repeated 'still waiting' messages.
