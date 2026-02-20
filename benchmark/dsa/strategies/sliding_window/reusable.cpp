// Level 2 optimization: Reusable operation slots.
// Bypasses stdexec connect/start entirely. Pre-allocates DsaOperationBase storage
// per slot and reuses it across operations. The hot path is:
//   memset desc/comp → fill_descriptor → set completion_addr → submit
//
// Per-op cost: ~8 ns (vs ~35 ns baseline, ~13 ns Level 1 direct connect).
//
// Trade-offs:
// - Skips stdexec page fault retry logic (DsaOperationMixin::notify checks comp status)
// - Completion handling is a simple callback, not stdexec::set_value
// - Appropriate for mock DSA benchmarking and known-good memory regions
// - All operation types supported via fill_for_op() dispatch

#include "strategy_common.hpp"
#include <cassert>
#include <cstddef>
#include <memory>

// ============================================================================
// ReusableSlot: pre-allocated operation storage with inline notify callback
// ============================================================================

// Each slot contains a DsaOperationBase for hardware-aligned descriptor/completion
// storage, plus metadata for the notify callback. The notify_fn pointer is set once
// during init and reused across all operations on this slot.
//
// Layout: op_base is the first member so that OperationBase* can be converted back
// to ReusableSlot* via offsetof (OperationBase is first base of DsaOperationBase).
struct ReusableSlot {
  dsa::DsaOperationBase op_base;
  ReusableSlot *next_free = nullptr;  // intrusive free-list link

  // Shared context (set once during arena init, stable across relaunches)
  std::atomic<size_t> *remaining = nullptr;
  LatencyCollector *latency = nullptr;
  DsaProxy *dsa = nullptr;

  // Per-op state (updated each relaunch)
  std::chrono::high_resolution_clock::time_point start_time;

  // Arena backpointer for slot release in notify callback
  BasicSlotArena<ReusableSlot> *arena = nullptr;

  void init(DsaProxy &d, std::atomic<size_t> *rem, LatencyCollector *lat,
            BasicSlotArena<ReusableSlot> *a) {
    dsa = &d;
    remaining = rem;
    latency = lat;
    arena = a;
    // Set function pointers once — same type every time, never changes
    op_base.notify_fn = &ReusableSlot::notify_impl;
    op_base.get_descriptor_fn = [](dsa_stdexec::OperationBase *base) -> dsa_hw_desc * {
      return static_cast<dsa::DsaOperationBase *>(base)->desc_ptr();
    };
  }

  // Hot path: fill descriptor and submit. No connect, no start, no receiver.
  template <class FillFn>
  void relaunch(FillFn &fill_fn, size_t offset) {
    if (latency->enabled()) {
      start_time = std::chrono::high_resolution_clock::now();
    }
    op_base.next = nullptr;

    auto *desc = op_base.desc_ptr();
    auto *comp = op_base.comp_ptr();
    std::memset(desc, 0, sizeof(dsa_hw_desc));
    std::memset(comp, 0, sizeof(dsa_completion_record));

    fill_fn(*desc, offset);
    desc->completion_addr = reinterpret_cast<uint64_t>(comp);

    dsa->submit(&op_base, desc);
  }

  // Notify callback: invoked by task queue poll when operation completes.
  // Recovers the ReusableSlot pointer from the OperationBase pointer.
  static void notify_impl(dsa_stdexec::OperationBase *base);

  ReusableSlot() = default;
  ReusableSlot(const ReusableSlot &) = delete;
  ReusableSlot &operator=(const ReusableSlot &) = delete;
  ReusableSlot(ReusableSlot &&) = delete;
  ReusableSlot &operator=(ReusableSlot &&) = delete;
};

// Defined after ReusableSlot so BasicSlotArena<ReusableSlot>::release() is visible.
void ReusableSlot::notify_impl(dsa_stdexec::OperationBase *base) {
  // OperationBase is the first base of DsaOperationBase, which is the first
  // member of ReusableSlot. Recover the slot via offsetof (GCC supports this
  // for non-standard-layout types — conditionally-supported per [support.types.layout]).
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Winvalid-offsetof"
  auto *slot = reinterpret_cast<ReusableSlot *>(
      reinterpret_cast<char *>(base) - offsetof(ReusableSlot, op_base));
#pragma GCC diagnostic pop

  // In debug builds, verify that mock/real DSA reported success.
  // This catches misuse on real hardware where page faults or errors
  // would be silently ignored by this benchmark-only path.
#ifndef NDEBUG
  {
    auto *comp = static_cast<dsa::DsaOperationBase *>(base)->comp_ptr();
    assert((comp->status & DSA_COMP_STATUS_MASK) == DSA_COMP_SUCCESS &&
           "ReusableSlot: unexpected completion status (page fault or DSA error). "
           "This path skips page fault retry — use stdexec-based strategies on real hardware.");
  }
#endif

  if (slot->latency->enabled()) {
    auto end = std::chrono::high_resolution_clock::now();
    slot->latency->record(
        std::chrono::duration<double, std::nano>(end - slot->start_time).count());
  }

  slot->remaining->fetch_sub(1, std::memory_order_release);
  slot->arena->release(slot);
}

// ============================================================================
// with_reusable_fill: compile-time dispatch to the right descriptor fill lambda
// ============================================================================
//
// Similar to with_op_sender but produces a (dsa_hw_desc&, size_t offset) -> void
// lambda instead of a sender factory. This allows the compiler to inline the
// fill function into the hot loop (no runtime switch per-op).

template <class F>
static void with_reusable_fill(OperationType op_type, BufferSet &bufs,
                               size_t msg_size, F &&f) {
  switch (op_type) {
  case OperationType::DataMove:
    f([&](dsa_hw_desc &desc, size_t offset) {
      dsa::fill_data_move(desc, bufs.src.data() + offset,
                          bufs.dst.data() + offset, msg_size);
    });
    break;
  case OperationType::MemFill:
    f([&](dsa_hw_desc &desc, size_t offset) {
      dsa::fill_mem_fill(desc, bufs.dst.data() + offset, msg_size,
                         BufferSet::fill_pattern);
    });
    break;
  case OperationType::Compare:
    f([&](dsa_hw_desc &desc, size_t offset) {
      dsa::fill_compare(desc, bufs.src.data() + offset,
                        bufs.dst.data() + offset, msg_size);
    });
    break;
  case OperationType::CompareValue:
    f([&](dsa_hw_desc &desc, size_t offset) {
      dsa::fill_compare_value(desc, bufs.src.data() + offset, msg_size,
                              BufferSet::fill_pattern);
    });
    break;
  case OperationType::Dualcast:
    f([&](dsa_hw_desc &desc, size_t offset) {
      dsa::fill_dualcast(desc, bufs.src.data() + offset,
                         bufs.dualcast_dst1 + offset,
                         bufs.dualcast_dst2 + offset, msg_size);
    });
    break;
  case OperationType::CrcGen:
    f([&](dsa_hw_desc &desc, size_t offset) {
      dsa::fill_crc_gen(desc, bufs.src.data() + offset, msg_size);
    });
    break;
  case OperationType::CopyCrc:
    f([&](dsa_hw_desc &desc, size_t offset) {
      dsa::fill_copy_crc(desc, bufs.src.data() + offset,
                         bufs.dst.data() + offset, msg_size);
    });
    break;
  case OperationType::CacheFlush:
    f([&](dsa_hw_desc &desc, size_t offset) {
      dsa::fill_cache_flush(desc, bufs.dst.data() + offset, msg_size);
    });
    break;
  }
}

// ============================================================================
// Sliding window reusable: the hot loop
// ============================================================================

template <class FillFn>
static void sliding_window_reusable_impl_inline(
    DsaProxy &dsa, size_t concurrency, size_t msg_size, size_t total_bytes,
    LatencyCollector &latency, FillFn fill_fn) {
  size_t num_ops = total_bytes / msg_size;
  std::atomic<size_t> remaining{num_ops};
  BasicSlotArena<ReusableSlot> arena(concurrency);
  for (auto &slot : arena.pool) {
    slot->init(dsa, &remaining, &latency, &arena);
  }

  size_t next_op = 0;
  while (next_op < num_ops) {
    // Fill the pipeline: acquire slots and launch ops
    while (next_op < num_ops) {
      auto *slot = arena.acquire();
      if (!slot) break;
      size_t offset = next_op * msg_size;
      slot->relaunch(fill_fn, offset);
      ++next_op;
    }
    // Process completions to free slots
    dsa.poll();
  }

  // Drain: poll until all ops complete
  while (remaining.load(std::memory_order_acquire) > 0) {
    dsa.poll();
  }
}

// ============================================================================
// Strategy entry point
// ============================================================================

void run_sliding_window_inline_reusable(const StrategyParams &params) {
  auto &[dsa, scope, concurrency, msg_size, total_bytes, batch_size, bufs, latency, op_type] = params;
  (void)scope;      // Reusable strategy bypasses stdexec entirely
  (void)batch_size;
  with_reusable_fill(op_type, bufs, msg_size, [&](auto fill_fn) {
    sliding_window_reusable_impl_inline(dsa, concurrency, msg_size, total_bytes,
                                        latency, fill_fn);
  });
}
