#include "strategy_common.hpp"

void run_sliding_window_inline(DsaProxy &dsa, exec::async_scope &scope,
                               size_t concurrency, size_t msg_size, size_t total_bytes,
                               BufferSet &bufs, LatencyCollector &latency,
                               OperationType op_type) {
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

  size_t num_ops = total_bytes / msg_size;
  std::atomic<size_t> in_flight{0};
  size_t next_op = 0;

  while (next_op < num_ops) {
    while (next_op < num_ops && in_flight.load(std::memory_order_acquire) < concurrency) {
      size_t offset = next_op * msg_size;
      spawn_op(dsa, scope, op_type, bufs, offset, msg_size, latency, &in_flight);
      ++next_op;
    }
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
