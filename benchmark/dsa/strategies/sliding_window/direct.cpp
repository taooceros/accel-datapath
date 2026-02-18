#include "strategy_common.hpp"

template <class MakeSender>
static void sliding_window_direct_impl_inline(
    DsaProxy &dsa, size_t concurrency, size_t msg_size, size_t total_bytes,
    BufferSet &bufs, LatencyCollector &latency,
    MakeSender make_sender) {
  constexpr size_t SlotSize = direct_arena_slot_size<MakeSender>();
  size_t num_ops = total_bytes / msg_size;
  std::atomic<size_t> remaining{num_ops};
  SlotArena<SlotSize> arena(concurrency);

  size_t next_op = 0;
  while (next_op < num_ops) {
    while (next_op < num_ops) {
      auto *slot = arena.acquire();
      if (!slot) break;
      size_t offset = next_op * msg_size;
      slot->start_op_with(
        make_sender(offset),
        DirectBenchReceiver<SlotSize>{
          &arena, slot, &remaining, &latency,
          latency.enabled() ? std::chrono::high_resolution_clock::now()
                            : std::chrono::high_resolution_clock::time_point{}
        });
      ++next_op;
    }
    dsa.poll();
  }

  // Drain: poll until all ops complete
  while (remaining.load(std::memory_order_acquire) > 0) {
    dsa.poll();
  }
}

void run_sliding_window_inline_direct(const StrategyParams &params) {
  auto &[dsa, scope, concurrency, msg_size, total_bytes, batch_size, bufs, latency, op_type] = params;
  (void)scope;      // Direct strategy bypasses async_scope entirely
  (void)batch_size;
  with_op_sender(op_type, dsa, bufs, msg_size, [&](auto op_sender) {
    sliding_window_direct_impl_inline(dsa, concurrency, msg_size, total_bytes, bufs, latency, op_sender);
  });
}
