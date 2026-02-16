#pragma once
#ifndef DSA_INIT_HPP
#define DSA_INIT_HPP

#include <atomic>
#include <cstddef>
#include <memory>
#include <thread>
#include <x86intrin.h>

extern "C" {
#include <accel-config/libaccel_config.h>
#include <linux/idxd.h>
}

#include "descriptor_submitter.hpp"
#include "dsa_operation_base.hpp"
#include "task_queue.hpp"
#include <dsa_stdexec/operation_base.hpp>

struct AccfgCtxDeleter {
  void operator()(accfg_ctx *ctx) const noexcept { accfg_unref(ctx); }
};

// Hardware context for DSA accelerator
// Satisfies the HwContext concept (check_completion) for task queue polling
class DsaHwContext {
public:
  DsaHwContext() = default;
  DsaHwContext(void *wq_portal, accfg_wq_mode mode)
      : wq_portal_(wq_portal), mode_(mode) {}

  void set_context(void *wq_portal, accfg_wq_mode mode) {
    wq_portal_ = wq_portal;
    mode_ = mode;
  }

  // Check if an operation has completed by examining its completion record
  // Required by HwContext concept for task queue polling
  bool check_completion(dsa_stdexec::OperationBase *op) const {
    auto *dsa_op = static_cast<dsa::DsaOperationBase *>(op);
    auto *comp = dsa_op->comp_ptr();
    return comp->status != 0;
  }

  void *portal() const { return wq_portal_; }
  accfg_wq_mode mode() const { return mode_; }

private:
  void *wq_portal_ = nullptr;
  accfg_wq_mode mode_ = ACCFG_WQ_SHARED;
};

class AccfgCtx {
public:
  AccfgCtx() {
    accfg_ctx *ctx = nullptr;
    if (accfg_new(&ctx) != 0 || ctx == nullptr) {
      throw std::runtime_error("accfg_new failed to create libaccfg context");
    }
    ctx_.reset(ctx);
  }

  accfg_ctx *get() const noexcept { return ctx_.get(); }

private:
  std::unique_ptr<accfg_ctx, AccfgCtxDeleter> ctx_;
};

// Template alias for queue types with DSA hardware context
template <template <typename> class QueueTemplate>
using DsaTaskQueue = QueueTemplate<DsaHwContext>;

// Unified DSA engine parameterized by descriptor submission strategy and queue type.
// Submitter controls how descriptors reach hardware (direct MMIO, staged batch, ring).
// QueueTemplate controls completion tracking synchronization (mutex, spinlock, etc).
template <DescriptorSubmitter Submitter = DirectSubmitter,
          template <typename> class QueueTemplate = dsa::MutexTaskQueue>
class DsaEngine {
public:
  using Queue = DsaTaskQueue<QueueTemplate>;

  explicit DsaEngine(bool start_poller = true);
  ~DsaEngine();

  void data_move(void *src, void *dst, size_t size);

  constexpr AccfgCtx const &context() const noexcept { return ctx_; }

  void submit(dsa_stdexec::OperationBase *op, dsa_hw_desc *desc);
  void submit(dsa_stdexec::OperationBase *op);
  void submit_raw(dsa_hw_desc *desc);
  void poll();
  void flush() { submitter_.flush(); }

  accfg_wq *wq() const noexcept { return wq_; }

  Queue &task_queue() noexcept { return task_queue_; }
  const Queue &task_queue() const noexcept { return task_queue_; }

private:
  AccfgCtx ctx_;

  accfg_wq *wq_;

  enum accfg_wq_mode mode_;

  void *wq_portal_;

  static constexpr std::size_t kWqPortalSize = 0x1000;

  void *map_wq(accfg_wq *wq);

  Submitter submitter_;
  Queue task_queue_;
  std::thread poller_;
  std::atomic<bool> running_{false};

  DsaEngine(const DsaEngine &) = delete;
  DsaEngine &operator=(const DsaEngine &) = delete;
};

// Backwards-compatible alias: DsaBase<Q> = DsaEngine<DirectSubmitter, Q>
template <template <typename> class QueueTemplate = dsa::MutexTaskQueue>
using DsaBase = DsaEngine<DirectSubmitter, QueueTemplate>;

// Default Dsa type using mutex-based queue
using Dsa = DsaEngine<DirectSubmitter, dsa::MutexTaskQueue>;

// Convenience aliases for different queue strategies
using DsaSingleThread = DsaEngine<DirectSubmitter, dsa::SingleThreadTaskQueue>;
using DsaTasSpinlock = DsaEngine<DirectSubmitter, dsa::TasSpinlockTaskQueue>;
using DsaSpinlock = DsaEngine<DirectSubmitter, dsa::SpinlockTaskQueue>;  // TTAS
using DsaBackoffSpinlock = DsaEngine<DirectSubmitter, dsa::BackoffSpinlockTaskQueue>;
using DsaLockFree = DsaEngine<DirectSubmitter, dsa::LockFreeTaskQueue>;

// Double-buffered batch submission (was DsaBatchBase)
using DsaBatch = DsaEngine<StagingSubmitter, dsa::MutexTaskQueue>;
using DsaBatchSingleThread = DsaEngine<StagingSubmitter, dsa::SingleThreadTaskQueue>;
using DsaBatchTasSpinlock = DsaEngine<StagingSubmitter, dsa::TasSpinlockTaskQueue>;
using DsaBatchSpinlock = DsaEngine<StagingSubmitter, dsa::SpinlockTaskQueue>;
using DsaBatchBackoffSpinlock = DsaEngine<StagingSubmitter, dsa::BackoffSpinlockTaskQueue>;
using DsaBatchLockFree = DsaEngine<StagingSubmitter, dsa::LockFreeTaskQueue>;

// Fixed ring batch submission (was DsaFixedRingBatchBase)
using DsaFixedRingBatch = DsaEngine<FixedRingSubmitter, dsa::MutexTaskQueue>;
using DsaFixedRingBatchSingleThread = DsaEngine<FixedRingSubmitter, dsa::SingleThreadTaskQueue>;
using DsaFixedRingBatchTasSpinlock = DsaEngine<FixedRingSubmitter, dsa::TasSpinlockTaskQueue>;
using DsaFixedRingBatchSpinlock = DsaEngine<FixedRingSubmitter, dsa::SpinlockTaskQueue>;
using DsaFixedRingBatchBackoffSpinlock = DsaEngine<FixedRingSubmitter, dsa::BackoffSpinlockTaskQueue>;
using DsaFixedRingBatchLockFree = DsaEngine<FixedRingSubmitter, dsa::LockFreeTaskQueue>;

// Ring batch submission (was DsaRingBatchBase)
using DsaRingBatch = DsaEngine<RingSubmitter, dsa::MutexTaskQueue>;
using DsaRingBatchSingleThread = DsaEngine<RingSubmitter, dsa::SingleThreadTaskQueue>;
using DsaRingBatchTasSpinlock = DsaEngine<RingSubmitter, dsa::TasSpinlockTaskQueue>;
using DsaRingBatchSpinlock = DsaEngine<RingSubmitter, dsa::SpinlockTaskQueue>;
using DsaRingBatchBackoffSpinlock = DsaEngine<RingSubmitter, dsa::BackoffSpinlockTaskQueue>;
using DsaRingBatchLockFree = DsaEngine<RingSubmitter, dsa::LockFreeTaskQueue>;

// Mirrored ring batch submission (wrap-free via virtual memory mirroring)
using DsaMirroredRingBatch = DsaEngine<MirroredRingSubmitter, dsa::MutexTaskQueue>;
using DsaMirroredRingBatchSingleThread = DsaEngine<MirroredRingSubmitter, dsa::SingleThreadTaskQueue>;
using DsaMirroredRingBatchTasSpinlock = DsaEngine<MirroredRingSubmitter, dsa::TasSpinlockTaskQueue>;
using DsaMirroredRingBatchSpinlock = DsaEngine<MirroredRingSubmitter, dsa::SpinlockTaskQueue>;
using DsaMirroredRingBatchBackoffSpinlock = DsaEngine<MirroredRingSubmitter, dsa::BackoffSpinlockTaskQueue>;
using DsaMirroredRingBatchLockFree = DsaEngine<MirroredRingSubmitter, dsa::LockFreeTaskQueue>;

#include "dsa.ipp"

#endif
