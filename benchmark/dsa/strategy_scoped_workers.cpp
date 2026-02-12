#include "strategy_common.hpp"
#include <exec/task.hpp>

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
