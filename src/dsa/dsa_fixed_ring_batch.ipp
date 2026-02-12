#pragma once
#ifndef DSA_FIXED_RING_BATCH_IMPL_IPP
#define DSA_FIXED_RING_BATCH_IMPL_IPP

#include "dsa_fixed_ring_batch.hpp"
#include <cstring>
#include <dsa_stdexec/trace.hpp>
#include <fmt/format.h>
#include <x86intrin.h>

extern "C" {
#include <accel-config/libaccel_config.h>
}

template <template <typename> class QueueTemplate>
DsaFixedRingBatchBase<QueueTemplate>::DsaFixedRingBatchBase(bool start_poller)
    : inner_(false) { // Never let inner start its own poller
  int wq_max = accfg_wq_get_max_batch_size(inner_.wq());
  if (wq_max > 0) {
    max_batch_size_ = std::min(static_cast<size_t>(wq_max),
                               kBatchCapacity);
  }

  for (auto &b : batches_) {
    memset(b.descs, 0, sizeof(b.descs));
    memset(&b.comp, 0, sizeof(b.comp));
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
DsaFixedRingBatchBase<QueueTemplate>::~DsaFixedRingBatchBase() {
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
void DsaFixedRingBatchBase<QueueTemplate>::submit(
    dsa_stdexec::OperationBase *op, dsa_hw_desc *desc) {
  TRACE_EVENT("dsa", "fixed_ring_batch_submit", "op", (uintptr_t)op);

  if (desc != nullptr) {
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

    // Auto-submit when batch reaches hardware limit
    if (batch.count >= max_batch_size_) {
      seal_and_submit_current();
    }
  }

  inner_.task_queue().push(op);
}

template <template <typename> class QueueTemplate>
void DsaFixedRingBatchBase<QueueTemplate>::submit(
    dsa_stdexec::OperationBase *op) {
  TRACE_EVENT("dsa", "fixed_ring_batch_submit_nodesc", "op", (uintptr_t)op);
  inner_.task_queue().push(op);
}

template <template <typename> class QueueTemplate>
void DsaFixedRingBatchBase<QueueTemplate>::poll() {
  // Drain any partial filling batch so descriptors get submitted
  BatchEntry &current = batches_[batch_index(batch_fill_)];
  if (current.state == BatchState::Filling && current.count > 0) {
    seal_and_submit_current();
  }

  reclaim_completed();
  inner_.poll();
}

template <template <typename> class QueueTemplate>
void DsaFixedRingBatchBase<QueueTemplate>::reclaim_completed() {
  // Walk from oldest batch forward, reclaim completed InFlight batches
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
      break; // not yet completed
    }
    b.state = BatchState::Free;
    batch_head_++;
  }
}

template <template <typename> class QueueTemplate>
void DsaFixedRingBatchBase<QueueTemplate>::submit_batch(BatchEntry &batch) {
  TRACE_EVENT("dsa", "fixed_ring_batch_submit_hw", "count", batch.count);

  if (batch.count == 1) {
    // Single descriptor — submit directly, no batch opcode overhead.
    inner_.submit_raw(&batch.descs[0]);
    batch.state = BatchState::Free;
  } else {
    // Build hardware batch descriptor (opcode 0x01)
    dsa_hw_desc bd{};
    memset(&bd, 0, sizeof(bd));
    bd.opcode = DSA_OPCODE_BATCH;
    bd.flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV;
    bd.desc_list_addr = reinterpret_cast<uint64_t>(&batch.descs[0]);
    bd.desc_count = static_cast<uint32_t>(batch.count);
    bd.completion_addr = reinterpret_cast<uint64_t>(&batch.comp);

    inner_.submit_raw(&bd);
    batch.state = BatchState::InFlight;
  }
}

template <template <typename> class QueueTemplate>
void DsaFixedRingBatchBase<QueueTemplate>::seal_and_submit_current() {
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
