#pragma once
#ifndef DSA_BATCH_IMPL_IPP
#define DSA_BATCH_IMPL_IPP

#include "dsa_batch.hpp"
#include <algorithm>
#include <cstring>
#include <dsa_stdexec/trace.hpp>
#include <fmt/format.h>
#include <x86intrin.h>

extern "C" {
#include <accel-config/libaccel_config.h>
}

template <template <typename> class QueueTemplate>
DsaBatchBase<QueueTemplate>::DsaBatchBase(bool start_poller)
    : inner_(false) {  // Never let inner start its own poller
  // Query hardware max batch size from the work queue
  int wq_max = accfg_wq_get_max_batch_size(inner_.wq());
  if (wq_max > 0) {
    max_batch_size_ = std::min(static_cast<size_t>(wq_max), kMaxStagingSize);
  }

  memset(staged_, 0, sizeof(staged_));
  memset(batch_comp_, 0, sizeof(batch_comp_));

  // Start our own poller thread that does flush() + poll()
  if (start_poller) {
    running_ = true;
    poller_ = std::thread([this] {
      while (running_) {
        flush();
        poll();
      }
    });
  }
}

template <template <typename> class QueueTemplate>
DsaBatchBase<QueueTemplate>::~DsaBatchBase() {
  running_ = false;
  if (poller_.joinable()) {
    poller_.join();
  }
  // Flush any remaining staged descriptors before destruction
  flush();
}

template <template <typename> class QueueTemplate>
void DsaBatchBase<QueueTemplate>::submit(dsa_stdexec::OperationBase *op,
                                         dsa_hw_desc *desc) {
  TRACE_EVENT("dsa", "batch_submit", "op", (uintptr_t)op);

  if (desc != nullptr) {
    // Stage the descriptor instead of submitting immediately.
    // The memcpy preserves the original completion_addr pointing to the
    // operation's own comp_buffer_, so hardware writes status there.
    memcpy(&staged_[active_buf_][staged_count_], desc, sizeof(dsa_hw_desc));
    staged_count_++;

    // Auto-flush when staging buffer is full
    if (staged_count_ >= max_batch_size_) {
      flush();
    }
  }

  // Queue for completion polling (unchanged from DsaBase)
  inner_.task_queue().push(op);
}

template <template <typename> class QueueTemplate>
void DsaBatchBase<QueueTemplate>::submit(dsa_stdexec::OperationBase *op) {
  TRACE_EVENT("dsa", "batch_submit_nodesec", "op", (uintptr_t)op);
  inner_.task_queue().push(op);
}

template <template <typename> class QueueTemplate>
void DsaBatchBase<QueueTemplate>::flush() {
  if (staged_count_ == 0) {
    return;
  }

  TRACE_EVENT("dsa", "batch_flush", "count", staged_count_);

  // Wait for previous batch's descriptor array to be released by hardware.
  // Hardware DMA-reads the array asynchronously after _movdir64b.
  // The array is safe to reuse once batch_comp_[prev].status != 0.
  int prev = active_buf_ ^ 1;
  if (batch_submitted_[prev]) {
    while (batch_comp_[prev].status == 0) {
      _mm_pause();
    }
    batch_submitted_[prev] = false;
  }

  if (staged_count_ == 1) {
    // Single descriptor — submit directly, no batch overhead
    inner_.submit_raw(&staged_[active_buf_][0]);
  } else {
    // Build batch descriptor (opcode 0x01)
    dsa_hw_desc batch{};
    memset(&batch, 0, sizeof(batch));
    batch.opcode = DSA_OPCODE_BATCH;
    batch.flags = IDXD_OP_FLAG_RCR | IDXD_OP_FLAG_CRAV;
    batch.desc_list_addr = reinterpret_cast<uint64_t>(&staged_[active_buf_][0]);
    batch.desc_count = static_cast<uint32_t>(staged_count_);
    batch.completion_addr = reinterpret_cast<uint64_t>(&batch_comp_[active_buf_]);

    memset(&batch_comp_[active_buf_], 0, sizeof(dsa_completion_record));

    inner_.submit_raw(&batch);
    batch_submitted_[active_buf_] = true;
  }

  // Swap to the other buffer for next batch
  active_buf_ ^= 1;
  staged_count_ = 0;
}

template <template <typename> class QueueTemplate>
void DsaBatchBase<QueueTemplate>::poll() {
  inner_.poll();
}

#endif
