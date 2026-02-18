#include "strategy_common.hpp"

void run_batch_inline(DsaProxy &dsa, exec::async_scope &scope,
                      size_t concurrency, size_t msg_size, size_t total_bytes,
                      BufferSet &bufs, LatencyCollector &latency,
                      OperationType op_type) {
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

  size_t num_ops = total_bytes / msg_size;
  size_t op_idx = 0;

  while (op_idx < num_ops) {
    size_t batch_end = std::min(op_idx + concurrency, num_ops);
    for (size_t i = op_idx; i < batch_end; ++i) {
      size_t offset = i * msg_size;
      spawn_op(dsa, scope, op_type, bufs, offset, msg_size, latency);
    }
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
