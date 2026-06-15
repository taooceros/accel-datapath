# Plan: Mock DSA Benchmark Integration & Software Overhead Investigation

## Goal

Integrate MockDsaBase into the main dsa_benchmark to measure pure software overhead
without DSA hardware latency. This isolates how fast the stdexec + task queue +
scheduling pattern machinery can run, establishing the software ceiling.

## Current State

- `MockDsaBase` exists in `src/dsa/mock_dsa.hpp` with instant completion (writes
  `DSA_COMP_SUCCESS` on submit, `MockHwContext::check_completion()` returns true)
- The benchmark uses `DsaProxy` (type-erased via `pro::proxy<DsaFacade>`) and
  `make_dsa()` factory to create concrete DSA types
- `MockDsaBase` satisfies `DsaSink` (has submit(op,desc), submit(op), poll())
  but is missing `flush()` which `DsaFacade` requires

## Implementation Steps

### Step 1: Add flush() to MockDsaBase

File: `src/dsa/mock_dsa.hpp`

Add `void flush() {}` to `MockDsaBase`. This is a no-op since mock has no submitter
staging to flush.

### Step 2: Add mock option to benchmark config

File: `benchmark/dsa/config.hpp`

Add to `BenchmarkConfig`:
```cpp
bool use_mock = false;  // Use MockDsaBase instead of real DSA
```

File: `benchmark/dsa/config.cpp`

Parse `[hardware]` section:
```toml
[hardware]
mock = false  # Use mock DSA (no real hardware, instant completion)
```

Also add `--mock` CLI flag.

### Step 3: Add make_mock_dsa() factory

File: `benchmark/dsa/main.cpp`

Add a factory function parallel to `make_dsa()`:
```cpp
#include <dsa/mock_dsa.hpp>

static DsaProxy make_mock_dsa(QueueType qt) {
    using dsa_stdexec::make_dsa_proxy;
    switch (qt) {
        case QueueType::NoLock:   return make_dsa_proxy<MockDsaSingleThread>();
        case QueueType::Mutex:    return make_dsa_proxy<MockDsa>();
        // ... etc
    }
}
```

### Step 4: Route benchmark through mock when enabled

File: `benchmark/dsa/main.cpp`

In `run_one_queue()`, check `config.use_mock`:
```cpp
static DsaMetric run_one_queue(..., bool use_mock = false) {
    DsaProxy dsa = use_mock
        ? make_mock_dsa(qt)
        : make_dsa(qt, ss, use_threaded_polling, batch_size);
    return run_benchmark(dsa, ...);
}
```

Pass `config.use_mock` through `run_all_queues()` and `benchmark_queues_with_dsa()`.

### Step 5: Run experiments and analyze

Configure benchmark_config.toml:
```toml
[hardware]
mock = true

[scheduling]
sliding_window_noalloc = true
batch_noalloc = true

[submission]
immediate = true  # Mock has no real submitter, immediate is simplest

[queues]
nolock = true
mutex = true

[parameters]
concurrency_levels = [32, 64, 128, 256, 512, 1024, 2048, 4096]
msg_sizes = [64]
total_bytes = 33554432
iterations = 10
```

Expected results:
- Mock with NoLock should show the pure software ceiling (~30-50+ Mpps)
- Mock with Mutex should show lock overhead
- Scaling with concurrency should reveal the O(N) poll traversal impact
- Comparing sliding_window_noalloc vs batch_noalloc shows scheduling overhead

### Step 6: Add per-phase instrumentation (optional)

For deeper analysis, add optional timing instrumentation to the hot loop:
- Time spent in slot scanning
- Time spent in start_op (connect + start)
- Time spent in poll (traversal + notify)

This can be enabled via a compile-time flag or config option.

## Verification Criteria

1. `xmake build dsa_benchmark` succeeds with mock code included
2. `--mock` flag produces valid results (non-zero msg_rate)
3. Mock msg_rate > real DSA msg_rate (confirms software overhead is measurable)
4. Mock results are stable across runs (no hardware-induced fluctuation)
5. O(N) scaling is visible: doubling concurrency should slow down msg_rate

## Team Assignment

| Agent | Task |
|-------|------|
| cpp-pro (Opus) | Step 1-4: Implement mock benchmark integration |
| Bash (Sonnet) | Step 5: Run experiments with mock and real DSA, compare |
| cpp-pro (Opus) | Analyze results, propose next optimizations |
