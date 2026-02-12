#pragma once
#ifndef DSA_FIXED_RING_BATCH_HPP
#define DSA_FIXED_RING_BATCH_HPP

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

// Fixed ring-buffer batching wrapper for DsaBase (ablation study).
//
// Unlike DsaRingBatchBase which uses a shared descriptor ring with dynamic
// range allocation per batch, this variant uses a simple ring of fixed-size
// batch entries. Each entry owns its own contiguous descriptor array of
// kBatchCapacity slots plus a batch completion record.
//
// Trade-offs vs DsaRingBatchBase:
//   + No wrap-around handling or dynamic range tracking
//   + Simpler state: only batch_fill_ and batch_head_, no desc_head_/desc_tail_
//   - Wastes descriptor space when batches are small
//   - Higher memory footprint (each entry reserves full kBatchCapacity)
//
// Composition over inheritance: has-a DsaBase, forwards task_queue().
// Operation senders work unchanged — they only call dsa.submit(op, desc).

template <template <typename> class QueueTemplate = dsa::MutexTaskQueue>
class DsaFixedRingBatchBase {
public:
  using Inner = DsaBase<QueueTemplate>;
  using Queue = typename Inner::Queue;

  explicit DsaFixedRingBatchBase(bool start_poller = true);
  ~DsaFixedRingBatchBase();

  void submit(dsa_stdexec::OperationBase *op, dsa_hw_desc *desc);
  void submit(dsa_stdexec::OperationBase *op);
  void poll();
  void flush() {} // no-op — batching handled by submit() and poll()

  Queue &task_queue() noexcept { return inner_.task_queue(); }
  const Queue &task_queue() const noexcept { return inner_.task_queue(); }

private:
  Inner inner_;

  // --- Fixed-size batch ring ---
  static constexpr size_t kMaxBatches = 16;
  static_assert((kMaxBatches & (kMaxBatches - 1)) == 0,
                "kMaxBatches must be power of 2");

  static constexpr size_t kBatchCapacity = 32;

  enum class BatchState : uint8_t { Free, Filling, InFlight };

  struct BatchEntry {
    alignas(64) dsa_hw_desc descs[kBatchCapacity];
    alignas(32) dsa_completion_record comp;
    uint32_t count;
    BatchState state;
  };

  BatchEntry batches_[kMaxBatches];
  size_t batch_fill_ = 0; // batch currently being filled
  size_t batch_head_ = 0; // oldest non-Free batch (for ordered reclaim)

  // Per-batch hw limit (queried from device at init)
  size_t max_batch_size_ = kBatchCapacity;

  // Poller thread
  std::thread poller_;
  std::atomic<bool> running_{false};

  // Helpers
  void reclaim_completed();
  void submit_batch(BatchEntry &batch);
  void seal_and_submit_current();

  size_t batch_index(size_t pos) const { return pos & (kMaxBatches - 1); }

  DsaFixedRingBatchBase(const DsaFixedRingBatchBase &) = delete;
  DsaFixedRingBatchBase &operator=(const DsaFixedRingBatchBase &) = delete;
};

// Type aliases paralleling the existing DsaRingBatchBase aliases
using DsaFixedRingBatch = DsaFixedRingBatchBase<dsa::MutexTaskQueue>;
using DsaFixedRingBatchSingleThread =
    DsaFixedRingBatchBase<dsa::SingleThreadTaskQueue>;
using DsaFixedRingBatchTasSpinlock =
    DsaFixedRingBatchBase<dsa::TasSpinlockTaskQueue>;
using DsaFixedRingBatchSpinlock =
    DsaFixedRingBatchBase<dsa::SpinlockTaskQueue>;
using DsaFixedRingBatchBackoffSpinlock =
    DsaFixedRingBatchBase<dsa::BackoffSpinlockTaskQueue>;
using DsaFixedRingBatchLockFree =
    DsaFixedRingBatchBase<dsa::LockFreeTaskQueue>;

#include "dsa_fixed_ring_batch.ipp"

#endif
