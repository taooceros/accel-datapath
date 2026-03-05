// Include fmt headers first to avoid partial specialization conflicts
#include <fmt/format.h>
#include <fmt/ranges.h>
#include "config.hpp"
#include "helpers.hpp"
#include <algorithm>
#include <chrono>
#include <cstring>
#include <dsa/dsa.hpp>
#include <dsa/task_queue.hpp>
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
#include <exec/async_scope.hpp>
#include <exec/task.hpp>
#include <fstream>
#include <functional>
#include <stdexec/execution.hpp>
#include <thread>
#include <utility>
#include <vector>

// Type-erased run function signature for each DSA type
template <typename DsaType>
using RunFunction = std::function<void(DsaType &, exec::async_scope &, size_t,
                                       size_t, size_t, BufferSet &,
                                       LatencyCollector &)>;

// ============================================================================
// SPAWN HELPERS
// Each spawn_op function creates the operation sender for the given op_type,
// pipes it through a latency-recording then() with unified auto&&... signature,
// and spawns it on the scope.
// ============================================================================

template <typename DsaType>
void spawn_op(DsaType &dsa, exec::async_scope &scope, OperationType op_type,
              BufferSet &bufs, size_t offset, size_t msg_size,
              LatencyCollector &latency, std::atomic<size_t> *in_flight = nullptr) {
  auto start_time = std::chrono::high_resolution_clock::now();
  auto record = [&latency, start_time, in_flight](auto&&...) {
    auto end = std::chrono::high_resolution_clock::now();
    latency.record(std::chrono::duration<double, std::nano>(end - start_time).count());
    if (in_flight) in_flight->fetch_sub(1, std::memory_order_release);
  };
  if (in_flight) in_flight->fetch_add(1, std::memory_order_relaxed);

  using namespace dsa_stdexec;
  switch (op_type) {
    case OperationType::DataMove:
      scope.spawn(dsa_data_move(dsa, bufs.src.data() + offset, bufs.dst.data() + offset, msg_size) | stdexec::then(record));
      break;
    case OperationType::MemFill:
      scope.spawn(dsa_mem_fill(dsa, bufs.dst.data() + offset, msg_size, BufferSet::fill_pattern) | stdexec::then(record));
      break;
    case OperationType::Compare:
      scope.spawn(dsa_compare(dsa, bufs.src.data() + offset, bufs.dst.data() + offset, msg_size) | stdexec::then(record));
      break;
    case OperationType::CompareValue:
      scope.spawn(dsa_compare_value(dsa, bufs.src.data() + offset, msg_size, BufferSet::fill_pattern) | stdexec::then(record));
      break;
    case OperationType::Dualcast:
      scope.spawn(dsa_dualcast(dsa, bufs.src.data() + offset, bufs.dualcast_dst1 + offset, bufs.dualcast_dst2 + offset, msg_size) | stdexec::then(record));
      break;
    case OperationType::CrcGen:
      scope.spawn(dsa_crc_gen(dsa, bufs.src.data() + offset, msg_size) | stdexec::then(record));
      break;
    case OperationType::CopyCrc:
      scope.spawn(dsa_copy_crc(dsa, bufs.src.data() + offset, bufs.dst.data() + offset, msg_size) | stdexec::then(record));
      break;
    case OperationType::CacheFlush:
      scope.spawn(dsa_cache_flush(dsa, bufs.dst.data() + offset, msg_size) | stdexec::then(record));
      break;
  }
}

// Scheduled variant: wraps operation in scheduler.schedule() | let_value(...)
template <typename DsaType, typename Scheduler>
void spawn_op_scheduled(DsaType &dsa, Scheduler &scheduler,
                        exec::async_scope &scope, OperationType op_type,
                        BufferSet &bufs, size_t offset, size_t msg_size,
                        LatencyCollector &latency,
                        std::atomic<size_t> *in_flight = nullptr) {
  auto start_time = std::chrono::high_resolution_clock::now();
  if (in_flight) in_flight->fetch_add(1, std::memory_order_relaxed);

  auto record = [&latency, start_time, in_flight](auto&&...) {
    auto end = std::chrono::high_resolution_clock::now();
    latency.record(std::chrono::duration<double, std::nano>(end - start_time).count());
    if (in_flight) in_flight->fetch_sub(1, std::memory_order_release);
  };

  using namespace dsa_stdexec;
  switch (op_type) {
    case OperationType::DataMove:
      scope.spawn(scheduler.schedule() | stdexec::let_value([&dsa, &bufs, offset, msg_size, record]() {
        return dsa_data_move(dsa, bufs.src.data() + offset, bufs.dst.data() + offset, msg_size) | stdexec::then(record);
      }));
      break;
    case OperationType::MemFill:
      scope.spawn(scheduler.schedule() | stdexec::let_value([&dsa, &bufs, offset, msg_size, record]() {
        return dsa_mem_fill(dsa, bufs.dst.data() + offset, msg_size, BufferSet::fill_pattern) | stdexec::then(record);
      }));
      break;
    case OperationType::Compare:
      scope.spawn(scheduler.schedule() | stdexec::let_value([&dsa, &bufs, offset, msg_size, record]() {
        return dsa_compare(dsa, bufs.src.data() + offset, bufs.dst.data() + offset, msg_size) | stdexec::then(record);
      }));
      break;
    case OperationType::CompareValue:
      scope.spawn(scheduler.schedule() | stdexec::let_value([&dsa, &bufs, offset, msg_size, record]() {
        return dsa_compare_value(dsa, bufs.src.data() + offset, msg_size, BufferSet::fill_pattern) | stdexec::then(record);
      }));
      break;
    case OperationType::Dualcast:
      scope.spawn(scheduler.schedule() | stdexec::let_value([&dsa, &bufs, offset, msg_size, record]() {
        return dsa_dualcast(dsa, bufs.src.data() + offset, bufs.dualcast_dst1 + offset, bufs.dualcast_dst2 + offset, msg_size) | stdexec::then(record);
      }));
      break;
    case OperationType::CrcGen:
      scope.spawn(scheduler.schedule() | stdexec::let_value([&dsa, &bufs, offset, msg_size, record]() {
        return dsa_crc_gen(dsa, bufs.src.data() + offset, msg_size) | stdexec::then(record);
      }));
      break;
    case OperationType::CopyCrc:
      scope.spawn(scheduler.schedule() | stdexec::let_value([&dsa, &bufs, offset, msg_size, record]() {
        return dsa_copy_crc(dsa, bufs.src.data() + offset, bufs.dst.data() + offset, msg_size) | stdexec::then(record);
      }));
      break;
    case OperationType::CacheFlush:
      scope.spawn(scheduler.schedule() | stdexec::let_value([&dsa, &bufs, offset, msg_size, record]() {
        return dsa_cache_flush(dsa, bufs.dst.data() + offset, msg_size) | stdexec::then(record);
      }));
      break;
  }
}

// ============================================================================
// SLIDING WINDOW STRATEGY
// ============================================================================

template <typename DsaType>
void run_sliding_window_inline(DsaType &dsa, exec::async_scope &scope,
                               size_t concurrency, size_t msg_size, size_t total_bytes,
                               BufferSet &bufs, LatencyCollector &latency,
                               OperationType op_type) {
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

  size_t num_ops = total_bytes / msg_size;
  std::atomic<size_t> in_flight{0};
  size_t next_op = 0;

  while (next_op < num_ops) {
    while (next_op < num_ops && in_flight.load(std::memory_order_acquire) < concurrency) {
      size_t offset = next_op * msg_size;
      spawn_op(dsa, scope, op_type, bufs, offset, msg_size, latency, &in_flight);
      ++next_op;
    }
    dsa.poll();
  }

  dsa_stdexec::wait_start(scope.on_empty(), loop);
}

template <typename DsaType>
void run_sliding_window_threaded(DsaType &dsa, exec::async_scope &scope,
                                 size_t concurrency, size_t msg_size, size_t total_bytes,
                                 BufferSet &bufs, LatencyCollector &latency,
                                 OperationType op_type) {
  dsa_stdexec::DsaScheduler<DsaType> scheduler(dsa);

  size_t num_ops = total_bytes / msg_size;
  std::atomic<size_t> in_flight{0};

  for (size_t op_idx = 0; op_idx < num_ops; ++op_idx) {
    while (in_flight.load(std::memory_order_acquire) >= concurrency) {
      std::this_thread::yield();
    }

    size_t offset = op_idx * msg_size;
    spawn_op_scheduled(dsa, scheduler, scope, op_type, bufs, offset, msg_size, latency, &in_flight);
  }
  stdexec::sync_wait(scope.on_empty());
}

// ============================================================================
// BATCH STRATEGY
// ============================================================================

template <typename DsaType>
void run_batch_inline(DsaType &dsa, exec::async_scope &scope,
                      size_t concurrency, size_t msg_size, size_t total_bytes,
                      BufferSet &bufs, LatencyCollector &latency,
                      OperationType op_type) {
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

  size_t num_ops = total_bytes / msg_size;
  size_t op_idx = 0;

  while (op_idx < num_ops) {
    size_t batch_end = std::min(op_idx + concurrency, num_ops);
    for (size_t i = op_idx; i < batch_end; ++i) {
      size_t offset = i * msg_size;
      spawn_op(dsa, scope, op_type, bufs, offset, msg_size, latency);
    }
    dsa_stdexec::wait_start(scope.on_empty(), loop);
    loop.reset();
    op_idx = batch_end;
  }
}

template <typename DsaType>
void run_batch_threaded(DsaType &dsa, exec::async_scope &scope,
                        size_t concurrency, size_t msg_size, size_t total_bytes,
                        BufferSet &bufs, LatencyCollector &latency,
                        OperationType op_type) {
  dsa_stdexec::DsaScheduler<DsaType> scheduler(dsa);

  size_t num_ops = total_bytes / msg_size;
  size_t op_idx = 0;

  while (op_idx < num_ops) {
    size_t batch_end = std::min(op_idx + concurrency, num_ops);
    for (size_t i = op_idx; i < batch_end; ++i) {
      size_t offset = i * msg_size;
      spawn_op_scheduled(dsa, scheduler, scope, op_type, bufs, offset, msg_size, latency);
    }
    stdexec::sync_wait(scope.on_empty());
    op_idx = batch_end;
  }
}

// ============================================================================
// SCOPED WORKERS STRATEGY
// ============================================================================

template <typename DsaType>
exec::task<void> worker_coro(DsaType &dsa, BufferSet &bufs,
                              LatencyCollector &latency, OperationType op_type,
                              size_t msg_size, size_t num_ops,
                              size_t num_workers, size_t worker_id) {
  size_t current_op = worker_id;

  while (current_op < num_ops) {
    size_t offset = current_op * msg_size;
    auto start_time = std::chrono::high_resolution_clock::now();

    using namespace dsa_stdexec;
    switch (op_type) {
      case OperationType::DataMove:
        co_await dsa_data_move(dsa, bufs.src.data() + offset, bufs.dst.data() + offset, msg_size);
        break;
      case OperationType::MemFill:
        co_await dsa_mem_fill(dsa, bufs.dst.data() + offset, msg_size, BufferSet::fill_pattern);
        break;
      case OperationType::Compare:
        co_await dsa_compare(dsa, bufs.src.data() + offset, bufs.dst.data() + offset, msg_size);
        break;
      case OperationType::CompareValue:
        co_await dsa_compare_value(dsa, bufs.src.data() + offset, msg_size, BufferSet::fill_pattern);
        break;
      case OperationType::Dualcast:
        co_await dsa_dualcast(dsa, bufs.src.data() + offset, bufs.dualcast_dst1 + offset, bufs.dualcast_dst2 + offset, msg_size);
        break;
      case OperationType::CrcGen:
        co_await dsa_crc_gen(dsa, bufs.src.data() + offset, msg_size);
        break;
      case OperationType::CopyCrc:
        co_await dsa_copy_crc(dsa, bufs.src.data() + offset, bufs.dst.data() + offset, msg_size);
        break;
      case OperationType::CacheFlush:
        co_await dsa_cache_flush(dsa, bufs.dst.data() + offset, msg_size);
        break;
    }

    auto end_time = std::chrono::high_resolution_clock::now();
    latency.record(std::chrono::duration<double, std::nano>(end_time - start_time).count());
    current_op += num_workers;
  }

  co_return;
}

template <typename DsaType>
void run_scoped_workers_inline(DsaType &dsa, exec::async_scope &scope,
                               size_t concurrency, size_t msg_size, size_t total_bytes,
                               BufferSet &bufs, LatencyCollector &latency,
                               OperationType op_type) {
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });
  auto scheduler = loop.get_scheduler();

  size_t num_ops = total_bytes / msg_size;
  size_t actual_workers = std::min(concurrency, num_ops);

  for (size_t worker_id = 0; worker_id < actual_workers; ++worker_id) {
    scope.spawn(
        scheduler.schedule()
      | stdexec::let_value([&dsa, &bufs, &latency, op_type, msg_size, num_ops, actual_workers, worker_id]() {
          return worker_coro(dsa, bufs, latency, op_type, msg_size, num_ops, actual_workers, worker_id);
        })
    );
  }

  dsa_stdexec::wait_start(scope.on_empty(), loop);
}

template <typename DsaType>
void run_scoped_workers_threaded(DsaType &dsa, exec::async_scope &scope,
                                 size_t concurrency, size_t msg_size, size_t total_bytes,
                                 BufferSet &bufs, LatencyCollector &latency,
                                 OperationType op_type) {
  dsa_stdexec::DsaScheduler<DsaType> scheduler(dsa);

  size_t num_ops = total_bytes / msg_size;
  size_t actual_workers = std::min(concurrency, num_ops);

  for (size_t worker_id = 0; worker_id < actual_workers; ++worker_id) {
    scope.spawn(
        scheduler.schedule()
      | stdexec::let_value([&dsa, &bufs, &latency, op_type, msg_size, num_ops, actual_workers, worker_id]() {
          return worker_coro(dsa, bufs, latency, op_type, msg_size, num_ops, actual_workers, worker_id);
        })
    );
  }

  stdexec::sync_wait(scope.on_empty());
}

// ============================================================================
// BENCHMARK INFRASTRUCTURE
// ============================================================================

enum class BenchmarkPattern {
  SlidingWindowInline,
  SlidingWindowThreaded,
  BatchInline,
  BatchThreaded,
  ScopedWorkersInline,
  ScopedWorkersThreaded
};

template <typename DsaType>
void dispatch_run(BenchmarkPattern pattern, OperationType op_type,
                  DsaType &dsa, exec::async_scope &scope,
                  size_t concurrency, size_t msg_size, size_t total_bytes,
                  BufferSet &bufs, LatencyCollector &latency) {
  switch (pattern) {
    case BenchmarkPattern::SlidingWindowInline:
      run_sliding_window_inline(dsa, scope, concurrency, msg_size, total_bytes, bufs, latency, op_type);
      break;
    case BenchmarkPattern::SlidingWindowThreaded:
      run_sliding_window_threaded(dsa, scope, concurrency, msg_size, total_bytes, bufs, latency, op_type);
      break;
    case BenchmarkPattern::BatchInline:
      run_batch_inline(dsa, scope, concurrency, msg_size, total_bytes, bufs, latency, op_type);
      break;
    case BenchmarkPattern::BatchThreaded:
      run_batch_threaded(dsa, scope, concurrency, msg_size, total_bytes, bufs, latency, op_type);
      break;
    case BenchmarkPattern::ScopedWorkersInline:
      run_scoped_workers_inline(dsa, scope, concurrency, msg_size, total_bytes, bufs, latency, op_type);
      break;
    case BenchmarkPattern::ScopedWorkersThreaded:
      run_scoped_workers_threaded(dsa, scope, concurrency, msg_size, total_bytes, bufs, latency, op_type);
      break;
  }
}

template <typename DsaType>
DsaMetric run_benchmark(DsaType &dsa, size_t concurrency, size_t msg_size,
                        size_t total_bytes, int iterations,
                        BufferSet &bufs,
                        const RunFunction<DsaType> &run_fn,
                        ProgressBar *progress = nullptr) {
  LatencyCollector warmup_latency;
  LatencyCollector latency;

  // Warmup (1 full iteration)
  {
    exec::async_scope scope;
    run_fn(dsa, scope, concurrency, msg_size, total_bytes, bufs, warmup_latency);
  }

  dsa_stdexec::reset_page_fault_retries();

  auto start = std::chrono::high_resolution_clock::now();
  for (int i = 0; i < iterations; ++i) {
    exec::async_scope scope;
    run_fn(dsa, scope, concurrency, msg_size, total_bytes, bufs, latency);
    if (progress) progress->increment();
  }
  auto end = std::chrono::high_resolution_clock::now();

  uint64_t page_faults = dsa_stdexec::get_page_fault_retries();
  std::chrono::duration<double> diff = end - start;
  double bw = static_cast<double>(total_bytes) * iterations / (1024.0 * 1024.0 * 1024.0) / diff.count();
  size_t num_ops = total_bytes / msg_size;
  double msg_rate = static_cast<double>(num_ops) * iterations / 1e6 / diff.count();
  return {bw, msg_rate, page_faults, latency.compute_stats()};
}

std::string format_metric(const DsaMetric &m) {
  if (m.page_faults == 0) {
    return fmt::format("{:.2f}", m.bandwidth);
  } else {
    return fmt::format("{:.2f}({})", m.bandwidth, m.page_faults);
  }
}

void export_to_csv(const std::string &filename,
                   const std::vector<std::pair<std::string, std::vector<BenchmarkResult>>> &all_results) {
  std::ofstream file(filename);
  if (!file.is_open()) {
    fmt::println(stderr, "Failed to open {} for writing", filename);
    return;
  }

  file << "operation,pattern,polling_mode,queue_type,concurrency,msg_size,bandwidth_gbps,msg_rate_mps,page_faults,"
       << "latency_min_ns,latency_max_ns,latency_avg_ns,latency_p50_ns,latency_p99_ns,latency_count\n";

  auto write_row = [&file](const char *operation, const char *pattern, const char *polling_mode,
                            const char *queue_type, size_t concurrency,
                            size_t msg_size, const DsaMetric &m) {
    file << operation << "," << pattern << "," << polling_mode << "," << queue_type << ","
         << concurrency << "," << msg_size << "," << m.bandwidth << ","
         << m.msg_rate << "," << m.page_faults << "," << m.latency.min_ns << "," << m.latency.max_ns
         << "," << m.latency.avg_ns << "," << m.latency.p50_ns << ","
         << m.latency.p99_ns << "," << m.latency.count << "\n";
  };

  for (const auto &[label, results] : all_results) {
    // Label format: "operation_pattern_pollingmode" e.g. "data_move__sliding_window_inline"
    // We use double underscore as separator between operation and pattern_polling
    size_t sep = label.find("__");
    std::string op_name = label.substr(0, sep);
    std::string rest = label.substr(sep + 2);
    size_t underscore_pos = rest.rfind('_');
    std::string pattern = rest.substr(0, underscore_pos);
    std::string polling_mode = rest.substr(underscore_pos + 1);
    bool include_nolock = (polling_mode == "inline");

    for (const auto &r : results) {
      if (include_nolock) {
        write_row(op_name.c_str(), pattern.c_str(), polling_mode.c_str(), "NoLock", r.concurrency, r.msg_size, r.single_thread);
      }
      write_row(op_name.c_str(), pattern.c_str(), polling_mode.c_str(), "Mutex", r.concurrency, r.msg_size, r.mutex);
      write_row(op_name.c_str(), pattern.c_str(), polling_mode.c_str(), "TAS", r.concurrency, r.msg_size, r.tas_spinlock);
      write_row(op_name.c_str(), pattern.c_str(), polling_mode.c_str(), "TTAS", r.concurrency, r.msg_size, r.ttas_spinlock);
      write_row(op_name.c_str(), pattern.c_str(), polling_mode.c_str(), "Backoff", r.concurrency, r.msg_size, r.backoff_spinlock);
      write_row(op_name.c_str(), pattern.c_str(), polling_mode.c_str(), "LockFree", r.concurrency, r.msg_size, r.lockfree);
    }
  }

  file.close();
  fmt::println("Results exported to {}", filename);
}

// Run benchmarks for all queue types with a given pattern and operation
std::vector<BenchmarkResult> run_all_queues(
    const BenchmarkConfig &config,
    BufferSet &bufs,
    bool use_threaded_polling,
    BenchmarkPattern pattern,
    OperationType op_type,
    const char *pattern_name) {

  std::vector<BenchmarkResult> results;

  size_t queue_count = 0;
  if (!use_threaded_polling && config.run_nolock) queue_count++;
  if (config.run_mutex) queue_count++;
  if (config.run_tas) queue_count++;
  if (config.run_ttas) queue_count++;
  if (config.run_backoff) queue_count++;
  if (config.run_lockfree) queue_count++;

  size_t total_configs = config.concurrency_levels.size() * config.msg_sizes.size();
  size_t total_iterations = total_configs * queue_count * config.iterations;

  std::string progress_label = fmt::format("{}/{}", operation_name(op_type), pattern_name);
  ProgressBar progress(total_iterations, progress_label);

  auto make_run_fn = [pattern, op_type](auto &dsa) -> RunFunction<std::remove_reference_t<decltype(dsa)>> {
    return [pattern, op_type](auto &d, exec::async_scope &scope, size_t c, size_t m, size_t t,
                               BufferSet &b, LatencyCollector &l) {
      dispatch_run(pattern, op_type, d, scope, c, m, t, b, l);
    };
  };

  for (auto concurrency : config.concurrency_levels) {
    for (auto msg_size : config.msg_sizes) {
      size_t effective_total_bytes = config.total_bytes;
      if (config.max_ops > 0) {
        size_t max_bytes = config.max_ops * msg_size;
        effective_total_bytes = std::min(config.total_bytes, max_bytes);
      }

      BenchmarkResult result{concurrency, msg_size, {}, {}, {}, {}, {}, {}};

      progress.set_label(fmt::format("{}/{} c={} sz={}", operation_name(op_type), pattern_name, concurrency, msg_size));

      if (!use_threaded_polling && config.run_nolock) {
        DsaSingleThread dsa(false);
        result.single_thread = run_benchmark(dsa, concurrency, msg_size,
            effective_total_bytes, config.iterations, bufs, make_run_fn(dsa), &progress);
      }
      if (config.run_mutex) {
        Dsa dsa(use_threaded_polling);
        result.mutex = run_benchmark(dsa, concurrency, msg_size,
            effective_total_bytes, config.iterations, bufs, make_run_fn(dsa), &progress);
      }
      if (config.run_tas) {
        DsaTasSpinlock dsa(use_threaded_polling);
        result.tas_spinlock = run_benchmark(dsa, concurrency, msg_size,
            effective_total_bytes, config.iterations, bufs, make_run_fn(dsa), &progress);
      }
      if (config.run_ttas) {
        DsaSpinlock dsa(use_threaded_polling);
        result.ttas_spinlock = run_benchmark(dsa, concurrency, msg_size,
            effective_total_bytes, config.iterations, bufs, make_run_fn(dsa), &progress);
      }
      if (config.run_backoff) {
        DsaBackoffSpinlock dsa(use_threaded_polling);
        result.backoff_spinlock = run_benchmark(dsa, concurrency, msg_size,
            effective_total_bytes, config.iterations, bufs, make_run_fn(dsa), &progress);
      }
      if (config.run_lockfree) {
        DsaLockFree dsa(use_threaded_polling);
        result.lockfree = run_benchmark(dsa, concurrency, msg_size,
            effective_total_bytes, config.iterations, bufs, make_run_fn(dsa), &progress);
      }

      results.push_back(result);
    }
  }

  progress.finish();
  return results;
}

void benchmark_queues_with_dsa(const BenchmarkConfig &config) {
  auto enabled_ops = config.enabled_operations();
  if (enabled_ops.empty()) {
    fmt::println("No operations enabled.");
    return;
  }

  fmt::println("=== DSA BENCHMARK WITH DIFFERENT TASK QUEUES ===\n");
  fmt::println("Configuration:");
  fmt::println("  Total bytes per iteration: {} MB", config.total_bytes / (1024 * 1024));
  fmt::println("  Iterations: {}", config.iterations);
  fmt::println("  Concurrency levels: {}", fmt::join(config.concurrency_levels, ", "));
  fmt::println("  Message sizes: {}", fmt::join(config.msg_sizes, ", "));
  fmt::println("  Operations: {}", [&] {
    std::string s;
    for (size_t i = 0; i < enabled_ops.size(); ++i) {
      if (i > 0) s += ", ";
      s += operation_name(enabled_ops[i]);
    }
    return s;
  }());
  fmt::println("");

  BufferSet bufs(config.total_bytes);

  std::vector<std::pair<std::string, std::vector<BenchmarkResult>>> all_results;

  struct PatternConfig {
    bool enabled;
    bool threaded;
    BenchmarkPattern pattern;
    const char *name;
  };

  PatternConfig pattern_configs[] = {
    {config.run_sliding_window && config.run_inline,   false, BenchmarkPattern::SlidingWindowInline,   "sliding_window"},
    {config.run_sliding_window && config.run_threaded,  true, BenchmarkPattern::SlidingWindowThreaded, "sliding_window"},
    {config.run_batch && config.run_inline,             false, BenchmarkPattern::BatchInline,           "batch"},
    {config.run_batch && config.run_threaded,            true, BenchmarkPattern::BatchThreaded,         "batch"},
    {config.run_scoped_workers && config.run_inline,    false, BenchmarkPattern::ScopedWorkersInline,   "scoped_workers"},
    {config.run_scoped_workers && config.run_threaded,   true, BenchmarkPattern::ScopedWorkersThreaded, "scoped_workers"},
  };

  for (auto op_type : enabled_ops) {
    const char *op_name = operation_name(op_type);
    for (auto &[enabled, threaded, pattern, name] : pattern_configs) {
      if (!enabled) continue;
      const char *mode = threaded ? "threaded" : "inline";
      fmt::println("Running {} {} + {} polling...", op_name, name, mode);
      auto results = run_all_queues(config, bufs, threaded, pattern, op_type, name);
      all_results.emplace_back(fmt::format("{}__{}_{}", op_name, name, mode), std::move(results));
      fmt::println("");
    }
  }

  // Print results tables
  fmt::println("==============================================================="
               "=================");
  fmt::println("                              BENCHMARK RESULTS");
  fmt::println("==============================================================="
               "=================\n");

  auto print_results_table = [](const std::string& title,
                                 const std::vector<BenchmarkResult>& results,
                                 bool include_nolock) {
    if (results.empty()) return;

    fmt::println("========== {} ==========\n", title);
    if (include_nolock) {
      fmt::println("{:>5} {:>10} {:>16} {:>16} {:>16} {:>16} {:>16} {:>16}",
                   "Conc", "Size", "NoLock", "Mutex", "TAS", "TTAS", "Backoff",
                   "LockFree");
      fmt::println(
          "{:-^5} {:-^10} {:-^16} {:-^16} {:-^16} {:-^16} {:-^16} {:-^16}",
          "", "", "", "", "", "", "", "");
      for (const auto &r : results) {
        fmt::println(
            "{:>5} {:>10} {:>16} {:>16} {:>16} {:>16} {:>16} {:>16}",
            r.concurrency, r.msg_size, format_metric(r.single_thread),
            format_metric(r.mutex), format_metric(r.tas_spinlock),
            format_metric(r.ttas_spinlock), format_metric(r.backoff_spinlock),
            format_metric(r.lockfree));
      }
    } else {
      fmt::println("{:>5} {:>10} {:>16} {:>16} {:>16} {:>16} {:>16}",
                   "Conc", "Size", "Mutex", "TAS", "TTAS", "Backoff", "LockFree");
      fmt::println("{:-^5} {:-^10} {:-^16} {:-^16} {:-^16} {:-^16} {:-^16}",
                   "", "", "", "", "", "", "");
      for (const auto &r : results) {
        fmt::println("{:>5} {:>10} {:>16} {:>16} {:>16} {:>16} {:>16}",
                     r.concurrency, r.msg_size, format_metric(r.mutex),
                     format_metric(r.tas_spinlock), format_metric(r.ttas_spinlock),
                     format_metric(r.backoff_spinlock), format_metric(r.lockfree));
      }
    }
    fmt::println("");
  };

  for (const auto &[label, results] : all_results) {
    bool include_nolock = label.find("inline") != std::string::npos;
    std::string title = label;
    std::transform(title.begin(), title.end(), title.begin(), ::toupper);
    print_results_table(title, results, include_nolock);
  }

  export_to_csv(config.csv_file, all_results);
}

int main(int argc, char **argv) {
  BenchmarkConfig config = parse_args(argc, argv);

  try {
    benchmark_queues_with_dsa(config);
    fmt::println("");
    fmt::println("Benchmark completed.");
  } catch (const std::exception &e) {
    fmt::println(stderr, "Error: {}", e.what());
    return 1;
  }

  return 0;
}
