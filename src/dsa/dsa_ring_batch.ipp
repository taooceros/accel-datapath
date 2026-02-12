#pragma once
#ifndef DSA_RING_BATCH_IMPL_IPP
#define DSA_RING_BATCH_IMPL_IPP

#include "dsa_ring_batch.hpp"
#include <algorithm>
#include <cstring>
#include <dsa_stdexec/trace.hpp>
#include <fmt/format.h>
#include <x86intrin.h>

extern "C" {
#include <accel-config/libaccel_config.h>
}

template <template <typename> class QueueTemplate>
DsaRingBatchBase<QueueTemplate>::DsaRingBatchBase(bool start_poller)
    : inner_(false) { // Never let inner start its own poller
  int wq_max = accfg_wq_get_max_batch_size(inner_.wq());
  if (wq_max > 0) {
    max_batch_size_ = std::min(static_cast<size_t>(wq_max),
                               static_cast<size_t>(32));
  }

  memset(desc_ring_, 0, sizeof(desc_ring_));
  for (auto &b : batches_) {
    memset(&b.batch_comp, 0, sizeof(b.batch_comp));
    b.start = 0;
    b.count = 0;
    b.state = BatchState::Free;
  }

  if (start_poller) {
    running_ = true;
    poller_ = std::thread([this] {
      while (running_) {
        poll();
      }
    });
  }
}

template <template <typename> class QueueTemplate>
DsaRingBatchBase<QueueTemplate>::~DsaRingBatchBase() {
  running_ = false;
  if (poller_.joinable()) {
    poller_.join();
  }
  // Drain any partial batch
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

template <template <typename> class QueueTemplate>
void DsaRingBatchBase<QueueTemplate>::submit(dsa_stdexec::OperationBase *op,
                                             dsa_hw_desc *desc) {
  TRACE_EVENT("dsa", "ring_batch_submit", "op", (uintptr_t)op);

  if (desc != nullptr) {
    // Ensure descriptor ring has space
    while (desc_available() == 0) {
      reclaim_completed();
      _mm_pause();
    }

    BatchEntry &batch = batches_[batch_index(batch_fill_)];

    // Start a new batch if current slot is Free
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

      // Ensure the new batch slot is available
      while (batches_[batch_index(batch_fill_)].state != BatchState::Free) {
        reclaim_completed();
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

    // Auto-submit when batch reaches hardware limit
    if (active.count >= max_batch_size_) {
      seal_and_submit_current();
    }
  }

  inner_.task_queue().push(op);
}

template <template <typename> class QueueTemplate>
void DsaRingBatchBase<QueueTemplate>::submit(dsa_stdexec::OperationBase *op) {
  TRACE_EVENT("dsa", "ring_batch_submit_nodesc", "op", (uintptr_t)op);
  inner_.task_queue().push(op);
}

template <template <typename> class QueueTemplate>
void DsaRingBatchBase<QueueTemplate>::poll() {
  // Drain any partial filling batch so descriptors get submitted
  BatchEntry &current = batches_[batch_index(batch_fill_)];
  if (current.state == BatchState::Filling && current.count > 0) {
    seal_and_submit_current();
  }

  reclaim_completed();
  inner_.poll();
}

template <template <typename> class QueueTemplate>
void DsaRingBatchBase<QueueTemplate>::reclaim_completed() {
  // Walk from oldest batch forward, reclaim completed InFlight batches
  // in order so desc_head_ advances correctly.
  while (batch_head_ != batch_fill_) {
    BatchEntry &b = batches_[batch_index(batch_head_)];
    if (b.state == BatchState::Free) {
      // Already reclaimed (e.g. single-descriptor batch freed inline)
      batch_head_++;
      continue;
    }
    if (b.state != BatchState::InFlight) {
      break;
    }
    if (b.batch_comp.status == 0) {
      break; // not yet completed
    }
    // Batch completed — free it and advance desc_head_
    desc_head_ += b.count;
    b.state = BatchState::Free;
    batch_head_++;
  }
}

template <template <typename> class QueueTemplate>
void DsaRingBatchBase<QueueTemplate>::submit_batch(BatchEntry &batch) {
  TRACE_EVENT("dsa", "ring_batch_submit_hw", "count", batch.count);

  if (batch.count == 1) {
    // Single descriptor — submit directly, no batch opcode overhead.
    // _movdir64b copies the 64-byte descriptor atomically into the portal,
    // so the source is free immediately — mark slot as Free.
    inner_.submit_raw(&desc_ring_[batch.start]);
    desc_head_ += 1;
    batch.state = BatchState::Free;
  } else {
    // Build hardware batch descriptor (opcode 0x01)
    dsa_hw_desc bd{};
    memset(&bd, 0, sizeof(bd));
    bd.opcode = DSA_OPCODE_BATCH;
    bd.flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV;
    bd.desc_list_addr = reinterpret_cast<uint64_t>(&desc_ring_[batch.start]);
    bd.desc_count = static_cast<uint32_t>(batch.count);
    bd.completion_addr = reinterpret_cast<uint64_t>(&batch.batch_comp);

    inner_.submit_raw(&bd);
    batch.state = BatchState::InFlight;
  }
}

template <template <typename> class QueueTemplate>
void DsaRingBatchBase<QueueTemplate>::seal_and_submit_current() {
  BatchEntry &batch = batches_[batch_index(batch_fill_)];
  if (batch.state != BatchState::Filling || batch.count == 0) {
    return;
  }

  submit_batch(batch);
  batch_fill_++;

  // If the next batch slot is not Free, try reclaiming
  if (batches_[batch_index(batch_fill_)].state != BatchState::Free) {
    reclaim_completed();
  }
}

#endif
