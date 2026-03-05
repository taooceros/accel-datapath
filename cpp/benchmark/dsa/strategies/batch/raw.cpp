#include "strategy_common.hpp"

void run_batch_raw_inline(const StrategyParams &params) {
  auto &[dsa, scope, concurrency, msg_size, total_bytes, batch_size, bufs, latency, op_type] = params;
  (void)scope;
  (void)batch_size;
  (void)latency;
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

  size_t num_ops = total_bytes / msg_size;
  size_t op_idx = 0;

  while (op_idx < num_ops) {
    size_t cur_batch = std::min(concurrency, num_ops - op_idx);

    auto sndr = dsa_stdexec::dsa_batch(
        dsa, cur_batch, [&](std::span<dsa_hw_desc> descs) {
          for (size_t i = 0; i < descs.size(); ++i) {
            size_t offset = (op_idx + i) * msg_size;
            fill_for_op(descs[i], op_type, bufs, offset, msg_size);
          }
        });

    dsa_stdexec::wait_start(std::move(sndr), loop);
    loop.reset();
    op_idx += cur_batch;
  }
}
