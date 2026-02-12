#include "strategy_common.hpp"

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
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

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
