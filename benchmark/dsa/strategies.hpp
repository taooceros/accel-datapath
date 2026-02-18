#pragma once
#ifndef BENCHMARK_STRATEGIES_HPP
#define BENCHMARK_STRATEGIES_HPP

#include "config.hpp"
#include "helpers.hpp"
#include <dsa_stdexec/dsa_facade.hpp>
#include <exec/async_scope.hpp>

using DsaProxy = dsa_stdexec::DsaProxy;

// Bundle of all arguments passed to strategy functions.
struct StrategyParams {
  DsaProxy &dsa;
  exec::async_scope &scope;
  size_t concurrency;
  size_t msg_size;
  size_t total_bytes;
  size_t batch_size;   // NEW: hardware submitter batch size (0 = default)
  BufferSet &bufs;
  LatencyCollector &latency;
  OperationType op_type;
};

// All strategy functions share this signature.
using StrategyFn = void(*)(const StrategyParams &);

// Sliding window
void run_sliding_window_inline(const StrategyParams &params);
void run_sliding_window_threaded(const StrategyParams &params);

// Sliding window noalloc
void run_sliding_window_inline_noalloc(const StrategyParams &params);
void run_sliding_window_threaded_noalloc(const StrategyParams &params);

// Sliding window arena
void run_sliding_window_inline_arena(const StrategyParams &params);
void run_sliding_window_threaded_arena(const StrategyParams &params);

// Batch
void run_batch_inline(const StrategyParams &params);
void run_batch_threaded(const StrategyParams &params);

// Batch noalloc
void run_batch_noalloc_inline(const StrategyParams &params);
void run_batch_noalloc_threaded(const StrategyParams &params);

// Scoped workers
void run_scoped_workers_inline(const StrategyParams &params);
void run_scoped_workers_threaded(const StrategyParams &params);

// Batch raw (hardware batch descriptor via dsa_batch sender, inline only)
void run_batch_raw_inline(const StrategyParams &params);

// Sliding window direct (no scope.nest, no then — inline only)
void run_sliding_window_inline_direct(const StrategyParams &params);

// Sliding window reusable (bypass stdexec connect/start — inline only)
void run_sliding_window_inline_reusable(const StrategyParams &params);

// Strategy taxonomy:
//
// ┌── SlidingWindow family       strategies/sliding_window/
// │   SlidingWindow              heap alloc per op (scope.spawn)     inline + threaded
// │   SlidingWindowNoAlloc       placement-new OperationSlot[]      inline + threaded
// │   SlidingWindowArena         free-list SlotArena (O(1) recycle) inline + threaded
// │   SlidingWindowDirect        bypasses async_scope/then          inline only
// │   SlidingWindowReusable      bypasses stdexec entirely          inline only
// │
// ├── Batch family               strategies/batch/
// │   Batch                      heap alloc per op (scope.spawn)    inline + threaded
// │   BatchNoAlloc               placement-new OperationSlot[]      inline + threaded
// │   BatchRaw                   hardware batch descriptor          inline only
// │
// └── ScopedWorkers family       strategies/scoped_workers/
//     ScopedWorkers              N coroutine workers, sequential    inline + threaded

// Indexed by [SchedulingPattern][PollingMode]: {inline, threaded}
// Order MUST match the SchedulingPattern enum in config.hpp.
inline constexpr StrategyFn strategy_table[][2] = {
  // SlidingWindow family (indices 0-2)
  /* SlidingWindow        */ { run_sliding_window_inline,           run_sliding_window_threaded },
  /* SlidingWindowNoAlloc */ { run_sliding_window_inline_noalloc,   run_sliding_window_threaded_noalloc },
  /* SlidingWindowArena   */ { run_sliding_window_inline_arena,     run_sliding_window_threaded_arena },
  // Batch family (indices 3-4)
  /* Batch                */ { run_batch_inline,                    run_batch_threaded },
  /* BatchNoAlloc         */ { run_batch_noalloc_inline,            run_batch_noalloc_threaded },
  // ScopedWorkers family (index 5)
  /* ScopedWorkers        */ { run_scoped_workers_inline,           run_scoped_workers_threaded },
  // Batch family continued (index 6)
  /* BatchRaw             */ { run_batch_raw_inline,                nullptr },
  // SlidingWindow family continued (indices 7-8)
  /* SlidingWindowDirect  */ { run_sliding_window_inline_direct,    nullptr },
  /* SlidingWindowReusable*/ { run_sliding_window_inline_reusable,  nullptr },
};

inline void dispatch_run(SchedulingPattern sp, PollingMode pm, const StrategyParams &params) {
  auto fn = strategy_table[static_cast<int>(sp)][static_cast<int>(pm)];
  if (!fn) return;
  fn(params);
}

#endif // BENCHMARK_STRATEGIES_HPP
