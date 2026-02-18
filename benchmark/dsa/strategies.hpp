#pragma once
#ifndef BENCHMARK_STRATEGIES_HPP
#define BENCHMARK_STRATEGIES_HPP

#include "config.hpp"
#include "helpers.hpp"
#include <dsa_stdexec/dsa_facade.hpp>
#include <exec/async_scope.hpp>

using DsaProxy = dsa_stdexec::DsaProxy;

// All strategy functions share this signature.
using StrategyFn = void(*)(DsaProxy &, exec::async_scope &, size_t, size_t, size_t,
                           BufferSet &, LatencyCollector &, OperationType);

// Sliding window
void run_sliding_window_inline(DsaProxy &dsa, exec::async_scope &scope,
                               size_t concurrency, size_t msg_size, size_t total_bytes,
                               BufferSet &bufs, LatencyCollector &latency,
                               OperationType op_type);
void run_sliding_window_threaded(DsaProxy &dsa, exec::async_scope &scope,
                                 size_t concurrency, size_t msg_size, size_t total_bytes,
                                 BufferSet &bufs, LatencyCollector &latency,
                                 OperationType op_type);

// Sliding window noalloc
void run_sliding_window_inline_noalloc(DsaProxy &dsa, exec::async_scope &scope,
                                       size_t concurrency, size_t msg_size, size_t total_bytes,
                                       BufferSet &bufs, LatencyCollector &latency,
                                       OperationType op_type);
void run_sliding_window_threaded_noalloc(DsaProxy &dsa, exec::async_scope &scope,
                                         size_t concurrency, size_t msg_size, size_t total_bytes,
                                         BufferSet &bufs, LatencyCollector &latency,
                                         OperationType op_type);

// Sliding window arena
void run_sliding_window_inline_arena(DsaProxy &dsa, exec::async_scope &scope,
                                     size_t concurrency, size_t msg_size, size_t total_bytes,
                                     BufferSet &bufs, LatencyCollector &latency,
                                     OperationType op_type);
void run_sliding_window_threaded_arena(DsaProxy &dsa, exec::async_scope &scope,
                                       size_t concurrency, size_t msg_size, size_t total_bytes,
                                       BufferSet &bufs, LatencyCollector &latency,
                                       OperationType op_type);

// Batch
void run_batch_inline(DsaProxy &dsa, exec::async_scope &scope,
                      size_t concurrency, size_t msg_size, size_t total_bytes,
                      BufferSet &bufs, LatencyCollector &latency,
                      OperationType op_type);
void run_batch_threaded(DsaProxy &dsa, exec::async_scope &scope,
                        size_t concurrency, size_t msg_size, size_t total_bytes,
                        BufferSet &bufs, LatencyCollector &latency,
                        OperationType op_type);

// Batch noalloc
void run_batch_noalloc_inline(DsaProxy &dsa, exec::async_scope &scope,
                              size_t concurrency, size_t msg_size, size_t total_bytes,
                              BufferSet &bufs, LatencyCollector &latency,
                              OperationType op_type);
void run_batch_noalloc_threaded(DsaProxy &dsa, exec::async_scope &scope,
                                size_t concurrency, size_t msg_size, size_t total_bytes,
                                BufferSet &bufs, LatencyCollector &latency,
                                OperationType op_type);

// Scoped workers
void run_scoped_workers_inline(DsaProxy &dsa, exec::async_scope &scope,
                               size_t concurrency, size_t msg_size, size_t total_bytes,
                               BufferSet &bufs, LatencyCollector &latency,
                               OperationType op_type);
void run_scoped_workers_threaded(DsaProxy &dsa, exec::async_scope &scope,
                                 size_t concurrency, size_t msg_size, size_t total_bytes,
                                 BufferSet &bufs, LatencyCollector &latency,
                                 OperationType op_type);

// Batch raw (hardware batch descriptor via dsa_batch sender, inline only)
void run_batch_raw_inline(DsaProxy &dsa, exec::async_scope &scope,
                          size_t concurrency, size_t msg_size, size_t total_bytes,
                          BufferSet &bufs, LatencyCollector &latency,
                          OperationType op_type);

// Sliding window direct (no scope.nest, no then — inline only)
void run_sliding_window_inline_direct(DsaProxy &dsa, exec::async_scope &scope,
                                      size_t concurrency, size_t msg_size, size_t total_bytes,
                                      BufferSet &bufs, LatencyCollector &latency,
                                      OperationType op_type);

// Sliding window reusable (bypass stdexec connect/start — inline only)
void run_sliding_window_inline_reusable(DsaProxy &dsa, exec::async_scope &scope,
                                        size_t concurrency, size_t msg_size, size_t total_bytes,
                                        BufferSet &bufs, LatencyCollector &latency,
                                        OperationType op_type);

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

inline void dispatch_run(SchedulingPattern sp, PollingMode pm, OperationType op_type,
                         DsaProxy &dsa, exec::async_scope &scope,
                         size_t concurrency, size_t msg_size, size_t total_bytes,
                         BufferSet &bufs, LatencyCollector &latency) {
  auto fn = strategy_table[static_cast<int>(sp)][static_cast<int>(pm)];
  if (!fn) return; // unsupported combination (e.g. batch_raw + threaded)
  fn(dsa, scope, concurrency, msg_size, total_bytes, bufs, latency, op_type);
}

#endif // BENCHMARK_STRATEGIES_HPP
