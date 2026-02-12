#pragma once
#ifndef DSA_RING_BATCH_HPP
#define DSA_RING_BATCH_HPP

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

// Ring-buffer batching wrapper for DsaBase.
//
// Uses two separate ring buffers:
//   - Descriptor ring: large contiguous array of dsa_hw_desc slots
//   - Batch ring: small metadata array, each entry references a range in the
//     descriptor ring and carries its own batch completion record
//
// No explicit flush() — submit() auto-submits full batches and poll() drains
// partial ones. Small batches use only the descriptor slots they need.
//
// Composition over inheritance: has-a DsaBase, forwards task_queue().
// Operation senders work unchanged — they only call dsa.submit(op, desc).

template <template <typename> class QueueTemplate = dsa::MutexTaskQueue>
class DsaRingBatchBase {
public:
  using Inner = DsaBase<QueueTemplate>;
  using Queue = typename Inner::Queue;

  explicit DsaRingBatchBase(bool start_poller = true);
  ~DsaRingBatchBase();

  void submit(dsa_stdexec::OperationBase *op, dsa_hw_desc *desc);
  void submit(dsa_stdexec::OperationBase *op);
  void poll();
  void flush() {} // no-op — batching handled by submit() and poll()

  Queue &task_queue() noexcept { return inner_.task_queue(); }
  const Queue &task_queue() const noexcept { return inner_.task_queue(); }

private:
  Inner inner_;

  // --- Descriptor ring ---
  static constexpr size_t kDescRingSize = 256;
  static_assert((kDescRingSize & (kDescRingSize - 1)) == 0,
                "kDescRingSize must be power of 2");
  alignas(64) dsa_hw_desc desc_ring_[kDescRingSize];
  size_t desc_head_ = 0; // oldest in-use slot (freed by reclaim)
  size_t desc_tail_ = 0; // next free slot (advanced by submit)

  // --- Batch metadata ring ---
  static constexpr size_t kMaxBatches = 16;
  static_assert((kMaxBatches & (kMaxBatches - 1)) == 0,
                "kMaxBatches must be power of 2");

  enum class BatchState : uint8_t { Free, Filling, InFlight };

  struct BatchEntry {
    alignas(32) dsa_completion_record batch_comp;
    size_t start;     // index into desc_ring_ where this batch begins
    uint32_t count;   // number of descriptors in this batch
    BatchState state;
  };

  BatchEntry batches_[kMaxBatches];
  size_t batch_fill_ = 0; // batch currently being filled
  size_t batch_head_ = 0; // oldest non-Free batch (for ordered reclaim)

  // Per-batch hw limit (queried from device at init)
  size_t max_batch_size_ = 32;

  // Poller thread
  std::thread poller_;
  std::atomic<bool> running_{false};

  // Helpers
  void reclaim_completed();
  void submit_batch(BatchEntry &batch);
  void seal_and_submit_current();

  size_t desc_index(size_t pos) const { return pos & (kDescRingSize - 1); }
  size_t batch_index(size_t pos) const { return pos & (kMaxBatches - 1); }
  size_t desc_available() const { return kDescRingSize - (desc_tail_ - desc_head_); }

  DsaRingBatchBase(const DsaRingBatchBase &) = delete;
  DsaRingBatchBase &operator=(const DsaRingBatchBase &) = delete;
};

// Type aliases paralleling DsaBatchBase aliases
using DsaRingBatch = DsaRingBatchBase<dsa::MutexTaskQueue>;
using DsaRingBatchSingleThread = DsaRingBatchBase<dsa::SingleThreadTaskQueue>;
using DsaRingBatchTasSpinlock = DsaRingBatchBase<dsa::TasSpinlockTaskQueue>;
using DsaRingBatchSpinlock = DsaRingBatchBase<dsa::SpinlockTaskQueue>;
using DsaRingBatchBackoffSpinlock =
    DsaRingBatchBase<dsa::BackoffSpinlockTaskQueue>;
using DsaRingBatchLockFree = DsaRingBatchBase<dsa::LockFreeTaskQueue>;

#include "dsa_ring_batch.ipp"

#endif
