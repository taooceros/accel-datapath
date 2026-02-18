#include "strategy_common.hpp"

template <class MakeSender>
static void sliding_window_noalloc_impl_inline(
    DsaProxy &dsa, exec::async_scope &scope,
    size_t concurrency, size_t msg_size, size_t total_bytes,
    BufferSet &bufs, LatencyCollector &latency,
    MakeSender make_sender) {
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

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
      auto record = CompletionRecord::make(latency, &in_flight);
      slot->start_op(scope.nest(make_sender(offset) | stdexec::then(record)));
      ++next_op;
    }
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
          auto record = CompletionRecord::make(latency, &in_flight);
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
