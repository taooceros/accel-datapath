// Benchmark strategy implementations — the template-heavy code.
// Split into its own TU to enable parallel compilation.

#include <fmt/format.h>
#include <fmt/ranges.h>
#include "strategies.hpp"
#include <chrono>
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
#include <exec/task.hpp>
#include <stdexec/execution.hpp>
#include <thread>

// ============================================================================
// OPERATION DISPATCH — single switch for all op_type → sender mappings
// ============================================================================

// Calls f(op_sender) where op_sender is a lambda: (size_t offset) -> Sender.
// Each operation type produces a different sender type, so f must be generic.
template <class F>
static void with_op_sender(OperationType op_type, DsaProxy &dsa,
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

static void spawn_op(DsaProxy &dsa, exec::async_scope &scope, OperationType op_type,
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

static void spawn_op_scheduled(DsaProxy &dsa, dsa_stdexec::DsaScheduler<DsaProxy> &scheduler,
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
// ============================================================================

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

template <class MakeSender>
constexpr size_t inline_noalloc_slot_size() {
  using Sender = decltype(std::declval<MakeSender>()(size_t{0}));
  using ThenSender = decltype(std::declval<Sender>() | stdexec::then(std::declval<NoAllocRecord>()));
  using NestSender = exec::async_scope::nest_result_t<ThenSender>;
  return sizeof(stdexec::connect_result_t<NestSender, SlotReceiver>);
}

template <class MakeSender>
static void sliding_window_noalloc_impl_inline(
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

template <class MakeSender>
constexpr size_t threaded_noalloc_slot_size() {
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
  return sizeof(stdexec::connect_result_t<NestSender, SlotReceiver>);
}

template <class MakeSender>
static void sliding_window_noalloc_impl_threaded(
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
static void sliding_window_arena_impl_inline(
    DsaProxy &dsa, exec::async_scope &scope,
    size_t concurrency, size_t msg_size, size_t total_bytes,
    BufferSet &bufs, LatencyCollector &latency,
    MakeSender make_sender) {
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.flush(); dsa.poll(); });

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
static void sliding_window_arena_impl_threaded(
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

static exec::task<void> worker_coro(DsaProxy &dsa, BufferSet &bufs,
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
