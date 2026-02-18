#include "strategy_common.hpp"

void run_batch_raw_inline(DsaProxy &dsa, exec::async_scope & /*scope*/,
                          size_t concurrency, size_t msg_size,
                          size_t total_bytes, BufferSet &bufs,
                          LatencyCollector & /*latency*/,
                          OperationType op_type) {
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

  size_t num_ops = total_bytes / msg_size;
  size_t op_idx = 0;

  while (op_idx < num_ops) {
    size_t batch_size = std::min(concurrency, num_ops - op_idx);

    auto sndr = dsa_stdexec::dsa_batch(
        dsa, batch_size, [&](std::span<dsa_hw_desc> descs) {
          for (size_t i = 0; i < descs.size(); ++i) {
            size_t offset = (op_idx + i) * msg_size;
            fill_for_op(descs[i], op_type, bufs, offset, msg_size);
          }
        });

    dsa_stdexec::wait_start(std::move(sndr), loop);
    loop.reset();
    op_idx += batch_size;
  }
}
