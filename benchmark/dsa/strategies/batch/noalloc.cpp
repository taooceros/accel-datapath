#include "strategy_common.hpp"

template <class MakeSender>
static void batch_noalloc_impl_inline(
    DsaProxy &dsa, exec::async_scope &scope,
    size_t concurrency, size_t msg_size, size_t total_bytes,
    BufferSet &bufs, LatencyCollector &latency,
    MakeSender make_sender) {
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

  constexpr size_t SlotSize = inline_noalloc_slot_size<MakeSender>();
  size_t num_ops = total_bytes / msg_size;

  std::vector<std::unique_ptr<OperationSlot<SlotSize>>> slots;
  slots.reserve(concurrency);
  for (size_t i = 0; i < concurrency; ++i)
    slots.push_back(std::make_unique<OperationSlot<SlotSize>>());

  size_t op_idx = 0;
  while (op_idx < num_ops) {
    size_t batch_end = std::min(op_idx + concurrency, num_ops);
    size_t batch_size = batch_end - op_idx;

    // Fill phase: start all ops in this batch
    for (size_t i = 0; i < batch_size; ++i) {
      size_t offset = (op_idx + i) * msg_size;
      auto record = CompletionRecord::make(latency, nullptr);
      slots[i]->start_op(scope.nest(make_sender(offset) | stdexec::then(record)));
    }

    // Barrier phase: poll until all batch slots complete
    for (;;) {
      dsa.poll();
      bool all_done = true;
      for (size_t i = 0; i < batch_size; ++i) {
        if (!slots[i]->ready.load(std::memory_order_acquire)) {
          all_done = false;
          break;
        }
      }
      if (all_done) break;
    }

    op_idx = batch_end;
  }
}

void run_batch_noalloc_inline(const StrategyParams &params) {
  auto &[dsa, scope, concurrency, msg_size, total_bytes, batch_size, bufs, latency, op_type] = params;
  (void)batch_size;
  with_op_sender(op_type, dsa, bufs, msg_size, [&](auto op_sender) {
    batch_noalloc_impl_inline(dsa, scope, concurrency, msg_size, total_bytes,
                              bufs, latency, op_sender);
  });
}

template <class MakeSender>
static void batch_noalloc_impl_threaded(
    DsaProxy &dsa, exec::async_scope &scope,
    size_t concurrency, size_t msg_size, size_t total_bytes,
    BufferSet &bufs, LatencyCollector &latency,
    MakeSender make_sender) {
  dsa_stdexec::DsaScheduler<DsaProxy> scheduler(dsa);

  constexpr size_t SlotSize = threaded_noalloc_slot_size<MakeSender>();
  size_t num_ops = total_bytes / msg_size;

  std::vector<std::unique_ptr<OperationSlot<SlotSize>>> slots;
  slots.reserve(concurrency);
  for (size_t i = 0; i < concurrency; ++i)
    slots.push_back(std::make_unique<OperationSlot<SlotSize>>());

  size_t op_idx = 0;
  while (op_idx < num_ops) {
    size_t batch_end = std::min(op_idx + concurrency, num_ops);
    size_t batch_size = batch_end - op_idx;

    // Fill phase: start all ops in this batch
    for (size_t i = 0; i < batch_size; ++i) {
      size_t offset = (op_idx + i) * msg_size;
      auto record = CompletionRecord::make(latency, nullptr);
      slots[i]->start_op(scope.nest(
        scheduler.schedule() | stdexec::let_value([make_sender, offset, record]() {
          return make_sender(offset) | stdexec::then(record);
        })
      ));
    }

    // Barrier phase: yield until all batch slots complete
    for (;;) {
      std::this_thread::yield();
      bool all_done = true;
      for (size_t i = 0; i < batch_size; ++i) {
        if (!slots[i]->ready.load(std::memory_order_acquire)) {
          all_done = false;
          break;
        }
      }
      if (all_done) break;
    }

    op_idx = batch_end;
  }
}

void run_batch_noalloc_threaded(const StrategyParams &params) {
  auto &[dsa, scope, concurrency, msg_size, total_bytes, batch_size, bufs, latency, op_type] = params;
  (void)batch_size;
  with_op_sender(op_type, dsa, bufs, msg_size, [&](auto op_sender) {
    batch_noalloc_impl_threaded(dsa, scope, concurrency, msg_size, total_bytes,
                                bufs, latency, op_sender);
  });
}
