// Dynamic dispatch benchmark using pro::proxy for type-erased DSA access.
// All templates instantiate once (for DsaProxy) instead of 6 times per queue type.

// Include fmt headers first to avoid partial specialization conflicts
#include <fmt/format.h>
#include <fmt/ranges.h>
#include "benchmark_config.hpp"
#include "benchmark_helpers.hpp"
#include <algorithm>
#include <chrono>
#include <cstring>
#include <dsa/dsa.hpp>
#include <dsa/dsa_batch.hpp>
#include <dsa/task_queue.hpp>
#include <dsa_stdexec/dsa_facade.hpp>
#include <dsa_stdexec/operations/cache_flush.hpp>
#include <dsa_stdexec/operations/compare.hpp>
#include <dsa_stdexec/operations/compare_value.hpp>
#include <dsa_stdexec/operations/copy_crc.hpp>
#include <dsa_stdexec/operations/crc_gen.hpp>
#include <dsa_stdexec/operations/data_move.hpp>
#include <dsa_stdexec/operations/dualcast.hpp>
#include <dsa_stdexec/operations/mem_fill.hpp>
#include <dsa_stdexec/run_loop.hpp>
#include <dsa_stdexec/scheduler.hpp>
#include <dsa_stdexec/sync_wait.hpp>
#include <exec/async_scope.hpp>
#include <exec/task.hpp>
#include <fstream>
#include <functional>
#include <stdexec/execution.hpp>
#include <thread>
#include <utility>
#include <vector>

using DsaProxy = dsa_stdexec::DsaProxy;

// Type-erased run function signature
using RunFunction = std::function<void(DsaProxy &, exec::async_scope &, size_t,
                                       size_t, size_t, BufferSet &,
                                       LatencyCollector &)>;

// ============================================================================
// OPERATION DISPATCH — single switch for all op_type → sender mappings
// ============================================================================

// Calls f(op_sender) where op_sender is a lambda: (size_t offset) -> Sender.
// Each operation type produces a different sender type, so f must be generic.
template <class F>
void with_op_sender(OperationType op_type, DsaProxy &dsa,
                    BufferSet &bufs, size_t msg_size, F &&f) {
  using namespace dsa_stdexec;
  switch (op_type) {
    case OperationType::DataMove:
      f([&](size_t off) { return dsa_data_move(dsa, bufs.src.data() + off, bufs.dst.data() + off, msg_size); });
      break;
    case OperationType::MemFill:
      f([&](size_t off) { return dsa_mem_fill(dsa, bufs.dst.data() + off, msg_size, BufferSet::fill_pattern); });
      break;
    case OperationType::Compare:
      f([&](size_t off) { return dsa_compare(dsa, bufs.src.data() + off, bufs.dst.data() + off, msg_size); });
      break;
    case OperationType::CompareValue:
      f([&](size_t off) { return dsa_compare_value(dsa, bufs.src.data() + off, msg_size, BufferSet::fill_pattern); });
      break;
    case OperationType::Dualcast:
      f([&](size_t off) { return dsa_dualcast(dsa, bufs.src.data() + off, bufs.dualcast_dst1 + off, bufs.dualcast_dst2 + off, msg_size); });
      break;
    case OperationType::CrcGen:
      f([&](size_t off) { return dsa_crc_gen(dsa, bufs.src.data() + off, msg_size); });
      break;
    case OperationType::CopyCrc:
      f([&](size_t off) { return dsa_copy_crc(dsa, bufs.src.data() + off, bufs.dst.data() + off, msg_size); });
      break;
    case OperationType::CacheFlush:
      f([&](size_t off) { return dsa_cache_flush(dsa, bufs.dst.data() + off, msg_size); });
      break;
  }
}

// ============================================================================
// SPAWN HELPERS
// ============================================================================

void spawn_op(DsaProxy &dsa, exec::async_scope &scope, OperationType op_type,
              BufferSet &bufs, size_t offset, size_t msg_size,
              LatencyCollector &latency, std::atomic<size_t> *in_flight = nullptr) {
  auto start_time = std::chrono::high_resolution_clock::now();
  auto record = [&latency, start_time, in_flight](auto&&...) {
    auto end = std::chrono::high_resolution_clock::now();
    latency.record(std::chrono::duration<double, std::nano>(end - start_time).count());
    if (in_flight) in_flight->fetch_sub(1, std::memory_order_release);
  };
  if (in_flight) in_flight->fetch_add(1, std::memory_order_relaxed);

  with_op_sender(op_type, dsa, bufs, msg_size, [&](auto op_sender) {
    scope.spawn(op_sender(offset) | stdexec::then(record));
  });
}

void spawn_op_scheduled(DsaProxy &dsa, dsa_stdexec::DsaScheduler<DsaProxy> &scheduler,
                        exec::async_scope &scope, OperationType op_type,
                        BufferSet &bufs, size_t offset, size_t msg_size,
                        LatencyCollector &latency,
                        std::atomic<size_t> *in_flight = nullptr) {
  auto start_time = std::chrono::high_resolution_clock::now();
  if (in_flight) in_flight->fetch_add(1, std::memory_order_relaxed);

  auto record = [&latency, start_time, in_flight](auto&&...) {
    auto end = std::chrono::high_resolution_clock::now();
    latency.record(std::chrono::duration<double, std::nano>(end - start_time).count());
    if (in_flight) in_flight->fetch_sub(1, std::memory_order_release);
  };

  with_op_sender(op_type, dsa, bufs, msg_size, [&](auto op_sender) {
    scope.spawn(scheduler.schedule() | stdexec::let_value([op_sender, offset, record]() {
      return op_sender(offset) | stdexec::then(record);
    }));
  });
}

// ============================================================================
// SLIDING WINDOW STRATEGY
// ============================================================================

void run_sliding_window_inline(DsaProxy &dsa, exec::async_scope &scope,
                               size_t concurrency, size_t msg_size, size_t total_bytes,
                               BufferSet &bufs, LatencyCollector &latency,
                               OperationType op_type) {
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.flush(); dsa.poll(); });

  size_t num_ops = total_bytes / msg_size;
  std::atomic<size_t> in_flight{0};
  size_t next_op = 0;

  while (next_op < num_ops) {
    while (next_op < num_ops && in_flight.load(std::memory_order_acquire) < concurrency) {
      size_t offset = next_op * msg_size;
      spawn_op(dsa, scope, op_type, bufs, offset, msg_size, latency, &in_flight);
      ++next_op;
    }
    dsa.flush();
    dsa.poll();
  }

  dsa_stdexec::wait_start(scope.on_empty(), loop);
}

void run_sliding_window_threaded(DsaProxy &dsa, exec::async_scope &scope,
                                 size_t concurrency, size_t msg_size, size_t total_bytes,
                                 BufferSet &bufs, LatencyCollector &latency,
                                 OperationType op_type) {
  dsa_stdexec::DsaScheduler<DsaProxy> scheduler(dsa);

  size_t num_ops = total_bytes / msg_size;
  std::atomic<size_t> in_flight{0};

  for (size_t op_idx = 0; op_idx < num_ops; ++op_idx) {
    while (in_flight.load(std::memory_order_acquire) >= concurrency) {
      std::this_thread::yield();
    }

    size_t offset = op_idx * msg_size;
    spawn_op_scheduled(dsa, scheduler, scope, op_type, bufs, offset, msg_size, latency, &in_flight);
  }
  stdexec::sync_wait(scope.on_empty());
}

// ============================================================================
// SLIDING WINDOW NOALLOC STRATEGY
// Uses scope.nest() + pre-allocated OperationSlots instead of scope.spawn()
// to eliminate per-operation heap allocation.
// ============================================================================

// Compute the record callback type used by the noalloc sliding window.
// All record lambdas have the same type (captures: LatencyCollector&, time_point, atomic&).
struct NoAllocRecord {
  LatencyCollector *latency;
  std::chrono::high_resolution_clock::time_point start_time;
  std::atomic<size_t> *in_flight;
  void operator()(auto &&...) const {
    auto end = std::chrono::high_resolution_clock::now();
    latency->record(std::chrono::duration<double, std::nano>(end - start_time).count());
    in_flight->fetch_sub(1, std::memory_order_release);
  }
};

// Compute the operation state size for inline noalloc: nest(make_sender(0) | then(record))
template <class MakeSender>
constexpr size_t inline_noalloc_slot_size() {
  using Sender = decltype(std::declval<MakeSender>()(size_t{0}));
  using ThenSender = decltype(std::declval<Sender>() | stdexec::then(std::declval<NoAllocRecord>()));
  using NestSender = exec::async_scope::nest_result_t<ThenSender>;
  return sizeof(stdexec::connect_result_t<NestSender, SlotReceiver>);
}

template <class MakeSender>
void sliding_window_noalloc_impl_inline(
    DsaProxy &dsa, exec::async_scope &scope,
    size_t concurrency, size_t msg_size, size_t total_bytes,
    BufferSet &bufs, LatencyCollector &latency,
    MakeSender make_sender) {
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.flush(); dsa.poll(); });

  constexpr size_t SlotSize = inline_noalloc_slot_size<MakeSender>();
  size_t num_ops = total_bytes / msg_size;
  std::atomic<size_t> in_flight{0};
  std::vector<std::unique_ptr<OperationSlot<SlotSize>>> slots;
  slots.reserve(concurrency);
  for (size_t i = 0; i < concurrency; ++i)
    slots.push_back(std::make_unique<OperationSlot<SlotSize>>());

  size_t next_op = 0;
  while (next_op < num_ops) {
    for (auto &slot : slots) {
      if (next_op >= num_ops) break;
      if (!slot->ready.load(std::memory_order_acquire)) continue;
      size_t offset = next_op * msg_size;
      in_flight.fetch_add(1, std::memory_order_relaxed);
      NoAllocRecord record{&latency, std::chrono::high_resolution_clock::now(), &in_flight};
      slot->start_op(scope.nest(make_sender(offset) | stdexec::then(record)));
      ++next_op;
    }
    dsa.flush();
    dsa.poll();
  }

  dsa_stdexec::wait_start(scope.on_empty(), loop);
}

void run_sliding_window_inline_noalloc(DsaProxy &dsa, exec::async_scope &scope,
                                       size_t concurrency, size_t msg_size, size_t total_bytes,
                                       BufferSet &bufs, LatencyCollector &latency,
                                       OperationType op_type) {
  with_op_sender(op_type, dsa, bufs, msg_size, [&](auto op_sender) {
    sliding_window_noalloc_impl_inline(dsa, scope, concurrency, msg_size, total_bytes, bufs, latency, op_sender);
  });
}

// Compute the operation state size for threaded noalloc:
// nest(schedule() | let_value([make_sender, offset, record]() { return make_sender(offset) | then(record); }))
template <class MakeSender>
constexpr size_t threaded_noalloc_slot_size() {
  using Scheduler = dsa_stdexec::DsaScheduler<DsaProxy>;
  using SchedSender = decltype(std::declval<Scheduler>().schedule());
  // The let_value lambda captures: MakeSender, size_t offset, NoAllocRecord
  struct LetLambda {
    MakeSender make_sender;
    size_t offset;
    NoAllocRecord record;
    auto operator()() {
      return make_sender(offset) | stdexec::then(record);
    }
  };
  using LetSender = decltype(std::declval<SchedSender>() | stdexec::let_value(std::declval<LetLambda>()));
  using NestSender = exec::async_scope::nest_result_t<LetSender>;
  return sizeof(stdexec::connect_result_t<NestSender, SlotReceiver>);
}

template <class MakeSender>
void sliding_window_noalloc_impl_threaded(
    DsaProxy &dsa, exec::async_scope &scope,
    size_t concurrency, size_t msg_size, size_t total_bytes,
    BufferSet &bufs, LatencyCollector &latency,
    MakeSender make_sender) {
  dsa_stdexec::DsaScheduler<DsaProxy> scheduler(dsa);

  constexpr size_t SlotSize = threaded_noalloc_slot_size<MakeSender>();
  size_t num_ops = total_bytes / msg_size;
  std::atomic<size_t> in_flight{0};
  std::vector<std::unique_ptr<OperationSlot<SlotSize>>> slots;
  slots.reserve(concurrency);
  for (size_t i = 0; i < concurrency; ++i)
    slots.push_back(std::make_unique<OperationSlot<SlotSize>>());

  for (size_t op_idx = 0; op_idx < num_ops; ++op_idx) {
    // Wait for a slot to become available
    while (true) {
      for (auto &slot : slots) {
        if (slot->ready.load(std::memory_order_acquire)) {
          size_t offset = op_idx * msg_size;
          in_flight.fetch_add(1, std::memory_order_relaxed);
          NoAllocRecord record{&latency, std::chrono::high_resolution_clock::now(), &in_flight};
          slot->start_op(scope.nest(
            scheduler.schedule() | stdexec::let_value([make_sender, offset, record]() {
              return make_sender(offset) | stdexec::then(record);
            })
          ));
          goto next_op;
        }
      }
      std::this_thread::yield();
    }
    next_op:;
  }
  stdexec::sync_wait(scope.on_empty());
}

void run_sliding_window_threaded_noalloc(DsaProxy &dsa, exec::async_scope &scope,
                                         size_t concurrency, size_t msg_size, size_t total_bytes,
                                         BufferSet &bufs, LatencyCollector &latency,
                                         OperationType op_type) {
  with_op_sender(op_type, dsa, bufs, msg_size, [&](auto op_sender) {
    sliding_window_noalloc_impl_threaded(dsa, scope, concurrency, msg_size, total_bytes, bufs, latency, op_sender);
  });
}

// ============================================================================
// SLIDING WINDOW ARENA STRATEGY
// Uses scope.nest() + SlotArena free-list instead of scanning all slots.
// Inspired by ibverbs/UCX WR free-list: O(1) acquire/release via intrusive
// linked list, zero allocation after init.
// ============================================================================

template <class MakeSender>
constexpr size_t inline_arena_slot_size() {
  using Sender = decltype(std::declval<MakeSender>()(size_t{0}));
  using ThenSender = decltype(std::declval<Sender>() | stdexec::then(std::declval<NoAllocRecord>()));
  using NestSender = exec::async_scope::nest_result_t<ThenSender>;
  constexpr size_t SlotSize = inline_noalloc_slot_size<MakeSender>();
  using Receiver = ArenaReceiver<SlotSize>;
  return sizeof(stdexec::connect_result_t<NestSender, Receiver>);
}

template <class MakeSender>
void sliding_window_arena_impl_inline(
    DsaProxy &dsa, exec::async_scope &scope,
    size_t concurrency, size_t msg_size, size_t total_bytes,
    BufferSet &bufs, LatencyCollector &latency,
    MakeSender make_sender) {
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.flush(); dsa.poll(); });

  // Use the noalloc slot size — the arena receiver has the same size as slot receiver
  // for the operation state; recompute to be safe.
  constexpr size_t SlotSize = inline_arena_slot_size<MakeSender>();
  size_t num_ops = total_bytes / msg_size;
  std::atomic<size_t> in_flight{0};
  SlotArena<SlotSize> arena(concurrency);

  size_t next_op = 0;
  while (next_op < num_ops) {
    while (next_op < num_ops) {
      auto *slot = arena.acquire();
      if (!slot) break;
      size_t offset = next_op * msg_size;
      in_flight.fetch_add(1, std::memory_order_relaxed);
      NoAllocRecord record{&latency, std::chrono::high_resolution_clock::now(), &in_flight};
      slot->start_op_with(
        scope.nest(make_sender(offset) | stdexec::then(record)),
        ArenaReceiver<SlotSize>{&arena, slot});
      ++next_op;
    }
    dsa.flush();
    dsa.poll();
  }

  dsa_stdexec::wait_start(scope.on_empty(), loop);
}

void run_sliding_window_inline_arena(DsaProxy &dsa, exec::async_scope &scope,
                                     size_t concurrency, size_t msg_size, size_t total_bytes,
                                     BufferSet &bufs, LatencyCollector &latency,
                                     OperationType op_type) {
  with_op_sender(op_type, dsa, bufs, msg_size, [&](auto op_sender) {
    sliding_window_arena_impl_inline(dsa, scope, concurrency, msg_size, total_bytes, bufs, latency, op_sender);
  });
}

template <class MakeSender>
constexpr size_t threaded_arena_slot_size() {
  using Scheduler = dsa_stdexec::DsaScheduler<DsaProxy>;
  using SchedSender = decltype(std::declval<Scheduler>().schedule());
  struct LetLambda {
    MakeSender make_sender;
    size_t offset;
    NoAllocRecord record;
    auto operator()() {
      return make_sender(offset) | stdexec::then(record);
    }
  };
  using LetSender = decltype(std::declval<SchedSender>() | stdexec::let_value(std::declval<LetLambda>()));
  using NestSender = exec::async_scope::nest_result_t<LetSender>;
  constexpr size_t SlotSize = threaded_noalloc_slot_size<MakeSender>();
  using Receiver = ArenaReceiver<SlotSize>;
  return sizeof(stdexec::connect_result_t<NestSender, Receiver>);
}

template <class MakeSender>
void sliding_window_arena_impl_threaded(
    DsaProxy &dsa, exec::async_scope &scope,
    size_t concurrency, size_t msg_size, size_t total_bytes,
    BufferSet &bufs, LatencyCollector &latency,
    MakeSender make_sender) {
  dsa_stdexec::DsaScheduler<DsaProxy> scheduler(dsa);

  constexpr size_t SlotSize = threaded_arena_slot_size<MakeSender>();
  size_t num_ops = total_bytes / msg_size;
  std::atomic<size_t> in_flight{0};
  SlotArena<SlotSize> arena(concurrency);

  for (size_t op_idx = 0; op_idx < num_ops; ++op_idx) {
    while (arena.empty()) {
      std::this_thread::yield();
    }
    auto *slot = arena.acquire();
    size_t offset = op_idx * msg_size;
    in_flight.fetch_add(1, std::memory_order_relaxed);
    NoAllocRecord record{&latency, std::chrono::high_resolution_clock::now(), &in_flight};
    slot->start_op_with(
      scope.nest(
        scheduler.schedule() | stdexec::let_value([make_sender, offset, record]() {
          return make_sender(offset) | stdexec::then(record);
        })
      ),
      ArenaReceiver<SlotSize>{&arena, slot});
  }
  stdexec::sync_wait(scope.on_empty());
}

void run_sliding_window_threaded_arena(DsaProxy &dsa, exec::async_scope &scope,
                                       size_t concurrency, size_t msg_size, size_t total_bytes,
                                       BufferSet &bufs, LatencyCollector &latency,
                                       OperationType op_type) {
  with_op_sender(op_type, dsa, bufs, msg_size, [&](auto op_sender) {
    sliding_window_arena_impl_threaded(dsa, scope, concurrency, msg_size, total_bytes, bufs, latency, op_sender);
  });
}

// ============================================================================
// BATCH STRATEGY
// ============================================================================

void run_batch_inline(DsaProxy &dsa, exec::async_scope &scope,
                      size_t concurrency, size_t msg_size, size_t total_bytes,
                      BufferSet &bufs, LatencyCollector &latency,
                      OperationType op_type) {
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.flush(); dsa.poll(); });

  size_t num_ops = total_bytes / msg_size;
  size_t op_idx = 0;

  while (op_idx < num_ops) {
    size_t batch_end = std::min(op_idx + concurrency, num_ops);
    for (size_t i = op_idx; i < batch_end; ++i) {
      size_t offset = i * msg_size;
      spawn_op(dsa, scope, op_type, bufs, offset, msg_size, latency);
    }
    dsa.flush();
    dsa_stdexec::wait_start(scope.on_empty(), loop);
    loop.reset();
    op_idx = batch_end;
  }
}

void run_batch_threaded(DsaProxy &dsa, exec::async_scope &scope,
                        size_t concurrency, size_t msg_size, size_t total_bytes,
                        BufferSet &bufs, LatencyCollector &latency,
                        OperationType op_type) {
  dsa_stdexec::DsaScheduler<DsaProxy> scheduler(dsa);

  size_t num_ops = total_bytes / msg_size;
  size_t op_idx = 0;

  while (op_idx < num_ops) {
    size_t batch_end = std::min(op_idx + concurrency, num_ops);
    for (size_t i = op_idx; i < batch_end; ++i) {
      size_t offset = i * msg_size;
      spawn_op_scheduled(dsa, scheduler, scope, op_type, bufs, offset, msg_size, latency);
    }
    stdexec::sync_wait(scope.on_empty());
    op_idx = batch_end;
  }
}

// ============================================================================
// SCOPED WORKERS STRATEGY
// ============================================================================

exec::task<void> worker_coro(DsaProxy &dsa, BufferSet &bufs,
                              LatencyCollector &latency, OperationType op_type,
                              size_t msg_size, size_t num_ops,
                              size_t num_workers, size_t worker_id) {
  size_t current_op = worker_id;

  while (current_op < num_ops) {
    size_t offset = current_op * msg_size;
    auto start_time = std::chrono::high_resolution_clock::now();

    using namespace dsa_stdexec;
    switch (op_type) {
      case OperationType::DataMove:
        co_await dsa_data_move(dsa, bufs.src.data() + offset, bufs.dst.data() + offset, msg_size);
        break;
      case OperationType::MemFill:
        co_await dsa_mem_fill(dsa, bufs.dst.data() + offset, msg_size, BufferSet::fill_pattern);
        break;
      case OperationType::Compare:
        co_await dsa_compare(dsa, bufs.src.data() + offset, bufs.dst.data() + offset, msg_size);
        break;
      case OperationType::CompareValue:
        co_await dsa_compare_value(dsa, bufs.src.data() + offset, msg_size, BufferSet::fill_pattern);
        break;
      case OperationType::Dualcast:
        co_await dsa_dualcast(dsa, bufs.src.data() + offset, bufs.dualcast_dst1 + offset, bufs.dualcast_dst2 + offset, msg_size);
        break;
      case OperationType::CrcGen:
        co_await dsa_crc_gen(dsa, bufs.src.data() + offset, msg_size);
        break;
      case OperationType::CopyCrc:
        co_await dsa_copy_crc(dsa, bufs.src.data() + offset, bufs.dst.data() + offset, msg_size);
        break;
      case OperationType::CacheFlush:
        co_await dsa_cache_flush(dsa, bufs.dst.data() + offset, msg_size);
        break;
    }

    auto end_time = std::chrono::high_resolution_clock::now();
    latency.record(std::chrono::duration<double, std::nano>(end_time - start_time).count());
    current_op += num_workers;
  }

  co_return;
}

void run_scoped_workers_inline(DsaProxy &dsa, exec::async_scope &scope,
                               size_t concurrency, size_t msg_size, size_t total_bytes,
                               BufferSet &bufs, LatencyCollector &latency,
                               OperationType op_type) {
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.flush(); dsa.poll(); });
  auto scheduler = loop.get_scheduler();

  size_t num_ops = total_bytes / msg_size;
  size_t actual_workers = std::min(concurrency, num_ops);

  for (size_t worker_id = 0; worker_id < actual_workers; ++worker_id) {
    scope.spawn(
        scheduler.schedule()
      | stdexec::let_value([&dsa, &bufs, &latency, op_type, msg_size, num_ops, actual_workers, worker_id]() {
          return worker_coro(dsa, bufs, latency, op_type, msg_size, num_ops, actual_workers, worker_id);
        })
    );
  }

  dsa_stdexec::wait_start(scope.on_empty(), loop);
}

void run_scoped_workers_threaded(DsaProxy &dsa, exec::async_scope &scope,
                                 size_t concurrency, size_t msg_size, size_t total_bytes,
                                 BufferSet &bufs, LatencyCollector &latency,
                                 OperationType op_type) {
  dsa_stdexec::DsaScheduler<DsaProxy> scheduler(dsa);

  size_t num_ops = total_bytes / msg_size;
  size_t actual_workers = std::min(concurrency, num_ops);

  for (size_t worker_id = 0; worker_id < actual_workers; ++worker_id) {
    scope.spawn(
        scheduler.schedule()
      | stdexec::let_value([&dsa, &bufs, &latency, op_type, msg_size, num_ops, actual_workers, worker_id]() {
          return worker_coro(dsa, bufs, latency, op_type, msg_size, num_ops, actual_workers, worker_id);
        })
    );
  }

  stdexec::sync_wait(scope.on_empty());
}

// ============================================================================
// BENCHMARK INFRASTRUCTURE
// ============================================================================

// All run_* functions share this signature.
using StrategyFn = void(*)(DsaProxy &, exec::async_scope &, size_t, size_t, size_t,
                           BufferSet &, LatencyCollector &, OperationType);

// Indexed by [SchedulingPattern][PollingMode]: {inline, threaded}
static constexpr StrategyFn strategy_table[][2] = {
  /* SlidingWindow       */ { run_sliding_window_inline,          run_sliding_window_threaded },
  /* SlidingWindowNoAlloc*/ { run_sliding_window_inline_noalloc,  run_sliding_window_threaded_noalloc },
  /* SlidingWindowArena  */ { run_sliding_window_inline_arena,    run_sliding_window_threaded_arena },
  /* Batch              */  { run_batch_inline,                   run_batch_threaded },
  /* ScopedWorkers      */  { run_scoped_workers_inline,          run_scoped_workers_threaded },
};

void dispatch_run(SchedulingPattern sp, PollingMode pm, OperationType op_type,
                  DsaProxy &dsa, exec::async_scope &scope,
                  size_t concurrency, size_t msg_size, size_t total_bytes,
                  BufferSet &bufs, LatencyCollector &latency) {
  strategy_table[static_cast<int>(sp)][static_cast<int>(pm)](
      dsa, scope, concurrency, msg_size, total_bytes, bufs, latency, op_type);
}

DsaMetric run_benchmark(DsaProxy &dsa, size_t concurrency, size_t msg_size,
                        size_t total_bytes, int iterations,
                        BufferSet &bufs,
                        const RunFunction &run_fn,
                        ProgressBar *progress = nullptr) {
  LatencyCollector warmup_latency;
  LatencyCollector latency;

  // Pre-allocate latency sample storage to avoid reallocation during measurement
  size_t num_ops = total_bytes / msg_size;
  latency.reserve(num_ops * iterations);

  // Warmup (1 full iteration)
  {
    exec::async_scope scope;
    run_fn(dsa, scope, concurrency, msg_size, total_bytes, bufs, warmup_latency);
  }

  dsa_stdexec::reset_page_fault_retries();

  auto start = std::chrono::high_resolution_clock::now();
  for (int i = 0; i < iterations; ++i) {
    exec::async_scope scope;
    run_fn(dsa, scope, concurrency, msg_size, total_bytes, bufs, latency);
    if (progress) progress->increment();
  }
  auto end = std::chrono::high_resolution_clock::now();

  uint64_t page_faults = dsa_stdexec::get_page_fault_retries();
  std::chrono::duration<double> diff = end - start;
  double bw = static_cast<double>(total_bytes) * iterations / (1024.0 * 1024.0 * 1024.0) / diff.count();
  double msg_rate = static_cast<double>(num_ops) * iterations / 1e6 / diff.count();
  return {bw, msg_rate, page_faults, latency.compute_stats()};
}

std::string format_metric(const DsaMetric &m) {
  if (m.page_faults == 0) {
    return fmt::format("{:.2f}", m.bandwidth);
  } else {
    return fmt::format("{:.2f}({})", m.bandwidth, m.page_faults);
  }
}

void export_to_csv(const std::string &filename,
                   const std::vector<std::pair<std::string, std::vector<BenchmarkResult>>> &all_results) {
  std::ofstream file(filename);
  if (!file.is_open()) {
    fmt::println(stderr, "Failed to open {} for writing", filename);
    return;
  }

  file << "operation,pattern,polling_mode,queue_type,concurrency,msg_size,bandwidth_gbps,msg_rate_mps,page_faults,"
       << "latency_min_ns,latency_max_ns,latency_avg_ns,latency_p50_ns,latency_p99_ns,latency_count\n";

  auto write_row = [&file](const char *operation, const char *pattern, const char *polling_mode,
                            const char *queue_type, size_t concurrency,
                            size_t msg_size, const DsaMetric &m) {
    file << operation << "," << pattern << "," << polling_mode << "," << queue_type << ","
         << concurrency << "," << msg_size << "," << m.bandwidth << ","
         << m.msg_rate << "," << m.page_faults << "," << m.latency.min_ns << "," << m.latency.max_ns
         << "," << m.latency.avg_ns << "," << m.latency.p50_ns << ","
         << m.latency.p99_ns << "," << m.latency.count << "\n";
  };

  for (const auto &[label, results] : all_results) {
    size_t sep = label.find("__");
    std::string op_name = label.substr(0, sep);
    std::string rest = label.substr(sep + 2);
    size_t underscore_pos = rest.rfind('_');
    std::string pattern = rest.substr(0, underscore_pos);
    std::string polling_mode = rest.substr(underscore_pos + 1);
    bool include_nolock = (polling_mode == "inline");

    for (const auto &r : results) {
      if (include_nolock) {
        write_row(op_name.c_str(), pattern.c_str(), polling_mode.c_str(), "NoLock", r.concurrency, r.msg_size, r.single_thread);
      }
      write_row(op_name.c_str(), pattern.c_str(), polling_mode.c_str(), "Mutex", r.concurrency, r.msg_size, r.mutex);
      write_row(op_name.c_str(), pattern.c_str(), polling_mode.c_str(), "TAS", r.concurrency, r.msg_size, r.tas_spinlock);
      write_row(op_name.c_str(), pattern.c_str(), polling_mode.c_str(), "TTAS", r.concurrency, r.msg_size, r.ttas_spinlock);
      write_row(op_name.c_str(), pattern.c_str(), polling_mode.c_str(), "Backoff", r.concurrency, r.msg_size, r.backoff_spinlock);
      write_row(op_name.c_str(), pattern.c_str(), polling_mode.c_str(), "LockFree", r.concurrency, r.msg_size, r.lockfree);
    }
  }

  file.close();
  fmt::println("Results exported to {}", filename);
}

// Run a single queue type benchmark, creating the right concrete DSA type.
// Returns the metric, or empty metric if the queue type is skipped.
static DsaProxy make_dsa(QueueType qt, bool use_hw_batch, bool use_threaded_polling) {
  using dsa_stdexec::make_dsa_proxy;
  bool poller = (qt == QueueType::NoLock) ? false : use_threaded_polling;

  if (use_hw_batch) {
    switch (qt) {
      case QueueType::NoLock:   return make_dsa_proxy<DsaBatchSingleThread>(poller);
      case QueueType::Mutex:    return make_dsa_proxy<DsaBatch>(poller);
      case QueueType::TAS:      return make_dsa_proxy<DsaBatchTasSpinlock>(poller);
      case QueueType::TTAS:     return make_dsa_proxy<DsaBatchSpinlock>(poller);
      case QueueType::Backoff:  return make_dsa_proxy<DsaBatchBackoffSpinlock>(poller);
      case QueueType::LockFree: return make_dsa_proxy<DsaBatchLockFree>(poller);
    }
  } else {
    switch (qt) {
      case QueueType::NoLock:   return make_dsa_proxy<DsaSingleThread>(poller);
      case QueueType::Mutex:    return make_dsa_proxy<Dsa>(poller);
      case QueueType::TAS:      return make_dsa_proxy<DsaTasSpinlock>(poller);
      case QueueType::TTAS:     return make_dsa_proxy<DsaSpinlock>(poller);
      case QueueType::Backoff:  return make_dsa_proxy<DsaBackoffSpinlock>(poller);
      case QueueType::LockFree: return make_dsa_proxy<DsaLockFree>(poller);
    }
  }
  __builtin_unreachable();
}

static DsaMetric run_one_queue(QueueType qt, bool use_hw_batch, bool use_threaded_polling,
                               size_t concurrency, size_t msg_size, size_t total_bytes,
                               int iterations, BufferSet &bufs,
                               const RunFunction &run_fn, ProgressBar *progress) {
  auto dsa = make_dsa(qt, use_hw_batch, use_threaded_polling);
  return run_benchmark(dsa, concurrency, msg_size, total_bytes, iterations, bufs, run_fn, progress);
}

// Map QueueType enum to the corresponding BenchmarkResult field
static DsaMetric& result_field(BenchmarkResult &r, QueueType qt) {
  switch (qt) {
    case QueueType::NoLock:   return r.single_thread;
    case QueueType::Mutex:    return r.mutex;
    case QueueType::TAS:      return r.tas_spinlock;
    case QueueType::TTAS:     return r.ttas_spinlock;
    case QueueType::Backoff:  return r.backoff_spinlock;
    case QueueType::LockFree: return r.lockfree;
  }
  return r.mutex;  // unreachable
}

// Run benchmarks for all queue types with a given scheduling pattern, polling mode, and operation
std::vector<BenchmarkResult> run_all_queues(
    const BenchmarkConfig &config,
    BufferSet &bufs,
    SchedulingPattern sp,
    PollingMode pm,
    OperationType op_type,
    const char *pattern_name,
    bool use_hw_batch = false) {

  bool use_threaded_polling = (pm == PollingMode::Threaded);
  std::vector<BenchmarkResult> results;

  // Count enabled queues for progress tracking
  size_t queue_count = 0;
  for (auto qt : config.queue_types) {
    if (qt == QueueType::NoLock && use_threaded_polling) continue;
    queue_count++;
  }

  size_t total_configs = config.concurrency_levels.size() * config.msg_sizes.size();
  size_t total_iterations = total_configs * queue_count * config.iterations;

  std::string progress_label = fmt::format("{}/{}", operation_name(op_type), pattern_name);
  ProgressBar progress(total_iterations, progress_label);

  auto run_fn = [sp, pm, op_type](DsaProxy &d, exec::async_scope &scope, size_t c, size_t m, size_t t,
                                   BufferSet &b, LatencyCollector &l) {
    dispatch_run(sp, pm, op_type, d, scope, c, m, t, b, l);
  };

  for (auto concurrency : config.concurrency_levels) {
    for (auto msg_size : config.msg_sizes) {
      size_t effective_total_bytes = config.total_bytes;
      if (config.max_ops > 0) {
        size_t max_bytes = config.max_ops * msg_size;
        effective_total_bytes = std::min(config.total_bytes, max_bytes);
      }

      BenchmarkResult result{concurrency, msg_size, {}, {}, {}, {}, {}, {}};
      progress.set_label(fmt::format("{}/{} c={} sz={}", operation_name(op_type), pattern_name, concurrency, msg_size));

      for (auto qt : config.queue_types) {
        // NoLock is only valid for inline polling (single-threaded)
        if (qt == QueueType::NoLock && use_threaded_polling) continue;

        result_field(result, qt) = run_one_queue(
            qt, use_hw_batch, use_threaded_polling,
            concurrency, msg_size, effective_total_bytes,
            config.iterations, bufs, run_fn, &progress);
      }

      results.push_back(result);
    }
  }

  progress.finish();
  return results;
}

void benchmark_queues_with_dsa(const BenchmarkConfig &config) {
  if (config.operations.empty()) {
    fmt::println("No operations enabled.");
    return;
  }

  fmt::println("=== DSA BENCHMARK (DYNAMIC DISPATCH) WITH DIFFERENT TASK QUEUES ===\n");
  fmt::println("Configuration:");
  fmt::println("  Total bytes per iteration: {} MB", config.total_bytes / (1024 * 1024));
  fmt::println("  Iterations: {}", config.iterations);
  fmt::println("  Concurrency levels: {}", fmt::join(config.concurrency_levels, ", "));
  fmt::println("  Message sizes: {}", fmt::join(config.msg_sizes, ", "));
  fmt::println("  Operations: {}", [&] {
    std::string s;
    for (size_t i = 0; i < config.operations.size(); ++i) {
      if (i > 0) s += ", ";
      s += operation_name(config.operations[i]);
    }
    return s;
  }());
  fmt::println("");

  BufferSet bufs(config.total_bytes);

  std::vector<std::pair<std::string, std::vector<BenchmarkResult>>> all_results;

  for (auto op_type : config.operations) {
    const char *op_name = operation_name(op_type);
    for (auto sp : config.scheduling_patterns) {
      for (auto pm : config.polling_modes) {
        for (auto ss : config.submission_strategies) {
          bool hw_batch = (ss == SubmissionStrategy::HwBatch);
          const char *sp_name = scheduling_pattern_name(sp);
          const char *pm_name = polling_mode_name(pm);
          std::string label_name = sp_name;
          if (hw_batch) label_name += "_hwbatch";

          fmt::println("Running {} {} + {} polling{}...", op_name, sp_name, pm_name,
                       hw_batch ? " (hw batch)" : "");
          auto results = run_all_queues(config, bufs, sp, pm, op_type,
                                         label_name.c_str(), hw_batch);
          all_results.emplace_back(fmt::format("{}__{}_{}", op_name, label_name, pm_name),
                                    std::move(results));
          fmt::println("");
        }
      }
    }
  }

  // Print results tables
  fmt::println("==============================================================="
               "=================");
  fmt::println("                              BENCHMARK RESULTS");
  fmt::println("==============================================================="
               "=================\n");

  auto print_results_table = [](const std::string& title,
                                 const std::vector<BenchmarkResult>& results,
                                 bool include_nolock) {
    if (results.empty()) return;

    fmt::println("========== {} ==========\n", title);
    if (include_nolock) {
      fmt::println("{:>5} {:>10} {:>16} {:>16} {:>16} {:>16} {:>16} {:>16}",
                   "Conc", "Size", "NoLock", "Mutex", "TAS", "TTAS", "Backoff",
                   "LockFree");
      fmt::println(
          "{:-^5} {:-^10} {:-^16} {:-^16} {:-^16} {:-^16} {:-^16} {:-^16}",
          "", "", "", "", "", "", "", "");
      for (const auto &r : results) {
        fmt::println(
            "{:>5} {:>10} {:>16} {:>16} {:>16} {:>16} {:>16} {:>16}",
            r.concurrency, r.msg_size, format_metric(r.single_thread),
            format_metric(r.mutex), format_metric(r.tas_spinlock),
            format_metric(r.ttas_spinlock), format_metric(r.backoff_spinlock),
            format_metric(r.lockfree));
      }
    } else {
      fmt::println("{:>5} {:>10} {:>16} {:>16} {:>16} {:>16} {:>16}",
                   "Conc", "Size", "Mutex", "TAS", "TTAS", "Backoff", "LockFree");
      fmt::println("{:-^5} {:-^10} {:-^16} {:-^16} {:-^16} {:-^16} {:-^16}",
                   "", "", "", "", "", "", "");
      for (const auto &r : results) {
        fmt::println("{:>5} {:>10} {:>16} {:>16} {:>16} {:>16} {:>16}",
                     r.concurrency, r.msg_size, format_metric(r.mutex),
                     format_metric(r.tas_spinlock), format_metric(r.ttas_spinlock),
                     format_metric(r.backoff_spinlock), format_metric(r.lockfree));
      }
    }
    fmt::println("");
  };

  for (const auto &[label, results] : all_results) {
    bool include_nolock = label.find("inline") != std::string::npos;
    std::string title = label;
    std::transform(title.begin(), title.end(), title.begin(), ::toupper);
    print_results_table(title, results, include_nolock);
  }

  export_to_csv(config.csv_file, all_results);
}

int main(int argc, char **argv) {
  BenchmarkConfig config = parse_args(argc, argv);

  try {
    benchmark_queues_with_dsa(config);
    fmt::println("");
    fmt::println("Benchmark completed.");
  } catch (const std::exception &e) {
    fmt::println(stderr, "Error: {}", e.what());
    return 1;
  }

  return 0;
}
