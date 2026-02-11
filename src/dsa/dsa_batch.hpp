#pragma once
#ifndef DSA_BATCH_HPP
#define DSA_BATCH_HPP

#include "dsa.hpp"
#include <atomic>
#include <cstddef>
#include <cstring>
#include <thread>
#include <x86intrin.h>

extern "C" {
#include <accel-config/libaccel_config.h>
#include <linux/idxd.h>
}

// Transparent batching wrapper for DsaBase.
// Stages descriptors in a double-buffered array and submits them as a
// hardware batch (opcode 0x01) on flush(), reducing MMIO doorbell writes.
//
// Composition over inheritance: has-a DsaBase, forwards poll()/task_queue().
// Operation senders work unchanged — they only call dsa.submit(op, desc).

template <template <typename> class QueueTemplate = dsa::MutexTaskQueue>
class DsaBatchBase {
public:
  using Inner = DsaBase<QueueTemplate>;
  using Queue = typename Inner::Queue;

  explicit DsaBatchBase(bool start_poller = true);
  ~DsaBatchBase();

  // Same interface as DsaBase — senders don't know the difference
  void submit(dsa_stdexec::OperationBase *op, dsa_hw_desc *desc);
  void submit(dsa_stdexec::OperationBase *op);
  void poll();
  void flush();

  Queue &task_queue() noexcept { return inner_.task_queue(); }
  const Queue &task_queue() const noexcept { return inner_.task_queue(); }

private:
  Inner inner_;

  // Double-buffered staging array for sub-descriptors
  static constexpr size_t kMaxStagingSize = 32;
  alignas(64) dsa_hw_desc staged_[2][kMaxStagingSize];
  size_t staged_count_ = 0;
  int active_buf_ = 0;

  // Batch completion records (one per buffer, for lifetime tracking)
  alignas(32) dsa_completion_record batch_comp_[2] = {};
  bool batch_submitted_[2] = {false, false};

  // Device batch size limit (queried from hardware at init)
  size_t max_batch_size_ = kMaxStagingSize;

  // Own poller thread (flush + poll, not delegated to inner DsaBase)
  std::thread poller_;
  std::atomic<bool> running_{false};

  DsaBatchBase(const DsaBatchBase &) = delete;
  DsaBatchBase &operator=(const DsaBatchBase &) = delete;
};

// Type aliases paralleling the existing DsaBase aliases
using DsaBatch = DsaBatchBase<dsa::MutexTaskQueue>;
using DsaBatchSingleThread = DsaBatchBase<dsa::SingleThreadTaskQueue>;
using DsaBatchTasSpinlock = DsaBatchBase<dsa::TasSpinlockTaskQueue>;
using DsaBatchSpinlock = DsaBatchBase<dsa::SpinlockTaskQueue>;
using DsaBatchBackoffSpinlock = DsaBatchBase<dsa::BackoffSpinlockTaskQueue>;
using DsaBatchLockFree = DsaBatchBase<dsa::LockFreeTaskQueue>;

#include "dsa_batch.ipp"

#endif
