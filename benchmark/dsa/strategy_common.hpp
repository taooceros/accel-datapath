#pragma once
#ifndef STRATEGY_COMMON_HPP
#define STRATEGY_COMMON_HPP

// Shared utilities for all strategy TUs: with_op_sender, spawn helpers, CompletionRecord.
// All functions are static inline or templates to avoid ODR issues across TUs.

#include "strategies.hpp"
#include <chrono>
#include <dsa_stdexec/batch.hpp>
#include <dsa_stdexec/operations/cache_flush.hpp>
#include <dsa_stdexec/operations/compare.hpp>
#include <dsa_stdexec/operations/compare_value.hpp>
#include <dsa_stdexec/operations/copy_crc.hpp>
#include <dsa_stdexec/operations/crc_gen.hpp>
#include <dsa_stdexec/operations/data_move.hpp>
#include <dsa_stdexec/operations/dualcast.hpp>
#include <dsa_stdexec/operations/mem_fill.hpp>
#include <dsa_stdexec/run_loop.hpp>
#include <dsa_stdexec/scheduler.hpp>
#include <dsa_stdexec/sync_wait.hpp>
#include <stdexec/execution.hpp>
#include <thread>

// Calls f(op_sender) where op_sender is a lambda: (size_t offset) -> Sender.
template <class F>
static void with_op_sender(OperationType op_type, DsaProxy &dsa,
                           BufferSet &bufs, size_t msg_size, F &&f) {
  using namespace dsa_stdexec;
  switch (op_type) {
    case OperationType::DataMove:
      f([&](size_t off) { return dsa_data_move(dsa, bufs.src.data() + off, bufs.dst.data() + off, msg_size); });
      break;
    case OperationType::MemFill:
      f([&](size_t off) { return dsa_mem_fill(dsa, bufs.dst.data() + off, msg_size, BufferSet::fill_pattern); });
      break;
    case OperationType::Compare:
      f([&](size_t off) { return dsa_compare(dsa, bufs.src.data() + off, bufs.dst.data() + off, msg_size); });
      break;
    case OperationType::CompareValue:
      f([&](size_t off) { return dsa_compare_value(dsa, bufs.src.data() + off, msg_size, BufferSet::fill_pattern); });
      break;
    case OperationType::Dualcast:
      f([&](size_t off) { return dsa_dualcast(dsa, bufs.src.data() + off, bufs.dualcast_dst1 + off, bufs.dualcast_dst2 + off, msg_size); });
      break;
    case OperationType::CrcGen:
      f([&](size_t off) { return dsa_crc_gen(dsa, bufs.src.data() + off, msg_size); });
      break;
    case OperationType::CopyCrc:
      f([&](size_t off) { return dsa_copy_crc(dsa, bufs.src.data() + off, bufs.dst.data() + off, msg_size); });
      break;
    case OperationType::CacheFlush:
      f([&](size_t off) { return dsa_cache_flush(dsa, bufs.dst.data() + off, msg_size); });
      break;
  }
}

// Completion record used by all strategies.
// When latency sampling is enabled, records start/end timestamps.
// When disabled, skips all chrono::now() calls — zero timing overhead.
struct CompletionRecord {
  LatencyCollector *latency;
  std::chrono::high_resolution_clock::time_point start_time;
  std::atomic<size_t> *in_flight;

  // Factory: only calls now() when latency is enabled
  static CompletionRecord make(LatencyCollector &lat, std::atomic<size_t> *inf) {
    return {&lat,
            lat.enabled() ? std::chrono::high_resolution_clock::now()
                          : std::chrono::high_resolution_clock::time_point{},
            inf};
  }

  void operator()(auto &&...) const {
    if (latency->enabled()) {
      auto end = std::chrono::high_resolution_clock::now();
      latency->record(std::chrono::duration<double, std::nano>(end - start_time).count());
    }
    if (in_flight) in_flight->fetch_sub(1, std::memory_order_release);
  }
};

static inline void spawn_op(DsaProxy &dsa, exec::async_scope &scope, OperationType op_type,
                            BufferSet &bufs, size_t offset, size_t msg_size,
                            LatencyCollector &latency, std::atomic<size_t> *in_flight = nullptr) {
  if (in_flight) in_flight->fetch_add(1, std::memory_order_relaxed);
  auto record = CompletionRecord::make(latency, in_flight);

  with_op_sender(op_type, dsa, bufs, msg_size, [&](auto op_sender) {
    scope.spawn(op_sender(offset) | stdexec::then(record));
  });
}

static inline void spawn_op_scheduled(DsaProxy &dsa, dsa_stdexec::DsaScheduler<DsaProxy> &scheduler,
                                      exec::async_scope &scope, OperationType op_type,
                                      BufferSet &bufs, size_t offset, size_t msg_size,
                                      LatencyCollector &latency,
                                      std::atomic<size_t> *in_flight = nullptr) {
  if (in_flight) in_flight->fetch_add(1, std::memory_order_relaxed);
  auto record = CompletionRecord::make(latency, in_flight);

  with_op_sender(op_type, dsa, bufs, msg_size, [&](auto op_sender) {
    scope.spawn(scheduler.schedule() | stdexec::let_value([op_sender, offset, record]() {
      return op_sender(offset) | stdexec::then(record);
    }));
  });
}

// Noalloc slot size helpers — used by both noalloc and arena strategies.

template <class MakeSender>
constexpr size_t inline_noalloc_slot_size() {
  using Sender = decltype(std::declval<MakeSender>()(size_t{0}));
  using ThenSender = decltype(std::declval<Sender>() | stdexec::then(std::declval<CompletionRecord>()));
  using NestSender = exec::async_scope::nest_result_t<ThenSender>;
  return sizeof(stdexec::connect_result_t<NestSender, SlotReceiver>);
}

template <class MakeSender>
constexpr size_t threaded_noalloc_slot_size() {
  using Scheduler = dsa_stdexec::DsaScheduler<DsaProxy>;
  using SchedSender = decltype(std::declval<Scheduler>().schedule());
  struct LetLambda {
    MakeSender make_sender;
    size_t offset;
    CompletionRecord record;
    auto operator()() {
      return make_sender(offset) | stdexec::then(record);
    }
  };
  using LetSender = decltype(std::declval<SchedSender>() | stdexec::let_value(std::declval<LetLambda>()));
  using NestSender = exec::async_scope::nest_result_t<LetSender>;
  return sizeof(stdexec::connect_result_t<NestSender, SlotReceiver>);
}

// Fill a single descriptor for the given operation type.
// Used by batch_raw strategy to populate sub-descriptors in a hardware batch.
static inline void fill_for_op(dsa_hw_desc &desc, OperationType op_type,
                               BufferSet &bufs, size_t offset, size_t msg_size) {
  switch (op_type) {
  case OperationType::DataMove:
    dsa::fill_data_move(desc, bufs.src.data() + offset,
                        bufs.dst.data() + offset, msg_size);
    break;
  case OperationType::MemFill:
    dsa::fill_mem_fill(desc, bufs.dst.data() + offset, msg_size,
                       BufferSet::fill_pattern);
    break;
  case OperationType::Compare:
    dsa::fill_compare(desc, bufs.src.data() + offset,
                      bufs.dst.data() + offset, msg_size);
    break;
  case OperationType::CompareValue:
    dsa::fill_compare_value(desc, bufs.src.data() + offset, msg_size,
                            BufferSet::fill_pattern);
    break;
  case OperationType::Dualcast:
    dsa::fill_dualcast(desc, bufs.src.data() + offset,
                       bufs.dualcast_dst1 + offset,
                       bufs.dualcast_dst2 + offset, msg_size);
    break;
  case OperationType::CrcGen:
    dsa::fill_crc_gen(desc, bufs.src.data() + offset, msg_size);
    break;
  case OperationType::CopyCrc:
    dsa::fill_copy_crc(desc, bufs.src.data() + offset,
                       bufs.dst.data() + offset, msg_size);
    break;
  case OperationType::CacheFlush:
    dsa::fill_cache_flush(desc, bufs.dst.data() + offset, msg_size);
    break;
  }
}

#endif // STRATEGY_COMMON_HPP
