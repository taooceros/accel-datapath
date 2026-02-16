// Dynamic dispatch benchmark using pro::proxy for type-erased DSA access.
// All templates instantiate once (for DsaProxy) instead of 6 times per queue type.

// Include fmt headers first to avoid partial specialization conflicts
#include <fmt/format.h>
#include <fmt/ranges.h>
#include "config.hpp"
#include "helpers.hpp"
#include "strategies.hpp"
#include <algorithm>
#include <chrono>
#include <cstring>
#include <dsa/dsa.hpp>
#include <dsa/task_queue.hpp>
#include <dsa_stdexec/dsa_facade.hpp>
#include <dsa_stdexec/operations/data_move.hpp>
#include <dsa_stdexec/sync_wait.hpp>
#include <exec/async_scope.hpp>
#include <fstream>
#include <functional>
#include <stdexec/execution.hpp>
#include <vector>

// Type-erased run function signature
using RunFunction = std::function<void(DsaProxy &, exec::async_scope &, size_t,
                                       size_t, size_t, BufferSet &,
                                       LatencyCollector &)>;

// ============================================================================
// BENCHMARK INFRASTRUCTURE
// ============================================================================

DsaMetric run_benchmark(DsaProxy &dsa, size_t concurrency, size_t msg_size,
                        size_t total_bytes, int iterations,
                        BufferSet &bufs,
                        const RunFunction &run_fn,
                        ProgressBar *progress = nullptr,
                        bool sample_latency = true) {
  LatencyCollector warmup_latency(false);  // always discard warmup
  LatencyCollector latency(sample_latency);

  // Pre-allocate latency sample storage to avoid reallocation during measurement
  size_t num_ops = total_bytes / msg_size;
  latency.reserve(num_ops * iterations);

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

// Run a single queue type benchmark, creating the right concrete DSA type.
static DsaProxy make_dsa(QueueType qt, SubmissionStrategy ss, bool use_threaded_polling) {
  using dsa_stdexec::make_dsa_proxy;
  bool poller = (qt == QueueType::NoLock) ? false : use_threaded_polling;

  switch (ss) {
  case SubmissionStrategy::DoubleBufBatch:
    switch (qt) {
      case QueueType::NoLock:   return make_dsa_proxy<DsaBatchSingleThread>(poller);
      case QueueType::Mutex:    return make_dsa_proxy<DsaBatch>(poller);
      case QueueType::TAS:      return make_dsa_proxy<DsaBatchTasSpinlock>(poller);
      case QueueType::TTAS:     return make_dsa_proxy<DsaBatchSpinlock>(poller);
      case QueueType::Backoff:  return make_dsa_proxy<DsaBatchBackoffSpinlock>(poller);
      case QueueType::LockFree: return make_dsa_proxy<DsaBatchLockFree>(poller);
    }
    break;
  case SubmissionStrategy::FixedRingBatch:
    switch (qt) {
      case QueueType::NoLock:   return make_dsa_proxy<DsaFixedRingBatchSingleThread>(poller);
      case QueueType::Mutex:    return make_dsa_proxy<DsaFixedRingBatch>(poller);
      case QueueType::TAS:      return make_dsa_proxy<DsaFixedRingBatchTasSpinlock>(poller);
      case QueueType::TTAS:     return make_dsa_proxy<DsaFixedRingBatchSpinlock>(poller);
      case QueueType::Backoff:  return make_dsa_proxy<DsaFixedRingBatchBackoffSpinlock>(poller);
      case QueueType::LockFree: return make_dsa_proxy<DsaFixedRingBatchLockFree>(poller);
    }
    break;
  case SubmissionStrategy::RingBatch:
    switch (qt) {
      case QueueType::NoLock:   return make_dsa_proxy<DsaRingBatchSingleThread>(poller);
      case QueueType::Mutex:    return make_dsa_proxy<DsaRingBatch>(poller);
      case QueueType::TAS:      return make_dsa_proxy<DsaRingBatchTasSpinlock>(poller);
      case QueueType::TTAS:     return make_dsa_proxy<DsaRingBatchSpinlock>(poller);
      case QueueType::Backoff:  return make_dsa_proxy<DsaRingBatchBackoffSpinlock>(poller);
      case QueueType::LockFree: return make_dsa_proxy<DsaRingBatchLockFree>(poller);
    }
    break;
  case SubmissionStrategy::Immediate:
    switch (qt) {
      case QueueType::NoLock:   return make_dsa_proxy<DsaSingleThread>(poller);
      case QueueType::Mutex:    return make_dsa_proxy<Dsa>(poller);
      case QueueType::TAS:      return make_dsa_proxy<DsaTasSpinlock>(poller);
      case QueueType::TTAS:     return make_dsa_proxy<DsaSpinlock>(poller);
      case QueueType::Backoff:  return make_dsa_proxy<DsaBackoffSpinlock>(poller);
      case QueueType::LockFree: return make_dsa_proxy<DsaLockFree>(poller);
    }
    break;
  }
  __builtin_unreachable();
}

static DsaMetric run_one_queue(QueueType qt, SubmissionStrategy ss, bool use_threaded_polling,
                               size_t concurrency, size_t msg_size, size_t total_bytes,
                               int iterations, BufferSet &bufs,
                               const RunFunction &run_fn, ProgressBar *progress,
                               bool sample_latency = true) {
  auto dsa = make_dsa(qt, ss, use_threaded_polling);
  return run_benchmark(dsa, concurrency, msg_size, total_bytes, iterations, bufs, run_fn, progress, sample_latency);
}

static DsaMetric& result_field(BenchmarkResult &r, QueueType qt) {
  switch (qt) {
    case QueueType::NoLock:   return r.single_thread;
    case QueueType::Mutex:    return r.mutex;
    case QueueType::TAS:      return r.tas_spinlock;
    case QueueType::TTAS:     return r.ttas_spinlock;
    case QueueType::Backoff:  return r.backoff_spinlock;
    case QueueType::LockFree: return r.lockfree;
  }
  return r.mutex;  // unreachable
}

std::vector<BenchmarkResult> run_all_queues(
    const BenchmarkConfig &config,
    BufferSet &bufs,
    SchedulingPattern sp,
    PollingMode pm,
    OperationType op_type,
    const char *pattern_name,
    SubmissionStrategy ss = SubmissionStrategy::Immediate) {

  bool use_threaded_polling = (pm == PollingMode::Threaded);
  std::vector<BenchmarkResult> results;

  size_t queue_count = 0;
  for (auto qt : config.queue_types) {
    if (qt == QueueType::NoLock && use_threaded_polling) continue;
    queue_count++;
  }

  size_t total_configs = config.concurrency_levels.size() * config.msg_sizes.size();
  size_t total_iterations = total_configs * queue_count * config.iterations;

  std::string progress_label = fmt::format("{}/{}", operation_name(op_type), pattern_name);
  ProgressBar progress(total_iterations, progress_label);

  auto run_fn = [sp, pm, op_type](DsaProxy &d, exec::async_scope &scope, size_t c, size_t m, size_t t,
                                   BufferSet &b, LatencyCollector &l) {
    dispatch_run(sp, pm, op_type, d, scope, c, m, t, b, l);
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

      for (auto qt : config.queue_types) {
        if (qt == QueueType::NoLock && use_threaded_polling) continue;

        result_field(result, qt) = run_one_queue(
            qt, ss, use_threaded_polling,
            concurrency, msg_size, effective_total_bytes,
            config.iterations, bufs, run_fn, &progress,
            config.sample_latency);
      }

      results.push_back(result);
    }
  }

  progress.finish();
  return results;
}

void benchmark_queues_with_dsa(const BenchmarkConfig &config) {
  if (config.operations.empty()) {
    fmt::println("No operations enabled.");
    return;
  }

  fmt::println("=== DSA BENCHMARK (DYNAMIC DISPATCH) WITH DIFFERENT TASK QUEUES ===\n");
  fmt::println("Configuration:");
  fmt::println("  Total bytes per iteration: {} MB", config.total_bytes / (1024 * 1024));
  fmt::println("  Iterations: {}", config.iterations);
  fmt::println("  Concurrency levels: {}", fmt::join(config.concurrency_levels, ", "));
  fmt::println("  Message sizes: {}", fmt::join(config.msg_sizes, ", "));
  fmt::println("  Latency sampling: {}", config.sample_latency ? "enabled" : "disabled");
  fmt::println("  Operations: {}", [&] {
    std::string s;
    for (size_t i = 0; i < config.operations.size(); ++i) {
      if (i > 0) s += ", ";
      s += operation_name(config.operations[i]);
    }
    return s;
  }());
  fmt::println("");

  BufferSet bufs(config.total_bytes);

  std::vector<std::pair<std::string, std::vector<BenchmarkResult>>> all_results;

  for (auto op_type : config.operations) {
    const char *op_name = operation_name(op_type);
    for (auto sp : config.scheduling_patterns) {
      for (auto pm : config.polling_modes) {
        for (auto ss : config.submission_strategies) {
          const char *sp_name = scheduling_pattern_name(sp);
          const char *pm_name = polling_mode_name(pm);
          const char *ss_name = submission_strategy_name(ss);
          std::string label_name = sp_name;
          if (ss != SubmissionStrategy::Immediate) {
            label_name += "_";
            label_name += ss_name;
          }

          fmt::println("Running {} {} + {} polling ({})...", op_name, sp_name, pm_name, ss_name);
          auto results = run_all_queues(config, bufs, sp, pm, op_type,
                                         label_name.c_str(), ss);
          all_results.emplace_back(fmt::format("{}__{}_{}", op_name, label_name, pm_name),
                                    std::move(results));
          fmt::println("");
        }
      }
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
