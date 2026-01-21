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

#include "dsa_operation_base.hpp"
#include "task_queue.hpp"
#include <dsa_stdexec/operation_base.hpp>

struct AccfgCtxDeleter {
  void operator()(accfg_ctx *ctx) const noexcept { accfg_unref(ctx); }
};

// Hardware context for DSA accelerator
// Satisfies the HwContext concept (check_completion) for task queue polling
// Also provides submit() for use by DsaBase::submit()
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
    // Flush the cache line containing the completion record to ensure
    // we see the hardware's DMA write, then reload from memory
    _mm_clflush(comp);
    _mm_lfence();

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

template <template <typename> class QueueTemplate = dsa::MutexTaskQueue>
class DsaBase {
public:
  using Queue = DsaTaskQueue<QueueTemplate>;

  explicit DsaBase(bool start_poller = true);
  ~DsaBase();

  void data_move(void *src, void *dst, size_t size);

  constexpr AccfgCtx const &context() const noexcept { return ctx_; }

  void submit(dsa_stdexec::OperationBase *op, dsa_hw_desc *desc);
  void submit(dsa_stdexec::OperationBase *op);
  void poll();

  Queue &task_queue() noexcept { return task_queue_; }
  const Queue &task_queue() const noexcept { return task_queue_; }

private:
  AccfgCtx ctx_;

  accfg_wq *wq_;

  enum accfg_wq_mode mode_;

  void *wq_portal_;

  static constexpr std::size_t kWqPortalSize = 0x1000;

  void *map_wq(accfg_wq *wq);

  Queue task_queue_;
  std::thread poller_;
  std::atomic<bool> running_{false};

  DsaBase(const DsaBase &) = delete;
  DsaBase &operator=(const DsaBase &) = delete;
};

// Default Dsa type using mutex-based queue
using Dsa = DsaBase<dsa::MutexTaskQueue>;

// Convenience aliases for different queue strategies
using DsaSingleThread = DsaBase<dsa::SingleThreadTaskQueue>;
using DsaTasSpinlock = DsaBase<dsa::TasSpinlockTaskQueue>;
using DsaSpinlock = DsaBase<dsa::SpinlockTaskQueue>;  // TTAS
using DsaBackoffSpinlock = DsaBase<dsa::BackoffSpinlockTaskQueue>;
using DsaLockFree = DsaBase<dsa::LockFreeTaskQueue>;

#include "dsa.ipp"

#endif
