#pragma once
#ifndef DSA_DESCRIPTOR_SUBMITTER_HPP
#define DSA_DESCRIPTOR_SUBMITTER_HPP

#include <algorithm>
#include <concepts>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <functional>
#include <x86intrin.h>

#include "mirrored_ring.hpp"

extern "C" {
#include <accel-config/libaccel_config.h>
#include <linux/idxd.h>
}

// ============================================================================
// DescriptorSubmitter concept
// ============================================================================

template <typename T>
concept DescriptorSubmitter = requires(T &s, dsa_hw_desc *desc) {
  { s.submit_descriptor(desc) } -> std::same_as<void>;
  { s.flush() } -> std::same_as<void>;
  { s.pre_poll() } -> std::same_as<void>;
  { s.drain() } -> std::same_as<void>;  // shutdown: flush + wait for in-flight
};

// ============================================================================
// DirectSubmitter — immediate MMIO submission via _movdir64b / _enqcmd
// ============================================================================

class DirectSubmitter {
public:
  DirectSubmitter() = default;

  void init(void *portal, accfg_wq_mode mode, accfg_wq *) {
    portal_ = portal;
    mode_ = mode;
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
  }

  void flush() {}
  void pre_poll() {}
  void drain() {}

private:
  void *portal_ = nullptr;
  accfg_wq_mode mode_ = ACCFG_WQ_SHARED;
};

static_assert(DescriptorSubmitter<DirectSubmitter>);

// ============================================================================
// StagingSubmitter — double-buffered batch descriptor staging
// ============================================================================
// Stages descriptors in a double-buffered array and submits them as a hardware
// batch (opcode 0x01) on flush(), reducing MMIO doorbell writes.
// Uses DirectSubmitter internally for raw descriptor submission.

class StagingSubmitter {
public:
  StagingSubmitter() = default;

  void init(void *portal, accfg_wq_mode mode, accfg_wq *wq) {
    direct_.init(portal, mode, wq);

    int wq_max = accfg_wq_get_max_batch_size(wq);
    if (wq_max > 0) {
      max_batch_size_ =
          std::min(static_cast<size_t>(wq_max), kMaxStagingSize);
    }

    memset(staged_, 0, sizeof(staged_));
    memset(batch_comp_, 0, sizeof(batch_comp_));
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
    }

    if (staged_count_ == 1) {
      // Single descriptor — submit directly, no batch overhead
      direct_.submit_descriptor(&staged_[active_buf_][0]);
    } else {
      // Build batch descriptor (opcode 0x01)
      dsa_hw_desc batch{};
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

private:
  DirectSubmitter direct_;

  static constexpr size_t kMaxStagingSize = 32;
  alignas(64) dsa_hw_desc staged_[2][kMaxStagingSize];
  size_t staged_count_ = 0;
  int active_buf_ = 0;

  alignas(32) dsa_completion_record batch_comp_[2] = {};
  bool batch_submitted_[2] = {false, false};

  size_t max_batch_size_ = kMaxStagingSize;
};

static_assert(DescriptorSubmitter<StagingSubmitter>);

// ============================================================================
// FixedRingSubmitter — fixed-size ring of batch entries
// ============================================================================
// Uses a ring of fixed-size batch entries. Each entry owns a contiguous
// descriptor array of kBatchCapacity slots plus a batch completion record.
// Simpler than RingSubmitter (no wrap-around or dynamic range tracking)
// but wastes descriptor space when batches are small.

class FixedRingSubmitter {
public:
  FixedRingSubmitter() = default;

  void init(void *portal, accfg_wq_mode mode, accfg_wq *wq) {
    direct_.init(portal, mode, wq);

    int wq_max = accfg_wq_get_max_batch_size(wq);
    if (wq_max > 0) {
      max_batch_size_ =
          std::min(static_cast<size_t>(wq_max), kBatchCapacity);
    }

    for (auto &b : batches_) {
      memset(b.descs, 0, sizeof(b.descs));
      memset(&b.comp, 0, sizeof(b.comp));
      b.count = 0;
      b.state = BatchState::Free;
    }
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
    for (auto &b : batches_) {
      while (b.state == BatchState::InFlight) {
        _mm_pause();
        reclaim_completed();
      }
    }
  }

private:
  DirectSubmitter direct_;

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
  size_t batch_fill_ = 0;
  size_t batch_head_ = 0;

  size_t max_batch_size_ = kBatchCapacity;

  size_t batch_index(size_t pos) const { return pos & (kMaxBatches - 1); }

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
      batch_head_++;
    }
  }

  void submit_batch(BatchEntry &batch) {
    if (batch.count == 1) {
      direct_.submit_descriptor(&batch.descs[0]);
      batch.state = BatchState::Free;
    } else {
      dsa_hw_desc bd{};
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

class RingSubmitter {
public:
  RingSubmitter() = default;

  void init(void *portal, accfg_wq_mode mode, accfg_wq *wq) {
    direct_.init(portal, mode, wq);

    int wq_max = accfg_wq_get_max_batch_size(wq);
    if (wq_max > 0) {
      max_batch_size_ =
          std::min(static_cast<size_t>(wq_max), static_cast<size_t>(32));
    }

    memset(desc_ring_, 0, sizeof(desc_ring_));
    for (auto &b : batches_) {
      memset(&b.batch_comp, 0, sizeof(b.batch_comp));
      b.start = 0;
      b.count = 0;
      b.state = BatchState::Free;
    }
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
    for (auto &b : batches_) {
      while (b.state == BatchState::InFlight) {
        _mm_pause();
        reclaim_completed();
      }
    }
  }

private:
  DirectSubmitter direct_;
  std::function<void()> poll_fn_;

  // Descriptor ring
  static constexpr size_t kDescRingSize = 256;
  static_assert((kDescRingSize & (kDescRingSize - 1)) == 0,
                "kDescRingSize must be power of 2");
  alignas(64) dsa_hw_desc desc_ring_[kDescRingSize];
  size_t desc_head_ = 0;
  size_t desc_tail_ = 0;

  // Batch metadata ring
  static constexpr size_t kMaxBatches = 16;
  static_assert((kMaxBatches & (kMaxBatches - 1)) == 0,
                "kMaxBatches must be power of 2");

  enum class BatchState : uint8_t { Free, Filling, InFlight };

  struct BatchEntry {
    alignas(32) dsa_completion_record batch_comp;
    size_t start;
    uint32_t count;
    BatchState state;
  };

  BatchEntry batches_[kMaxBatches];
  size_t batch_fill_ = 0;
  size_t batch_head_ = 0;

  size_t max_batch_size_ = 32;

  size_t desc_index(size_t pos) const { return pos & (kDescRingSize - 1); }
  size_t batch_index(size_t pos) const { return pos & (kMaxBatches - 1); }
  size_t desc_available() const { return kDescRingSize - (desc_tail_ - desc_head_); }

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
      batch_head_++;
    }
  }

  void submit_batch(BatchEntry &batch) {
    if (batch.count == 1) {
      direct_.submit_descriptor(&desc_ring_[batch.start]);
      batch.batch_comp.status = 1;
      batch.state = BatchState::InFlight;
    } else {
      dsa_hw_desc bd{};
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

class MirroredRingSubmitter {
public:
  MirroredRingSubmitter() = default;

  void init(void *portal, accfg_wq_mode mode, accfg_wq *wq) {
    direct_.init(portal, mode, wq);

    int wq_max = accfg_wq_get_max_batch_size(wq);
    if (wq_max > 0) {
      max_batch_size_ =
          std::min(static_cast<size_t>(wq_max), static_cast<size_t>(32));
    }

    desc_ring_ = static_cast<dsa_hw_desc *>(ring_.data());

    for (auto &b : batches_) {
      memset(&b.batch_comp, 0, sizeof(b.batch_comp));
      b.start = 0;
      b.count = 0;
      b.state = BatchState::Free;
    }
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
    for (auto &b : batches_) {
      while (b.state == BatchState::InFlight) {
        _mm_pause();
        reclaim_completed();
      }
    }
  }

private:
  DirectSubmitter direct_;
  std::function<void()> poll_fn_;

  // Descriptor ring — mirrored virtual memory mapping
  static constexpr size_t kDescRingSize = 256;
  static_assert((kDescRingSize & (kDescRingSize - 1)) == 0,
                "kDescRingSize must be power of 2");
  MirroredRing ring_{kDescRingSize, sizeof(dsa_hw_desc)};
  dsa_hw_desc *desc_ring_ = nullptr;
  size_t desc_head_ = 0;
  size_t desc_tail_ = 0;

  // Batch metadata ring
  static constexpr size_t kMaxBatches = 16;
  static_assert((kMaxBatches & (kMaxBatches - 1)) == 0,
                "kMaxBatches must be power of 2");

  enum class BatchState : uint8_t { Free, Filling, InFlight };

  struct BatchEntry {
    alignas(32) dsa_completion_record batch_comp;
    size_t start;
    uint32_t count;
    BatchState state;
  };

  BatchEntry batches_[kMaxBatches];
  size_t batch_fill_ = 0;
  size_t batch_head_ = 0;

  size_t max_batch_size_ = 32;

  size_t desc_index(size_t pos) const { return pos & (kDescRingSize - 1); }
  size_t batch_index(size_t pos) const { return pos & (kMaxBatches - 1); }
  size_t desc_available() const { return kDescRingSize - (desc_tail_ - desc_head_); }

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
      batch_head_++;
    }
  }

  void submit_batch(BatchEntry &batch) {
    // batch.start is already a masked index in [0, kDescRingSize).
    // The mirror region guarantees contiguous access even if the batch
    // spans the ring boundary (e.g. start=250, count=32 → slots 250..281
    // are contiguous in virtual memory, wrapping to physical slots 0..25).
    if (batch.count == 1) {
      direct_.submit_descriptor(&desc_ring_[batch.start]);
      batch.batch_comp.status = 1;
      batch.state = BatchState::InFlight;
    } else {
      dsa_hw_desc bd{};
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
