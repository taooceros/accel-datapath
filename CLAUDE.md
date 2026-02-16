# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This project provides C++ sender/receiver (stdexec) bindings for Intel Data Streaming Accelerator (DSA). It enables asynchronous hardware-accelerated memory operations using the P2300 (std::execution) programming model.

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

# Run any binary via the launcher directly
./build/linux/x86_64/release/dsa_launcher ./build/linux/x86_64/release/dsa-stdexec
./build/linux/x86_64/release/dsa_launcher ./build/linux/x86_64/release/example_data_move
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
include/dsa_stdexec/              stdexec sender/receiver integration
src/dsa/                          Low-level DSA hardware interface
```

### Low-Level DSA Interface (`src/dsa/`)

**`DsaHwContext`** (`dsa.hpp`): Manages work queue portal, descriptor submission (`_movdir64b`/`_enqcmd`), and completion checking. Satisfies the `HwContext` concept used by task queues.

**`DsaBase<QueueTemplate>`** (`dsa.hpp`): Generic DSA class templated on queue type. Handles device discovery via libaccel-config, work queue mapping, submit, and poll. Explicit instantiations live in `dsa_instantiate.cpp` to avoid redundant compilation.

Type aliases for common configurations:
- `Dsa` — `DsaBase<MutexTaskQueue>` (default, thread-safe)
- `DsaSingleThread` — `DsaBase<SingleThreadTaskQueue>` (no locks)
- `DsaSpinlock` — `DsaBase<SpinlockTaskQueue>` (TTAS)
- `DsaBackoffSpinlock` — `DsaBase<BackoffSpinlockTaskQueue>`
- `DsaLockFree` — `DsaBase<LockFreeTaskQueue>` (atomic CAS)

**Task Queues** (`task_queue.hpp`): Concept-based design (`TaskQueue` concept). Intrusive linked list of `OperationBase*`. Implementations: `LockedTaskQueue<Lock, HwCtx>` (parameterized by lock type), `RingBufferTaskQueue`, `LockFreeTaskQueue`.

**`DsaOperationBase`** (`dsa_operation_base.hpp`): Base class for all hardware operations. Over-allocates buffers for 64-byte aligned descriptors and 32-byte aligned completion records (needed for coroutine compatibility instead of `alignas`).

### stdexec Integration (`include/dsa_stdexec/`)

**Operation Senders** (`operations/*.hpp`): Each DSA operation has a sender following the same pattern:
- `<Op>Operation` inherits `DsaOperationBase`, implements `start()` (fill descriptor + submit) and `notify()` (handle completion/page fault retry)
- `<Op>Sender` is the stdexec sender with completion signatures `set_value_t(...)` and `set_error_t(exception_ptr)`
- Free function `dsa_<op>(dsa, ...)` creates the sender

Supported operations: `data_move`, `mem_fill`, `compare`, `compare_value`, `dualcast`, `crc_gen`, `copy_crc`, `cache_flush`.

**`OperationBase`** (`operation_base.hpp`): Type-erased operation using `pro::proxy<OperationFacade>`. Stores intrusive `next` pointer for queue linking. Proxy dispatches `notify()` and `get_descriptor()`.

**`PollingRunLoop`** (`run_loop.hpp`): Custom run loop that interleaves stdexec task execution with DSA polling. Takes a poll function (typically `[&dsa] { dsa.poll(); }`).

**`DsaScheduler`** (`scheduler.hpp`): stdexec scheduler whose `schedule()` returns a sender that completes immediately on DSA poll (sets completion status to 1, no hardware descriptor).

**Sync Helpers** (`sync_wait.hpp`):
- `sync_wait_threaded(sender)` — for background-thread polling mode (uses binary semaphore)
- `wait_start(sender, loop)` — for inline polling mode (runs the polling loop until completion)

### Key Design Decisions

- **Type erasure via proxy**: `pro::proxy<OperationFacade>` instead of virtual dispatch, enabling heterogeneous operation types in intrusive linked lists
- **Page fault handling**: `DSA_COMP_PAGE_FAULT_NOBOF` triggers automatic page touch + re-submit with adjusted byte offsets
- **Static dispatch for completion**: `HwContext::check_completion()` avoids virtual calls in the hot poll loop
- **Over-alignment**: Runtime alignment computation in `DsaOperationBase` instead of `alignas()` for coroutine frame compatibility

## Benchmark Configuration

Benchmarks are configured via `benchmark/benchmark_config.toml` with dimensions: polling mode (inline/threaded), scheduling pattern (sliding_window/batch/scoped_workers), queue type (6 variants), concurrency levels, and message sizes. Results output to CSV; visualize with `benchmark/visualize_benchmark.py`.

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
