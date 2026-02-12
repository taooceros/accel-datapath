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

// Scoped workers
void run_scoped_workers_inline(DsaProxy &dsa, exec::async_scope &scope,
                               size_t concurrency, size_t msg_size, size_t total_bytes,
                               BufferSet &bufs, LatencyCollector &latency,
                               OperationType op_type);
void run_scoped_workers_threaded(DsaProxy &dsa, exec::async_scope &scope,
                                 size_t concurrency, size_t msg_size, size_t total_bytes,
                                 BufferSet &bufs, LatencyCollector &latency,
                                 OperationType op_type);

// Indexed by [SchedulingPattern][PollingMode]: {inline, threaded}
inline constexpr StrategyFn strategy_table[][2] = {
  /* SlidingWindow       */ { run_sliding_window_inline,          run_sliding_window_threaded },
  /* SlidingWindowNoAlloc*/ { run_sliding_window_inline_noalloc,  run_sliding_window_threaded_noalloc },
  /* SlidingWindowArena  */ { run_sliding_window_inline_arena,    run_sliding_window_threaded_arena },
  /* Batch              */  { run_batch_inline,                   run_batch_threaded },
  /* ScopedWorkers      */  { run_scoped_workers_inline,          run_scoped_workers_threaded },
};

inline void dispatch_run(SchedulingPattern sp, PollingMode pm, OperationType op_type,
                         DsaProxy &dsa, exec::async_scope &scope,
                         size_t concurrency, size_t msg_size, size_t total_bytes,
                         BufferSet &bufs, LatencyCollector &latency) {
  strategy_table[static_cast<int>(sp)][static_cast<int>(pm)](
      dsa, scope, concurrency, msg_size, total_bytes, bufs, latency, op_type);
}

#endif // BENCHMARK_STRATEGIES_HPP
