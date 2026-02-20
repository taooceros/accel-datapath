#pragma once
#ifndef DSA_DESCRIPTOR_SUBMITTER_HPP
#define DSA_DESCRIPTOR_SUBMITTER_HPP

#include <algorithm>
#include <concepts>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <functional>
#include <memory>
#include <optional>
#include <x86intrin.h>

#include "mirrored_ring.hpp"

extern "C" {
#include <accel-config/libaccel_config.h>
#include <linux/idxd.h>
}

// ============================================================================
// Utilities
// ============================================================================

// Custom deleter for aligned_alloc'd memory (used with std::unique_ptr)
struct AlignedDeleter {
  void operator()(void *p) const noexcept { std::free(p); }
};

template <typename T>
using AlignedArray = std::unique_ptr<T[], AlignedDeleter>;

// Allocate a zero-initialized, aligned array of T.
// alignment must be a power of 2 and >= sizeof(void*).
// Returned memory satisfies: base % alignment == 0.
template <typename T>
AlignedArray<T> alloc_aligned(size_t count, size_t alignment) {
  size_t bytes = count * sizeof(T);
  // std::aligned_alloc requires bytes to be a multiple of alignment
  bytes = (bytes + alignment - 1) & ~(alignment - 1);
  void *p = std::aligned_alloc(alignment, bytes);
  if (!p)
    throw std::bad_alloc();
  std::memset(p, 0, bytes);
  return AlignedArray<T>(static_cast<T *>(p));
}

// Round up to the next power of 2 (returns n if already a power of 2).
inline size_t round_up_pow2(size_t n) {
  if (n <= 1)
    return 1;
  n--;
  n |= n >> 1;
  n |= n >> 2;
  n |= n >> 4;
  n |= n >> 8;
  n |= n >> 16;
  n |= n >> 32;
  return n + 1;
}

static constexpr size_t kDefaultBatchSize = 32;
static constexpr size_t kDefaultNumBatches = 16;

// ============================================================================
// DescriptorSubmitter concept
// ============================================================================

template <typename T>
concept DescriptorSubmitter = requires(T &s, dsa_hw_desc *desc, size_t n) {
  { s.submit_descriptor(desc) } -> std::same_as<void>;
  { s.flush() } -> std::same_as<void>;
  { s.pre_poll() } -> std::same_as<void>;
  { s.drain() } -> std::same_as<void>;  // shutdown: flush + wait for in-flight
  { s.wq_capacity() } -> std::same_as<size_t>;   // WQ depth (0 = no gating)
  { s.inflight() } -> std::same_as<size_t>;       // current in-flight doorbells
  { s.notify_complete(n) } -> std::same_as<void>; // decrement inflight by n
};

// ============================================================================
// DirectSubmitter — immediate MMIO submission via _movdir64b / _enqcmd
// ============================================================================

class DirectSubmitter {
public:
  DirectSubmitter() = default;

  void init(void *portal, accfg_wq_mode mode, accfg_wq *wq, size_t = 0) {
    portal_ = portal;
    mode_ = mode;
    // Query WQ depth for dedicated mode backpressure
    if (mode == ACCFG_WQ_DEDICATED && wq != nullptr) {
      int sz = accfg_wq_get_size(wq);
      wq_depth_ = sz > 0 ? static_cast<size_t>(sz) : 0;
    } else {
      wq_depth_ = 0;  // shared WQ: _enqcmd has natural backpressure
    }
    inflight_ = 0;
  }

  void submit_descriptor(dsa_hw_desc *desc) {
    _mm_sfence();
    if (mode_ == ACCFG_WQ_DEDICATED) {
      _movdir64b(portal_, desc);
    } else {
      while (_enqcmd(portal_, desc) != 0) {
        _mm_pause();
      }
    }
    ++inflight_;
  }

  void flush() {}
  void pre_poll() {}
  void drain() {}

  size_t wq_capacity() { return wq_depth_; }
  size_t inflight() { return inflight_; }
  void notify_complete(size_t n) {
    inflight_ = n <= inflight_ ? inflight_ - n : 0;  // saturating subtract
  }

private:
  void *portal_ = nullptr;
  accfg_wq_mode mode_ = ACCFG_WQ_SHARED;
  size_t wq_depth_ = 0;    // dedicated WQ size (0 = no gating)
  size_t inflight_ = 0;    // current in-flight doorbells
};

static_assert(DescriptorSubmitter<DirectSubmitter>);

// ============================================================================
// StagingSubmitter — double-buffered batch descriptor staging
// ============================================================================
// Stages descriptors in a double-buffered array and submits them as a hardware
// batch (opcode 0x01) on flush(), reducing MMIO doorbell writes.
// Uses DirectSubmitter internally for raw descriptor submission.
// Buffer sizes are determined at init() time via batch_size parameter.

class StagingSubmitter {
public:
  StagingSubmitter() = default;

  void init(void *portal, accfg_wq_mode mode, accfg_wq *wq,
            size_t batch_size = 0) {
    direct_.init(portal, mode, wq);

    // Determine capacity
    capacity_ = batch_size > 0 ? batch_size : kDefaultBatchSize;
    int wq_max = accfg_wq_get_max_batch_size(wq);
    if (wq_max > 0) {
      capacity_ = std::min(capacity_, static_cast<size_t>(wq_max));
    }
    max_batch_size_ = capacity_;

    // Allocate aligned buffers
    staged_[0] = alloc_aligned<dsa_hw_desc>(capacity_, 64);
    staged_[1] = alloc_aligned<dsa_hw_desc>(capacity_, 64);
    batch_comp_ = alloc_aligned<dsa_completion_record>(2, 32);

    staged_count_ = 0;
    active_buf_ = 0;
    batch_submitted_[0] = false;
    batch_submitted_[1] = false;
  }

  void submit_descriptor(dsa_hw_desc *desc) {
    memcpy(&staged_[active_buf_][staged_count_], desc, sizeof(dsa_hw_desc));
    staged_count_++;

    if (staged_count_ >= max_batch_size_) {
      flush();
    }
  }

  void flush() {
    if (staged_count_ == 0) {
      return;
    }

    // Wait for previous batch's descriptor array to be released by hardware.
    // Hardware DMA-reads the array asynchronously after _movdir64b.
    int prev = active_buf_ ^ 1;
    if (batch_submitted_[prev]) {
      while (batch_comp_[prev].status == 0) {
        _mm_pause();
      }
      batch_submitted_[prev] = false;
      direct_.notify_complete(1);  // previous batch doorbell completed
    }

    if (staged_count_ == 1) {
      // Single descriptor — submit directly, no batch overhead
      direct_.submit_descriptor(&staged_[active_buf_][0]);
    } else {
      // Build batch descriptor (opcode 0x01) — must be 64-byte aligned
      alignas(64) dsa_hw_desc batch{};
      memset(&batch, 0, sizeof(batch));
      batch.opcode = DSA_OPCODE_BATCH;
      batch.flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV;
      batch.desc_list_addr =
          reinterpret_cast<uint64_t>(&staged_[active_buf_][0]);
      batch.desc_count = static_cast<uint32_t>(staged_count_);
      batch.completion_addr =
          reinterpret_cast<uint64_t>(&batch_comp_[active_buf_]);

      memset(&batch_comp_[active_buf_], 0, sizeof(dsa_completion_record));

      direct_.submit_descriptor(&batch);
      batch_submitted_[active_buf_] = true;
    }

    // Swap to the other buffer for next batch
    active_buf_ ^= 1;
    staged_count_ = 0;
  }

  void pre_poll() { flush(); }
  void drain() { flush(); }

  size_t wq_capacity() { return direct_.wq_capacity(); }
  size_t inflight() { return direct_.inflight(); }
  void notify_complete(size_t) {} // no-op: batch submitters decrement in reclaim

private:
  DirectSubmitter direct_;

  AlignedArray<dsa_hw_desc> staged_[2];  // each holds capacity_ descriptors
  AlignedArray<dsa_completion_record> batch_comp_;  // 2 elements
  size_t staged_count_ = 0;
  int active_buf_ = 0;
  bool batch_submitted_[2] = {false, false};

  size_t capacity_ = 0;         // allocated buffer size per staging buffer
  size_t max_batch_size_ = 0;   // flush threshold
};

static_assert(DescriptorSubmitter<StagingSubmitter>);

// ============================================================================
// FixedRingSubmitter — fixed-size ring of batch entries
// ============================================================================
// Uses a ring of fixed-size batch entries. Each entry owns a contiguous
// descriptor array plus a batch completion record.
// Simpler than RingSubmitter (no wrap-around or dynamic range tracking)
// but wastes descriptor space when batches are small.
// Buffer sizes are determined at init() time via batch_size parameter.

class FixedRingSubmitter {
public:
  FixedRingSubmitter() = default;

  void init(void *portal, accfg_wq_mode mode, accfg_wq *wq,
            size_t batch_size = 0) {
    direct_.init(portal, mode, wq);

    // Determine capacity
    batch_capacity_ = batch_size > 0 ? batch_size : kDefaultBatchSize;
    int wq_max = accfg_wq_get_max_batch_size(wq);
    if (wq_max > 0) {
      batch_capacity_ = std::min(batch_capacity_, static_cast<size_t>(wq_max));
    }
    max_batch_size_ = batch_capacity_;
    num_batches_ = kDefaultNumBatches;

    // Allocate batch entry ring
    batches_ = std::make_unique<BatchEntry[]>(num_batches_);
    for (size_t i = 0; i < num_batches_; ++i) {
      batches_[i].descs = alloc_aligned<dsa_hw_desc>(batch_capacity_, 64);
      memset(&batches_[i].comp, 0, sizeof(batches_[i].comp));
      batches_[i].count = 0;
      batches_[i].state = BatchState::Free;
    }

    batch_fill_ = 0;
    batch_head_ = 0;
  }

  void submit_descriptor(dsa_hw_desc *desc) {
    BatchEntry &batch = batches_[batch_index(batch_fill_)];

    // Wait for slot if current batch is still InFlight
    while (batch.state == BatchState::InFlight) {
      reclaim_completed();
      _mm_pause();
    }

    // Start a new batch if current slot is Free
    if (batch.state == BatchState::Free) {
      batch.state = BatchState::Filling;
      batch.count = 0;
      memset(&batch.comp, 0, sizeof(batch.comp));
    }

    memcpy(&batch.descs[batch.count], desc, sizeof(dsa_hw_desc));
    batch.count++;

    if (batch.count >= max_batch_size_) {
      seal_and_submit_current();
    }
  }

  void flush() {}

  void pre_poll() {
    // Drain any partial filling batch so descriptors get submitted
    BatchEntry &current = batches_[batch_index(batch_fill_)];
    if (current.state == BatchState::Filling && current.count > 0) {
      seal_and_submit_current();
    }
    reclaim_completed();
  }

  void drain() {
    // Seal partial batch
    BatchEntry &current = batches_[batch_index(batch_fill_)];
    if (current.state == BatchState::Filling && current.count > 0) {
      seal_and_submit_current();
    }
    // Wait for all InFlight batches to complete
    for (size_t i = 0; i < num_batches_; ++i) {
      while (batches_[i].state == BatchState::InFlight) {
        _mm_pause();
        reclaim_completed();
      }
    }
  }

  size_t wq_capacity() { return direct_.wq_capacity(); }
  size_t inflight() { return direct_.inflight(); }
  void notify_complete(size_t) {} // no-op: batch submitters decrement in reclaim

private:
  DirectSubmitter direct_;

  enum class BatchState : uint8_t { Free, Filling, InFlight };

  struct BatchEntry {
    AlignedArray<dsa_hw_desc> descs;          // batch_capacity_ descriptors (64-byte aligned)
    alignas(32) dsa_completion_record comp;
    uint32_t count = 0;
    BatchState state = BatchState::Free;
  };

  std::unique_ptr<BatchEntry[]> batches_;
  size_t batch_fill_ = 0;
  size_t batch_head_ = 0;

  size_t batch_capacity_ = 0;   // descriptors per batch slot
  size_t num_batches_ = 0;      // number of batch slots (power of 2)
  size_t max_batch_size_ = 0;   // flush threshold

  size_t batch_index(size_t pos) const { return pos & (num_batches_ - 1); }

  void reclaim_completed() {
    while (batch_head_ != batch_fill_) {
      BatchEntry &b = batches_[batch_index(batch_head_)];
      if (b.state == BatchState::Free) {
        batch_head_++;
        continue;
      }
      if (b.state != BatchState::InFlight) {
        break;
      }
      if (b.comp.status == 0) {
        break;
      }
      b.state = BatchState::Free;
      direct_.notify_complete(1);  // batch doorbell completed
      batch_head_++;
    }
  }

  void submit_batch(BatchEntry &batch) {
    if (batch.count == 1) {
      direct_.submit_descriptor(&batch.descs[0]);
      direct_.notify_complete(1);  // single-desc: doorbell consumed immediately
      batch.state = BatchState::Free;
    } else {
      // Batch descriptor must be 64-byte aligned for _enqcmd
      alignas(64) dsa_hw_desc bd{};
      memset(&bd, 0, sizeof(bd));
      bd.opcode = DSA_OPCODE_BATCH;
      bd.flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV;
      bd.desc_list_addr = reinterpret_cast<uint64_t>(&batch.descs[0]);
      bd.desc_count = static_cast<uint32_t>(batch.count);
      bd.completion_addr = reinterpret_cast<uint64_t>(&batch.comp);

      direct_.submit_descriptor(&bd);
      batch.state = BatchState::InFlight;
    }
  }

  void seal_and_submit_current() {
    BatchEntry &batch = batches_[batch_index(batch_fill_)];
    if (batch.state != BatchState::Filling || batch.count == 0) {
      return;
    }

    submit_batch(batch);
    batch_fill_++;

    if (batches_[batch_index(batch_fill_)].state != BatchState::Free) {
      reclaim_completed();
    }
  }
};

static_assert(DescriptorSubmitter<FixedRingSubmitter>);

// ============================================================================
// RingSubmitter — shared descriptor ring with dynamic range allocation
// ============================================================================
// Uses two separate ring buffers:
//   - Descriptor ring: large contiguous array of dsa_hw_desc slots
//   - Batch ring: small metadata array, each entry references a range in the
//     descriptor ring and carries its own batch completion record
//
// More memory-efficient than FixedRingSubmitter for variable-size batches,
// but requires wrap-around handling.
// Buffer sizes are determined at init() time via batch_size parameter.

class RingSubmitter {
public:
  RingSubmitter() = default;

  void init(void *portal, accfg_wq_mode mode, accfg_wq *wq,
            size_t batch_size = 0) {
    direct_.init(portal, mode, wq);

    // Determine batch size
    max_batch_size_ = batch_size > 0 ? batch_size : kDefaultBatchSize;
    int wq_max = accfg_wq_get_max_batch_size(wq);
    if (wq_max > 0) {
      max_batch_size_ = std::min(max_batch_size_, static_cast<size_t>(wq_max));
    }
    num_batches_ = kDefaultNumBatches;

    // Descriptor ring: must hold all batches at full capacity, power of 2
    desc_ring_size_ = round_up_pow2(max_batch_size_ * num_batches_);
    desc_ring_ = alloc_aligned<dsa_hw_desc>(desc_ring_size_, 64);

    // Batch metadata ring
    batches_ = std::make_unique<BatchEntry[]>(num_batches_);
    for (size_t i = 0; i < num_batches_; ++i) {
      memset(&batches_[i].batch_comp, 0, sizeof(batches_[i].batch_comp));
      batches_[i].start = 0;
      batches_[i].count = 0;
      batches_[i].state = BatchState::Free;
    }

    desc_head_ = 0;
    desc_tail_ = 0;
    batch_fill_ = 0;
    batch_head_ = 0;
  }

  void set_poll_fn(std::function<void()> fn) { poll_fn_ = std::move(fn); }

  void submit_descriptor(dsa_hw_desc *desc) {
    // Ensure descriptor ring has space
    while (desc_available() == 0) {
      reclaim_completed();
      if (poll_fn_) poll_fn_();
      _mm_pause();
    }

    // Ensure the current batch slot is available
    while (batches_[batch_index(batch_fill_)].state == BatchState::InFlight) {
      reclaim_completed();
      if (poll_fn_) poll_fn_();
      _mm_pause();
    }

    BatchEntry &batch = batches_[batch_index(batch_fill_)];

    if (batch.state == BatchState::Free) {
      batch.state = BatchState::Filling;
      batch.start = desc_index(desc_tail_);
      batch.count = 0;
      memset(&batch.batch_comp, 0, sizeof(batch.batch_comp));
    }

    // Wrap-around check: if the next descriptor would cross the ring
    // boundary, seal the current batch and start a new one at index 0.
    size_t next_idx = desc_index(desc_tail_);
    if (batch.count > 0 && next_idx == 0) {
      seal_and_submit_current();

      while (batches_[batch_index(batch_fill_)].state == BatchState::InFlight) {
        reclaim_completed();
        if (poll_fn_) poll_fn_();
        _mm_pause();
      }

      BatchEntry &new_batch = batches_[batch_index(batch_fill_)];
      new_batch.state = BatchState::Filling;
      new_batch.start = 0;
      new_batch.count = 0;
      memset(&new_batch.batch_comp, 0, sizeof(new_batch.batch_comp));
    }

    BatchEntry &active = batches_[batch_index(batch_fill_)];
    memcpy(&desc_ring_[desc_index(desc_tail_)], desc, sizeof(dsa_hw_desc));
    desc_tail_++;
    active.count++;

    if (active.count >= max_batch_size_) {
      seal_and_submit_current();
    }
  }

  void flush() {}

  void pre_poll() {
    BatchEntry &current = batches_[batch_index(batch_fill_)];
    if (current.state == BatchState::Filling && current.count > 0) {
      seal_and_submit_current();
    }
    reclaim_completed();
  }

  void drain() {
    BatchEntry &current = batches_[batch_index(batch_fill_)];
    if (current.state == BatchState::Filling && current.count > 0) {
      seal_and_submit_current();
    }
    for (size_t i = 0; i < num_batches_; ++i) {
      while (batches_[i].state == BatchState::InFlight) {
        _mm_pause();
        reclaim_completed();
      }
    }
  }

  size_t wq_capacity() { return direct_.wq_capacity(); }
  size_t inflight() { return direct_.inflight(); }
  void notify_complete(size_t) {} // no-op: batch submitters decrement in reclaim

private:
  DirectSubmitter direct_;
  std::function<void()> poll_fn_;

  // Descriptor ring (64-byte aligned, power-of-2 size)
  AlignedArray<dsa_hw_desc> desc_ring_;
  size_t desc_ring_size_ = 0;   // power of 2
  size_t desc_head_ = 0;
  size_t desc_tail_ = 0;

  // Batch metadata ring
  enum class BatchState : uint8_t { Free, Filling, InFlight };

  struct BatchEntry {
    alignas(32) dsa_completion_record batch_comp;
    size_t start;
    uint32_t count;
    BatchState state;
  };

  std::unique_ptr<BatchEntry[]> batches_;
  size_t num_batches_ = 0;      // power of 2
  size_t batch_fill_ = 0;
  size_t batch_head_ = 0;

  size_t max_batch_size_ = 0;   // flush threshold

  size_t desc_index(size_t pos) const { return pos & (desc_ring_size_ - 1); }
  size_t batch_index(size_t pos) const { return pos & (num_batches_ - 1); }
  size_t desc_available() const { return desc_ring_size_ - (desc_tail_ - desc_head_); }

  void reclaim_completed() {
    while (batch_head_ != batch_fill_) {
      BatchEntry &b = batches_[batch_index(batch_head_)];
      if (b.state == BatchState::Free) {
        batch_head_++;
        continue;
      }
      if (b.state != BatchState::InFlight) {
        break;
      }
      if (b.batch_comp.status == 0) {
        break;
      }
      desc_head_ += b.count;
      b.state = BatchState::Free;
      direct_.notify_complete(1);  // batch doorbell completed
      batch_head_++;
    }
  }

  void submit_batch(BatchEntry &batch) {
    if (batch.count == 1) {
      direct_.submit_descriptor(&desc_ring_[batch.start]);
      direct_.notify_complete(1);  // single-desc: doorbell consumed immediately
      batch.batch_comp.status = 1;
      batch.state = BatchState::InFlight;
    } else {
      // Batch descriptor must be 64-byte aligned for _enqcmd
      alignas(64) dsa_hw_desc bd{};
      memset(&bd, 0, sizeof(bd));
      bd.opcode = DSA_OPCODE_BATCH;
      bd.flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV;
      bd.desc_list_addr = reinterpret_cast<uint64_t>(&desc_ring_[batch.start]);
      bd.desc_count = static_cast<uint32_t>(batch.count);
      bd.completion_addr = reinterpret_cast<uint64_t>(&batch.batch_comp);

      direct_.submit_descriptor(&bd);
      batch.state = BatchState::InFlight;
    }
  }

  void seal_and_submit_current() {
    BatchEntry &batch = batches_[batch_index(batch_fill_)];
    if (batch.state != BatchState::Filling || batch.count == 0) {
      return;
    }

    submit_batch(batch);
    batch_fill_++;

    if (batches_[batch_index(batch_fill_)].state != BatchState::Free) {
      reclaim_completed();
    }
  }
};

static_assert(DescriptorSubmitter<RingSubmitter>);

// ============================================================================
// MirroredRingSubmitter — wrap-free ring via virtual memory mirroring
// ============================================================================
// Same architecture as RingSubmitter (shared descriptor ring + batch metadata
// ring) but the descriptor ring is backed by a MirroredRing: the same physical
// pages are mapped twice contiguously in virtual memory. This eliminates the
// wrap-around check entirely — a batch that starts near the end of the ring
// and extends past it sees contiguous memory via the mirror region.
// Buffer sizes are determined at init() time via batch_size parameter.

class MirroredRingSubmitter {
public:
  MirroredRingSubmitter() = default;

  void init(void *portal, accfg_wq_mode mode, accfg_wq *wq,
            size_t batch_size = 0) {
    direct_.init(portal, mode, wq);

    // Determine batch size
    max_batch_size_ = batch_size > 0 ? batch_size : kDefaultBatchSize;
    int wq_max = accfg_wq_get_max_batch_size(wq);
    if (wq_max > 0) {
      max_batch_size_ = std::min(max_batch_size_, static_cast<size_t>(wq_max));
    }
    num_batches_ = kDefaultNumBatches;

    // Descriptor ring: must hold all batches at full capacity, power of 2
    desc_ring_size_ = round_up_pow2(max_batch_size_ * num_batches_);
    ring_.emplace(desc_ring_size_, sizeof(dsa_hw_desc));
    desc_ring_ = static_cast<dsa_hw_desc *>(ring_->data());

    // Batch metadata ring
    batches_ = std::make_unique<BatchEntry[]>(num_batches_);
    for (size_t i = 0; i < num_batches_; ++i) {
      memset(&batches_[i].batch_comp, 0, sizeof(batches_[i].batch_comp));
      batches_[i].start = 0;
      batches_[i].count = 0;
      batches_[i].state = BatchState::Free;
    }

    desc_head_ = 0;
    desc_tail_ = 0;
    batch_fill_ = 0;
    batch_head_ = 0;
  }

  void set_poll_fn(std::function<void()> fn) { poll_fn_ = std::move(fn); }

  void submit_descriptor(dsa_hw_desc *desc) {
    // Ensure descriptor ring has space
    while (desc_available() == 0) {
      reclaim_completed();
      if (poll_fn_) poll_fn_();
      _mm_pause();
    }

    // Ensure the current batch slot is available
    while (batches_[batch_index(batch_fill_)].state == BatchState::InFlight) {
      reclaim_completed();
      if (poll_fn_) poll_fn_();
      _mm_pause();
    }

    BatchEntry &batch = batches_[batch_index(batch_fill_)];

    if (batch.state == BatchState::Free) {
      batch.state = BatchState::Filling;
      batch.start = desc_index(desc_tail_);
      batch.count = 0;
      memset(&batch.batch_comp, 0, sizeof(batch.batch_comp));
    }

    // No wrap-around check needed — mirror region makes ring contiguous
    memcpy(&desc_ring_[desc_index(desc_tail_)], desc, sizeof(dsa_hw_desc));
    desc_tail_++;
    batch.count++;

    if (batch.count >= max_batch_size_) {
      seal_and_submit_current();
    }
  }

  void flush() {}

  void pre_poll() {
    BatchEntry &current = batches_[batch_index(batch_fill_)];
    if (current.state == BatchState::Filling && current.count > 0) {
      seal_and_submit_current();
    }
    reclaim_completed();
  }

  void drain() {
    BatchEntry &current = batches_[batch_index(batch_fill_)];
    if (current.state == BatchState::Filling && current.count > 0) {
      seal_and_submit_current();
    }
    for (size_t i = 0; i < num_batches_; ++i) {
      while (batches_[i].state == BatchState::InFlight) {
        _mm_pause();
        reclaim_completed();
      }
    }
  }

  size_t wq_capacity() { return direct_.wq_capacity(); }
  size_t inflight() { return direct_.inflight(); }
  void notify_complete(size_t) {} // no-op: batch submitters decrement in reclaim

private:
  DirectSubmitter direct_;
  std::function<void()> poll_fn_;

  // Descriptor ring — mirrored virtual memory mapping (page-aligned by mmap)
  std::optional<MirroredRing> ring_;
  dsa_hw_desc *desc_ring_ = nullptr;
  size_t desc_ring_size_ = 0;   // power of 2
  size_t desc_head_ = 0;
  size_t desc_tail_ = 0;

  // Batch metadata ring
  enum class BatchState : uint8_t { Free, Filling, InFlight };

  struct BatchEntry {
    alignas(32) dsa_completion_record batch_comp;
    size_t start;
    uint32_t count;
    BatchState state;
  };

  std::unique_ptr<BatchEntry[]> batches_;
  size_t num_batches_ = 0;      // power of 2
  size_t batch_fill_ = 0;
  size_t batch_head_ = 0;

  size_t max_batch_size_ = 0;   // flush threshold

  size_t desc_index(size_t pos) const { return pos & (desc_ring_size_ - 1); }
  size_t batch_index(size_t pos) const { return pos & (num_batches_ - 1); }
  size_t desc_available() const { return desc_ring_size_ - (desc_tail_ - desc_head_); }

  void reclaim_completed() {
    while (batch_head_ != batch_fill_) {
      BatchEntry &b = batches_[batch_index(batch_head_)];
      if (b.state == BatchState::Free) {
        batch_head_++;
        continue;
      }
      if (b.state != BatchState::InFlight) {
        break;
      }
      if (b.batch_comp.status == 0) {
        break;
      }
      desc_head_ += b.count;
      b.state = BatchState::Free;
      direct_.notify_complete(1);  // batch doorbell completed
      batch_head_++;
    }
  }

  void submit_batch(BatchEntry &batch) {
    // batch.start is already a masked index in [0, desc_ring_size_).
    // The mirror region guarantees contiguous access even if the batch
    // spans the ring boundary (e.g. start=250, count=32 → slots 250..281
    // are contiguous in virtual memory, wrapping to physical slots 0..25).
    if (batch.count == 1) {
      direct_.submit_descriptor(&desc_ring_[batch.start]);
      direct_.notify_complete(1);  // single-desc: doorbell consumed immediately
      batch.batch_comp.status = 1;
      batch.state = BatchState::InFlight;
    } else {
      // Batch descriptor must be 64-byte aligned for _enqcmd
      alignas(64) dsa_hw_desc bd{};
      memset(&bd, 0, sizeof(bd));
      bd.opcode = DSA_OPCODE_BATCH;
      bd.flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV;
      bd.desc_list_addr = reinterpret_cast<uint64_t>(&desc_ring_[batch.start]);
      bd.desc_count = static_cast<uint32_t>(batch.count);
      bd.completion_addr = reinterpret_cast<uint64_t>(&batch.batch_comp);

      direct_.submit_descriptor(&bd);
      batch.state = BatchState::InFlight;
    }
  }

  void seal_and_submit_current() {
    BatchEntry &batch = batches_[batch_index(batch_fill_)];
    if (batch.state != BatchState::Filling || batch.count == 0) {
      return;
    }

    submit_batch(batch);
    batch_fill_++;

    if (batches_[batch_index(batch_fill_)].state != BatchState::Free) {
      reclaim_completed();
    }
  }
};

static_assert(DescriptorSubmitter<MirroredRingSubmitter>);

#endif
