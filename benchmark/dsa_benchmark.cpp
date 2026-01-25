// Include fmt headers first to avoid partial specialization conflicts
#include <fmt/format.h>
#include <fmt/ranges.h>
#include "benchmark_config.hpp"
#include "benchmark_helpers.hpp"
#include <algorithm>
#include <chrono>
#include <cstring>
#include <dsa/dsa.hpp>
#include <dsa/task_queue.hpp>
#include <dsa_stdexec/operations/data_move.hpp>
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
// Using std::function reduces template instantiation outside the hot path
template <typename DsaType>
using RunFunction = std::function<void(DsaType &, exec::async_scope &, size_t,
                                       size_t, size_t, std::vector<char> &,
                                       std::vector<char> &, LatencyCollector &)>;

// ============================================================================
// SLIDING WINDOW STRATEGY (semaphore-like)
// Maintains exactly `concurrency` operations in-flight. As one completes,
// immediately spawn the next one.
// ============================================================================

// Sliding window + inline polling
template <typename DsaType>
void run_sliding_window_inline(DsaType &dsa, exec::async_scope &scope,
                               size_t concurrency, size_t msg_size, size_t total_bytes,
                               std::vector<char> &src, std::vector<char> &dst,
                               LatencyCollector &latency) {
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });
  auto scheduler = loop.get_scheduler();

  size_t num_ops = total_bytes / msg_size;
  std::atomic<size_t> in_flight{0};
  size_t next_op = 0;

  auto spawn_one = [&](size_t op_idx) {
    size_t offset = op_idx * msg_size;
    auto start_time = std::chrono::high_resolution_clock::now();
    // Don't use scheduler.schedule() here - it queues to the run loop which
    // we can't process while spawning. Instead, start with dsa_data_move directly.
    auto snd = dsa_stdexec::dsa_data_move(dsa, src.data() + offset,
                                           dst.data() + offset, msg_size)
             | stdexec::then([&latency, start_time, &in_flight]() {
                 auto end_time = std::chrono::high_resolution_clock::now();
                 latency.record(std::chrono::duration<double, std::nano>(
                     end_time - start_time).count());
                 in_flight.fetch_sub(1, std::memory_order_release);
               });
    in_flight.fetch_add(1, std::memory_order_relaxed);
    scope.spawn(std::move(snd));
  };

  // Spawn operations with concurrency limiting (sliding window)
  while (next_op < num_ops) {
    // Spawn up to concurrency limit
    while (next_op < num_ops && in_flight.load(std::memory_order_acquire) < concurrency) {
      spawn_one(next_op++);
    }
    // Poll to make progress and free up slots
    dsa.poll();
  }

  dsa_stdexec::wait_start(scope.on_empty(), loop);
}

// Sliding window + threaded polling
template <typename DsaType>
void run_sliding_window_threaded(DsaType &dsa, exec::async_scope &scope,
                                 size_t concurrency, size_t msg_size, size_t total_bytes,
                                 std::vector<char> &src, std::vector<char> &dst,
                                 LatencyCollector &latency) {
  dsa_stdexec::DsaScheduler<DsaType> scheduler(dsa);

  size_t num_ops = total_bytes / msg_size;
  std::atomic<size_t> in_flight{0};

  for (size_t op_idx = 0; op_idx < num_ops; ++op_idx) {
    while (in_flight.load(std::memory_order_acquire) >= concurrency) {
      std::this_thread::yield();
    }

    size_t offset = op_idx * msg_size;
    auto start_time = std::chrono::high_resolution_clock::now();
    in_flight.fetch_add(1, std::memory_order_relaxed);

    auto snd = scheduler.schedule()
             | stdexec::let_value([&dsa, &src, &dst, offset, msg_size, &latency, start_time, &in_flight]() {
                 return dsa_stdexec::dsa_data_move(dsa, src.data() + offset,
                                                   dst.data() + offset, msg_size)
                      | stdexec::then([&latency, start_time, &in_flight]() {
                          auto end_time = std::chrono::high_resolution_clock::now();
                          latency.record(std::chrono::duration<double, std::nano>(
                              end_time - start_time).count());
                          in_flight.fetch_sub(1, std::memory_order_release);
                        });
               });
    scope.spawn(std::move(snd));
  }
  stdexec::sync_wait(scope.on_empty());
}

// ============================================================================
// BATCH STRATEGY
// Spawn `concurrency` operations, wait for ALL to complete, then spawn next batch.
// ============================================================================

// Batch + inline polling
template <typename DsaType>
void run_batch_inline(DsaType &dsa, exec::async_scope &scope,
                      size_t concurrency, size_t msg_size, size_t total_bytes,
                      std::vector<char> &src, std::vector<char> &dst,
                      LatencyCollector &latency) {
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });

  size_t num_ops = total_bytes / msg_size;
  size_t op_idx = 0;

  while (op_idx < num_ops) {
    // Spawn a batch of up to `concurrency` operations
    size_t batch_end = std::min(op_idx + concurrency, num_ops);
    for (size_t i = op_idx; i < batch_end; ++i) {
      size_t offset = i * msg_size;
      auto start_time = std::chrono::high_resolution_clock::now();
      // Start with dsa_data_move directly - no need to go through scheduler
      auto snd = dsa_stdexec::dsa_data_move(dsa, src.data() + offset,
                                             dst.data() + offset, msg_size)
               | stdexec::then([&latency, start_time]() {
                   auto end_time = std::chrono::high_resolution_clock::now();
                   latency.record(std::chrono::duration<double, std::nano>(
                       end_time - start_time).count());
                 });
      scope.spawn(std::move(snd));
    }
    // Wait for all operations in this batch to complete
    dsa_stdexec::wait_start(scope.on_empty(), loop);
    loop.reset();  // Reset stop_ flag for next batch
    op_idx = batch_end;
  }
}

// Batch + threaded polling
template <typename DsaType>
void run_batch_threaded(DsaType &dsa, exec::async_scope &scope,
                        size_t concurrency, size_t msg_size, size_t total_bytes,
                        std::vector<char> &src, std::vector<char> &dst,
                        LatencyCollector &latency) {
  dsa_stdexec::DsaScheduler<DsaType> scheduler(dsa);

  size_t num_ops = total_bytes / msg_size;
  size_t op_idx = 0;

  while (op_idx < num_ops) {
    // Spawn a batch of up to `concurrency` operations
    size_t batch_end = std::min(op_idx + concurrency, num_ops);
    for (size_t i = op_idx; i < batch_end; ++i) {
      size_t offset = i * msg_size;
      auto start_time = std::chrono::high_resolution_clock::now();
      auto snd = scheduler.schedule()
               | stdexec::let_value([&dsa, &src, &dst, offset, msg_size, &latency, start_time]() {
                   return dsa_stdexec::dsa_data_move(dsa, src.data() + offset,
                                                     dst.data() + offset, msg_size)
                        | stdexec::then([&latency, start_time]() {
                            auto end_time = std::chrono::high_resolution_clock::now();
                            latency.record(std::chrono::duration<double, std::nano>(
                                end_time - start_time).count());
                          });
                 });
      scope.spawn(std::move(snd));
    }
    // Wait for all operations in this batch to complete
    stdexec::sync_wait(scope.on_empty());
    op_idx = batch_end;
  }
}



// Coroutine-based worker that processes items sequentially
// Uses exec::task to properly suspend/resume across async DSA operations
// Each worker handles ops: worker_id, worker_id + num_workers, worker_id + 2*num_workers, ...
template <typename DsaType>
exec::task<void> worker_coro(DsaType &dsa,
                              std::vector<char> &src, std::vector<char> &dst,
                              LatencyCollector &latency,
                              size_t msg_size, size_t num_ops,
                              size_t num_workers, size_t worker_id) {
  size_t current_op = worker_id;

  while (current_op < num_ops) {
    size_t offset = current_op * msg_size;
    auto start_time = std::chrono::high_resolution_clock::now();

    // co_await the DSA data move - this properly suspends the coroutine
    // until the hardware operation completes
    co_await dsa_stdexec::dsa_data_move(dsa,
                                         src.data() + offset,
                                         dst.data() + offset,
                                         msg_size);

    auto end_time = std::chrono::high_resolution_clock::now();
    latency.record(std::chrono::duration<double, std::nano>(end_time - start_time).count());
    current_op += num_workers;
  }

  co_return;
}

// Scoped workers + inline polling (PollingRunLoop)
// Spawns `concurrency` workers via async_scope, each processes items sequentially using coroutines
// Only `concurrency` allocations instead of num_ops allocations
template <typename DsaType>
void run_scoped_workers_inline(DsaType &dsa, exec::async_scope &scope,
                               size_t concurrency, size_t msg_size, size_t total_bytes,
                               std::vector<char> &src, std::vector<char> &dst,
                               LatencyCollector &latency) {
  // Use PollingRunLoop's scheduler for execution context scheduling
  // DSA operations are handled via dsa_data_move sender, not via scheduler
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });
  auto scheduler = loop.get_scheduler();

  size_t num_ops = total_bytes / msg_size;
  // Limit workers to num_ops if there are fewer operations than concurrency
  size_t actual_workers = std::min(concurrency, num_ops);

  // Spawn N workers using async_scope with coroutines
  for (size_t worker_id = 0; worker_id < actual_workers; ++worker_id) {
    scope.spawn(
        scheduler.schedule()
      | stdexec::let_value([&dsa, &src, &dst, &latency, msg_size, num_ops, actual_workers, worker_id]() {
          return worker_coro(dsa, src, dst, latency, msg_size, num_ops, actual_workers, worker_id);
        })
    );
  }

  dsa_stdexec::wait_start(scope.on_empty(), loop);
}



// Scoped workers + threaded polling (background thread via Dsa(true))
// Spawns `concurrency` workers via async_scope, each processes items sequentially using coroutines
// Only `concurrency` allocations instead of num_ops allocations
template <typename DsaType>
void run_scoped_workers_threaded(DsaType &dsa, exec::async_scope &scope,
                                 size_t concurrency, size_t msg_size, size_t total_bytes,
                                 std::vector<char> &src, std::vector<char> &dst,
                                 LatencyCollector &latency) {
  dsa_stdexec::DsaScheduler<DsaType> scheduler(dsa);

  size_t num_ops = total_bytes / msg_size;
  // Limit workers to num_ops if there are fewer operations than concurrency
  size_t actual_workers = std::min(concurrency, num_ops);

  // Spawn N workers using async_scope with coroutines
  for (size_t worker_id = 0; worker_id < actual_workers; ++worker_id) {
    scope.spawn(
        scheduler.schedule()
      | stdexec::let_value([&dsa, &src, &dst, &latency, msg_size, num_ops, actual_workers, worker_id]() {
          return worker_coro(dsa, src, dst, latency, msg_size, num_ops, actual_workers, worker_id);
        })
    );
  }

  stdexec::sync_wait(scope.on_empty());
}


// Generic benchmark runner using a run function
// Uses type-erased RunFunction to reduce template instantiation
template <typename DsaType>
DsaMetric run_benchmark(DsaType &dsa, size_t concurrency, size_t msg_size,
                        size_t total_bytes, int iterations,
                        std::vector<char> &src, std::vector<char> &dst,
                        const RunFunction<DsaType> &run_fn,
                        ProgressBar *progress = nullptr) {
  LatencyCollector warmup_latency;
  LatencyCollector latency;

  // Warmup (1 full iteration)
  {
    exec::async_scope scope;
    run_fn(dsa, scope, concurrency, msg_size, total_bytes, src, dst, warmup_latency);
  }

  dsa_stdexec::reset_page_fault_retries();

  auto start = std::chrono::high_resolution_clock::now();
  for (int i = 0; i < iterations; ++i) {
    exec::async_scope scope;
    run_fn(dsa, scope, concurrency, msg_size, total_bytes, src, dst, latency);
    if (progress) progress->increment();
  }
  auto end = std::chrono::high_resolution_clock::now();

  uint64_t page_faults = dsa_stdexec::get_page_fault_retries();
  std::chrono::duration<double> diff = end - start;
  double bw = static_cast<double>(total_bytes) * iterations / (1024.0 * 1024.0 * 1024.0) / diff.count();
  size_t num_ops = total_bytes / msg_size;
  double msg_rate = static_cast<double>(num_ops) * iterations / 1e6 / diff.count();  // Million msgs/sec
  return {bw, msg_rate, page_faults, latency.compute_stats()};
}


// Format a metric as "x.xxGB/s(pgfaults)"
std::string format_metric(const DsaMetric &m) {
  if (m.page_faults == 0) {
    return fmt::format("{:.2f}", m.bandwidth);
  } else {
    return fmt::format("{:.2f}({})", m.bandwidth, m.page_faults);
  }
}

// Export benchmark results to CSV file
void export_to_csv(const std::string &filename,
                   const std::vector<std::pair<std::string, std::vector<BenchmarkResult>>> &all_results) {
  std::ofstream file(filename);
  if (!file.is_open()) {
    fmt::println(stderr, "Failed to open {} for writing", filename);
    return;
  }

  // Write CSV header
  file << "pattern,polling_mode,queue_type,concurrency,msg_size,bandwidth_gbps,msg_rate_mps,page_faults,"
       << "latency_min_ns,latency_max_ns,latency_avg_ns,latency_p50_ns,latency_p99_ns,latency_count\n";

  // Helper to write one metric row
  auto write_row = [&file](const char *pattern, const char *polling_mode,
                            const char *queue_type, size_t concurrency,
                            size_t msg_size, const DsaMetric &m) {
    file << pattern << "," << polling_mode << "," << queue_type << ","
         << concurrency << "," << msg_size << "," << m.bandwidth << ","
         << m.msg_rate << "," << m.page_faults << "," << m.latency.min_ns << "," << m.latency.max_ns
         << "," << m.latency.avg_ns << "," << m.latency.p50_ns << ","
         << m.latency.p99_ns << "," << m.latency.count << "\n";
  };

  for (const auto &[label, results] : all_results) {
    // Parse label like "sliding_window_inline" or "batch_threaded"
    size_t underscore_pos = label.rfind('_');
    std::string pattern = label.substr(0, underscore_pos);
    std::string polling_mode = label.substr(underscore_pos + 1);
    bool include_nolock = (polling_mode == "inline");

    for (const auto &r : results) {
      if (include_nolock) {
        write_row(pattern.c_str(), polling_mode.c_str(), "NoLock", r.concurrency, r.msg_size, r.single_thread);
      }
      write_row(pattern.c_str(), polling_mode.c_str(), "Mutex", r.concurrency, r.msg_size, r.mutex);
      write_row(pattern.c_str(), polling_mode.c_str(), "TAS", r.concurrency, r.msg_size, r.tas_spinlock);
      write_row(pattern.c_str(), polling_mode.c_str(), "TTAS", r.concurrency, r.msg_size, r.ttas_spinlock);
      write_row(pattern.c_str(), polling_mode.c_str(), "Backoff", r.concurrency, r.msg_size, r.backoff_spinlock);
      write_row(pattern.c_str(), polling_mode.c_str(), "LockFree", r.concurrency, r.msg_size, r.lockfree);
    }
  }

  file.close();
  fmt::println("Results exported to {}", filename);
}

// Benchmark pattern enum to avoid template instantiation for each lambda
enum class BenchmarkPattern {
  SlidingWindowInline,
  SlidingWindowThreaded,
  BatchInline,
  BatchThreaded,
  ScopedWorkersInline,
  ScopedWorkersThreaded
};

// Dispatch helper to call the appropriate run function
template <typename DsaType>
void dispatch_run(BenchmarkPattern pattern, DsaType &dsa, exec::async_scope &scope,
                  size_t concurrency, size_t msg_size, size_t total_bytes,
                  std::vector<char> &src, std::vector<char> &dst,
                  LatencyCollector &latency) {
  switch (pattern) {
    case BenchmarkPattern::SlidingWindowInline:
      run_sliding_window_inline(dsa, scope, concurrency, msg_size, total_bytes, src, dst, latency);
      break;
    case BenchmarkPattern::SlidingWindowThreaded:
      run_sliding_window_threaded(dsa, scope, concurrency, msg_size, total_bytes, src, dst, latency);
      break;
    case BenchmarkPattern::BatchInline:
      run_batch_inline(dsa, scope, concurrency, msg_size, total_bytes, src, dst, latency);
      break;
    case BenchmarkPattern::BatchThreaded:
      run_batch_threaded(dsa, scope, concurrency, msg_size, total_bytes, src, dst, latency);
      break;
    case BenchmarkPattern::ScopedWorkersInline:
      run_scoped_workers_inline(dsa, scope, concurrency, msg_size, total_bytes, src, dst, latency);
      break;
    case BenchmarkPattern::ScopedWorkersThreaded:
      run_scoped_workers_threaded(dsa, scope, concurrency, msg_size, total_bytes, src, dst, latency);
      break;
  }
}

// Helper to run benchmarks for all queue types with a given pattern
// Non-templated on pattern to reduce instantiation count
std::vector<BenchmarkResult> run_all_queues(
    const BenchmarkConfig &config,
    std::vector<char> &src, std::vector<char> &dst,
    bool use_threaded_polling,
    BenchmarkPattern pattern,
    const char *pattern_name) {

  std::vector<BenchmarkResult> results;

  // Count enabled queue types for progress bar
  size_t queue_count = 0;
  if (!use_threaded_polling && config.run_nolock) queue_count++;
  if (config.run_mutex) queue_count++;
  if (config.run_tas) queue_count++;
  if (config.run_ttas) queue_count++;
  if (config.run_backoff) queue_count++;
  if (config.run_lockfree) queue_count++;

  size_t total_configs = config.concurrency_levels.size() * config.msg_sizes.size();
  size_t total_iterations = total_configs * queue_count * config.iterations;

  ProgressBar progress(total_iterations, pattern_name);

  // Create type-erased run functions for each DSA type using the dispatch helper
  auto make_run_fn = [pattern](auto &dsa) -> RunFunction<std::remove_reference_t<decltype(dsa)>> {
    return [pattern](auto &d, exec::async_scope &scope, size_t c, size_t m, size_t t,
                     std::vector<char> &s, std::vector<char> &dst, LatencyCollector &l) {
      dispatch_run(pattern, d, scope, c, m, t, s, dst, l);
    };
  };

  for (auto concurrency : config.concurrency_levels) {
    for (auto msg_size : config.msg_sizes) {
      // Calculate effective total_bytes based on max_ops limit
      size_t effective_total_bytes = config.total_bytes;
      if (config.max_ops > 0) {
        size_t max_bytes = config.max_ops * msg_size;
        effective_total_bytes = std::min(config.total_bytes, max_bytes);
      }

      BenchmarkResult result{concurrency, msg_size, {}, {}, {}, {}, {}, {}};

      progress.set_label(fmt::format("{} c={} sz={}", pattern_name, concurrency, msg_size));

      if (!use_threaded_polling && config.run_nolock) {
        DsaSingleThread dsa(false);
        result.single_thread = run_benchmark(dsa, concurrency, msg_size,
            effective_total_bytes, config.iterations, src, dst, make_run_fn(dsa), &progress);
      }
      if (config.run_mutex) {
        Dsa dsa(use_threaded_polling);
        result.mutex = run_benchmark(dsa, concurrency, msg_size,
            effective_total_bytes, config.iterations, src, dst, make_run_fn(dsa), &progress);
      }
      if (config.run_tas) {
        DsaTasSpinlock dsa(use_threaded_polling);
        result.tas_spinlock = run_benchmark(dsa, concurrency, msg_size,
            effective_total_bytes, config.iterations, src, dst, make_run_fn(dsa), &progress);
      }
      if (config.run_ttas) {
        DsaSpinlock dsa(use_threaded_polling);
        result.ttas_spinlock = run_benchmark(dsa, concurrency, msg_size,
            effective_total_bytes, config.iterations, src, dst, make_run_fn(dsa), &progress);
      }
      if (config.run_backoff) {
        DsaBackoffSpinlock dsa(use_threaded_polling);
        result.backoff_spinlock = run_benchmark(dsa, concurrency, msg_size,
            effective_total_bytes, config.iterations, src, dst, make_run_fn(dsa), &progress);
      }
      if (config.run_lockfree) {
        DsaLockFree dsa(use_threaded_polling);
        result.lockfree = run_benchmark(dsa, concurrency, msg_size,
            effective_total_bytes, config.iterations, src, dst, make_run_fn(dsa), &progress);
      }

      results.push_back(result);
    }
  }

  progress.finish();
  return results;
}

void benchmark_queues_with_dsa(const BenchmarkConfig &config) {
  fmt::println("=== DSA BENCHMARK WITH DIFFERENT TASK QUEUES ===\n");
  fmt::println("Configuration:");
  fmt::println("  Total bytes per iteration: {} MB", config.total_bytes / (1024 * 1024));
  fmt::println("  Iterations: {}", config.iterations);
  fmt::println("  Concurrency levels: {}", fmt::join(config.concurrency_levels, ", "));
  fmt::println("  Message sizes: {}", fmt::join(config.msg_sizes, ", "));
  fmt::println("");

  std::vector<char> src(config.total_bytes);
  std::vector<char> dst(config.total_bytes);
  std::memset(src.data(), 1, config.total_bytes);
  std::memset(dst.data(), 0, config.total_bytes);

  // Collect all results with labels
  std::vector<std::pair<std::string, std::vector<BenchmarkResult>>> all_results;

  // Sliding window pattern
  if (config.run_sliding_window && config.run_inline) {
    fmt::println("Running sliding_window + inline polling...");
    auto results = run_all_queues(config, src, dst, false,
        BenchmarkPattern::SlidingWindowInline, "sliding_window");
    all_results.emplace_back("sliding_window_inline", std::move(results));
    fmt::println("");
  }

  if (config.run_sliding_window && config.run_threaded) {
    fmt::println("Running sliding_window + threaded polling...");
    auto results = run_all_queues(config, src, dst, true,
        BenchmarkPattern::SlidingWindowThreaded, "sliding_window");
    all_results.emplace_back("sliding_window_threaded", std::move(results));
    fmt::println("");
  }

  // Batch pattern
  if (config.run_batch && config.run_inline) {
    fmt::println("Running batch + inline polling...");
    auto results = run_all_queues(config, src, dst, false,
        BenchmarkPattern::BatchInline, "batch");
    all_results.emplace_back("batch_inline", std::move(results));
    fmt::println("");
  }

  if (config.run_batch && config.run_threaded) {
    fmt::println("Running batch + threaded polling...");
    auto results = run_all_queues(config, src, dst, true,
        BenchmarkPattern::BatchThreaded, "batch");
    all_results.emplace_back("batch_threaded", std::move(results));
    fmt::println("");
  }

  // Scoped workers pattern
  if (config.run_scoped_workers && config.run_inline) {
    fmt::println("Running scoped_workers + inline polling...");
    auto results = run_all_queues(config, src, dst, false,
        BenchmarkPattern::ScopedWorkersInline, "scoped_workers");
    all_results.emplace_back("scoped_workers_inline", std::move(results));
    fmt::println("");
  }

  if (config.run_scoped_workers && config.run_threaded) {
    fmt::println("Running scoped_workers + threaded polling...");
    auto results = run_all_queues(config, src, dst, true,
        BenchmarkPattern::ScopedWorkersThreaded, "scoped_workers");
    all_results.emplace_back("scoped_workers_threaded", std::move(results));
    fmt::println("");
  }

  // Print results tables
  fmt::println("==============================================================="
               "=================");
  fmt::println("                              BENCHMARK RESULTS");
  fmt::println("==============================================================="
               "=================\n");

  // Helper lambda to print a results table
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
    // Convert label to uppercase for display
    std::string title = label;
    std::transform(title.begin(), title.end(), title.begin(), ::toupper);
    print_results_table(title, results, include_nolock);
  }

  // Export results to CSV
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
