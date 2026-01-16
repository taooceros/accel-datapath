#include <algorithm>
#include <chrono>
#include <cstdlib>
#include <cstring>
#include <dsa/dsa.hpp>
#include <dsa/task_queue.hpp>
#include <dsa_stdexec/data_move.hpp>
#include <dsa_stdexec/run_loop.hpp>
#include <dsa_stdexec/scheduler.hpp>
#include <dsa_stdexec/sync_wait.hpp>
#include <exec/async_scope.hpp>
#include <exec/repeat_effect_until.hpp>
#include <fmt/base.h>
#include <fmt/core.h>
#include <fmt/ranges.h>
#include <fstream>
#include <functional>
#include <memory>
#include <mutex>
#include <numeric>
#include <stdexec/execution.hpp>
#include <thread>
#include <toml++/toml.hpp>
#include <utility>
#include <vector>

// Thread-safe latency collector
class LatencyCollector {
public:
  void record(double latency_ns) {
    std::lock_guard<std::mutex> lock(mutex_);
    samples_.push_back(latency_ns);
  }

  void clear() {
    std::lock_guard<std::mutex> lock(mutex_);
    samples_.clear();
  }

  struct Stats {
    double min_ns = 0;
    double max_ns = 0;
    double avg_ns = 0;
    double p50_ns = 0;
    double p99_ns = 0;
    size_t count = 0;
  };

  Stats compute_stats() {
    std::lock_guard<std::mutex> lock(mutex_);
    if (samples_.empty()) {
      return {};
    }

    std::sort(samples_.begin(), samples_.end());
    Stats s;
    s.count = samples_.size();
    s.min_ns = samples_.front();
    s.max_ns = samples_.back();
    s.avg_ns = std::accumulate(samples_.begin(), samples_.end(), 0.0) / s.count;
    s.p50_ns = samples_[s.count / 2];
    s.p99_ns = samples_[static_cast<size_t>(s.count * 0.99)];
    return s;
  }

private:
  std::mutex mutex_;
  std::vector<double> samples_;
};

// async_scope + inline polling (PollingRunLoop)
// Uses schedule() before data_move to properly anchor operations to the scheduler context
template <typename DsaType>
void run_scope_inline(DsaType &dsa, exec::async_scope &scope,
                      size_t batch_size, size_t msg_size,
                      std::vector<char> &src, std::vector<char> &dst,
                      size_t base_offset, LatencyCollector &latency) {
  dsa_stdexec::DsaScheduler<DsaType> scheduler(dsa);
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });
  for (size_t i = 0; i < batch_size; ++i) {
    size_t offset = base_offset + i * msg_size;
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
  dsa_stdexec::wait_start(scope.on_empty(), loop);
}


// async_scope + threaded polling (background thread via Dsa(true))
// MUST use schedule() before data_move to coordinate with the dedicated polling thread
template <typename DsaType>
void run_scope_threaded(DsaType &dsa, exec::async_scope &scope,
                        size_t batch_size, size_t msg_size,
                        std::vector<char> &src, std::vector<char> &dst,
                        size_t base_offset, LatencyCollector &latency) {
  dsa_stdexec::DsaScheduler<DsaType> scheduler(dsa);
  for (size_t i = 0; i < batch_size; ++i) {
    size_t offset = base_offset + i * msg_size;
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
  stdexec::sync_wait(scope.on_empty());
}


// Scoped workers + inline polling (PollingRunLoop)
// Spawns N workers via async_scope, each processes items sequentially with repeat_effect_until
// Only N allocations instead of batch_size allocations
template <typename DsaType>
void run_scoped_workers_inline(DsaType &dsa, exec::async_scope &scope,
                               size_t batch_size, size_t msg_size,
                               std::vector<char> &src, std::vector<char> &dst,
                               size_t base_offset, LatencyCollector &latency,
                               size_t num_workers = 16) {
  dsa_stdexec::DsaScheduler<DsaType> scheduler(dsa);
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });
  std::atomic<std::size_t> completed{0};

  // Limit workers to batch_size if batch is smaller
  size_t actual_workers = batch_size;

  // Store current index for each worker
  std::vector<size_t> worker_indices(actual_workers);
  for (size_t i = 0; i < actual_workers; ++i) {
    worker_indices[i] = i;
  }


  // Spawn N workers using async_scope (N allocations total)
  for (size_t worker_id = 0; worker_id < actual_workers; ++worker_id) {
    scope.spawn(
        scheduler.schedule()
      | stdexec::let_value([&, worker_id]() {
          // Each worker loops with repeat_effect_until (no allocation per iteration)
          return exec::repeat_effect_until(
              stdexec::just()
            | stdexec::then([]() { return std::chrono::high_resolution_clock::now(); })
            | stdexec::let_value([&, worker_id](auto start_time) {
                return dsa_stdexec::dsa_data_move(dsa, src.data() + base_offset + worker_indices[worker_id] * msg_size,
                                                  dst.data() + base_offset + worker_indices[worker_id] * msg_size, msg_size)
                     | stdexec::then([&, worker_id, start_time]() {
                         auto end_time = std::chrono::high_resolution_clock::now();
                         latency.record(std::chrono::duration<double, std::nano>(end_time - start_time).count());
                         completed.fetch_add(1, std::memory_order_relaxed);
                         worker_indices[worker_id] += actual_workers;
                         return worker_indices[worker_id] >= batch_size;
                       });
              })
          );
        })
    );
  }

  dsa_stdexec::wait_start(scope.on_empty(), loop);
}


// Scoped workers + threaded polling (background thread via Dsa(true))
// Spawns N workers via async_scope, each processes items sequentially with repeat_effect_until
// Only N allocations instead of batch_size allocations
template <typename DsaType>
void run_scoped_workers_threaded(DsaType &dsa, exec::async_scope &scope,
                                 size_t batch_size, size_t msg_size,
                                 std::vector<char> &src, std::vector<char> &dst,
                                 size_t base_offset, LatencyCollector &latency,
                                 size_t num_workers = 16) {
  dsa_stdexec::DsaScheduler<DsaType> scheduler(dsa);

  // Limit workers to batch_size if batch is smaller
  size_t actual_workers = std::min(num_workers, batch_size);

  // Store current index for each worker
  std::vector<size_t> worker_indices(actual_workers);
  for (size_t i = 0; i < actual_workers; ++i) {
    worker_indices[i] = i;
  }

  // Spawn N workers using async_scope (N allocations total)
  for (size_t worker_id = 0; worker_id < actual_workers; ++worker_id) {
    scope.spawn(
        scheduler.schedule()
      | stdexec::let_value([&dsa, &src, &dst, &latency, &worker_indices, base_offset, msg_size, batch_size, actual_workers, worker_id]() {
          // Each worker loops with repeat_effect_until (no allocation per iteration)
          return exec::repeat_effect_until(
              stdexec::just()
            | stdexec::then([]() { return std::chrono::high_resolution_clock::now(); })
            | stdexec::let_value([&dsa, &src, &dst, &latency, &worker_indices, base_offset, msg_size, batch_size, actual_workers, worker_id](auto start_time) {
                return dsa_stdexec::dsa_data_move(dsa, src.data() + base_offset + worker_indices[worker_id] * msg_size,
                                                  dst.data() + base_offset + worker_indices[worker_id] * msg_size, msg_size)
                     | stdexec::then([&latency, &worker_indices, msg_size, batch_size, actual_workers, worker_id, start_time]() {
                         auto end_time = std::chrono::high_resolution_clock::now();
                         latency.record(std::chrono::duration<double, std::nano>(end_time - start_time).count());
                         fmt::println("one operation done {} {}", msg_size, batch_size);
                         worker_indices[worker_id] += actual_workers;
                         return worker_indices[worker_id] >= batch_size;
                       });
              })
          );
        })
    );
  }

  stdexec::sync_wait(scope.on_empty());
  fmt::println("completed: message size {}, batch size {}", msg_size, batch_size);
}


struct DsaMetric {
  double bandwidth;
  uint64_t page_faults;
  LatencyCollector::Stats latency;
};

// Benchmark DSA dynamic batch with inline polling, returns bandwidth, page
// faults, and latency stats
template <typename DsaType>
DsaMetric benchmark_scope_inline(DsaType &dsa, exec::async_scope &scope,
                                   size_t batch_size, size_t msg_size,
                                   std::vector<char> &src,
                                   std::vector<char> &dst, int iterations) {
  size_t batch_bytes = batch_size * msg_size;
  LatencyCollector warmup_latency;
  LatencyCollector latency;

  // Warmup
  run_scope_inline(dsa, scope, batch_size, msg_size, src, dst, 0, warmup_latency);

  dsa_stdexec::reset_page_fault_retries();

  auto start = std::chrono::high_resolution_clock::now();
  for (int i = 0; i < iterations; ++i) {
    size_t base_offset = static_cast<size_t>(i) * batch_bytes;
    run_scope_inline(dsa, scope, batch_size, msg_size, src, dst, base_offset, latency);
  }
  auto end = std::chrono::high_resolution_clock::now();

  uint64_t page_faults = dsa_stdexec::get_page_fault_retries();
  std::chrono::duration<double> diff = end - start;
  double bw = (double)batch_bytes * iterations / (1024.0 * 1024.0 * 1024.0) /
              diff.count();
  return {bw, page_faults, latency.compute_stats()};
}

// Benchmark DSA dynamic batch with background thread polling, returns bandwidth,
// page faults, and latency stats
template <typename DsaType>
DsaMetric benchmark_scope_threaded(DsaType &dsa, exec::async_scope &scope,
                                     size_t batch_size, size_t msg_size,
                                     std::vector<char> &src,
                                     std::vector<char> &dst, int iterations) {
  size_t batch_bytes = batch_size * msg_size;
  LatencyCollector warmup_latency;
  LatencyCollector latency;

  // Warmup
  run_scope_threaded(dsa, scope, batch_size, msg_size, src, dst, 0, warmup_latency);

  dsa_stdexec::reset_page_fault_retries();

  auto start = std::chrono::high_resolution_clock::now();
  for (int i = 0; i < iterations; ++i) {
    size_t base_offset = static_cast<size_t>(i) * batch_bytes;
    run_scope_threaded(dsa, scope, batch_size, msg_size, src, dst, base_offset, latency);
  }
  auto end = std::chrono::high_resolution_clock::now();

  uint64_t page_faults = dsa_stdexec::get_page_fault_retries();
  std::chrono::duration<double> diff = end - start;
  double bw = (double)batch_bytes * iterations / (1024.0 * 1024.0 * 1024.0) /
              diff.count();
  return {bw, page_faults, latency.compute_stats()};
}

// Benchmark DSA with scoped workers + inline polling
template <typename DsaType>
DsaMetric benchmark_scoped_workers_inline(DsaType &dsa, exec::async_scope &scope,
                                          size_t batch_size, size_t msg_size,
                                          std::vector<char> &src,
                                          std::vector<char> &dst, int iterations) {
  size_t batch_bytes = batch_size * msg_size;
  LatencyCollector warmup_latency;
  LatencyCollector latency;

  // Warmup
  run_scoped_workers_inline(dsa, scope, batch_size, msg_size, src, dst, 0, warmup_latency);

  dsa_stdexec::reset_page_fault_retries();

  auto start = std::chrono::high_resolution_clock::now();
  for (int i = 0; i < iterations; ++i) {
    size_t base_offset = static_cast<size_t>(i) * batch_bytes;
    run_scoped_workers_inline(dsa, scope, batch_size, msg_size, src, dst, base_offset, latency);
  }
  auto end = std::chrono::high_resolution_clock::now();

  uint64_t page_faults = dsa_stdexec::get_page_fault_retries();
  std::chrono::duration<double> diff = end - start;
  double bw = (double)batch_bytes * iterations / (1024.0 * 1024.0 * 1024.0) /
              diff.count();
  return {bw, page_faults, latency.compute_stats()};
}

// Benchmark DSA with scoped workers + threaded polling
template <typename DsaType>
DsaMetric benchmark_scoped_workers_threaded(DsaType &dsa, exec::async_scope &scope,
                                            size_t batch_size, size_t msg_size,
                                            std::vector<char> &src,
                                            std::vector<char> &dst, int iterations) {
  size_t batch_bytes = batch_size * msg_size;
  LatencyCollector warmup_latency;
  LatencyCollector latency;

  // Warmup
  run_scoped_workers_threaded(dsa, scope, batch_size, msg_size, src, dst, 0, warmup_latency);

  dsa_stdexec::reset_page_fault_retries();

  auto start = std::chrono::high_resolution_clock::now();
  for (int i = 0; i < iterations; ++i) {
    size_t base_offset = static_cast<size_t>(i) * batch_bytes;
    run_scoped_workers_threaded(dsa, scope, batch_size, msg_size, src, dst, base_offset, latency);
  }
  auto end = std::chrono::high_resolution_clock::now();

  uint64_t page_faults = dsa_stdexec::get_page_fault_retries();
  std::chrono::duration<double> diff = end - start;
  double bw = (double)batch_bytes * iterations / (1024.0 * 1024.0 * 1024.0) /
              diff.count();
  return {bw, page_faults, latency.compute_stats()};
}


struct BenchmarkResult {
  size_t batch_size;
  size_t msg_size;
  DsaMetric single_thread;
  DsaMetric mutex;
  DsaMetric tas_spinlock;
  DsaMetric ttas_spinlock;
  DsaMetric backoff_spinlock;
  DsaMetric lockfree;
};

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
                   const std::vector<BenchmarkResult> &async_scope_inline,
                   const std::vector<BenchmarkResult> &async_scope_threaded,
                   const std::vector<BenchmarkResult> &scoped_workers_inline,
                   const std::vector<BenchmarkResult> &scoped_workers_threaded) {
  std::ofstream file(filename);
  if (!file.is_open()) {
    fmt::println(stderr, "Failed to open {} for writing", filename);
    return;
  }

  // Write CSV header with pattern and polling_mode as separate dimensions
  file << "pattern,polling_mode,queue_type,batch_size,msg_size,bandwidth_gbps,page_faults,"
       << "latency_min_ns,latency_max_ns,latency_avg_ns,latency_p50_ns,latency_p99_ns,latency_count\n";

  // Helper to write one metric row
  auto write_row = [&file](const char *pattern, const char *polling_mode,
                            const char *queue_type, size_t batch_size,
                            size_t msg_size, const DsaMetric &m) {
    file << pattern << "," << polling_mode << "," << queue_type << ","
         << batch_size << "," << msg_size << "," << m.bandwidth << ","
         << m.page_faults << "," << m.latency.min_ns << "," << m.latency.max_ns
         << "," << m.latency.avg_ns << "," << m.latency.p50_ns << ","
         << m.latency.p99_ns << "," << m.latency.count << "\n";
  };

  // Helper to write all queue types for a result set
  auto write_results = [&write_row](const char *pattern, const char *polling_mode,
                                     const std::vector<BenchmarkResult> &results,
                                     bool include_nolock) {
    for (const auto &r : results) {
      if (include_nolock) {
        write_row(pattern, polling_mode, "NoLock", r.batch_size, r.msg_size, r.single_thread);
      }
      write_row(pattern, polling_mode, "Mutex", r.batch_size, r.msg_size, r.mutex);
      write_row(pattern, polling_mode, "TAS", r.batch_size, r.msg_size, r.tas_spinlock);
      write_row(pattern, polling_mode, "TTAS", r.batch_size, r.msg_size, r.ttas_spinlock);
      write_row(pattern, polling_mode, "Backoff", r.batch_size, r.msg_size, r.backoff_spinlock);
      write_row(pattern, polling_mode, "LockFree", r.batch_size, r.msg_size, r.lockfree);
    }
  };

  write_results("async_scope", "inline", async_scope_inline, true);
  write_results("async_scope", "threaded", async_scope_threaded, false);
  write_results("scoped_workers", "inline", scoped_workers_inline, true);
  write_results("scoped_workers", "threaded", scoped_workers_threaded, false);

  file.close();
  fmt::println("Results exported to {}", filename);
}

// Benchmark configuration from command-line options or TOML file
struct BenchmarkConfig {
  // Polling mode dimension
  bool run_inline = true;
  bool run_threaded = true;

  // Scheduling pattern dimension
  bool run_async_scope = true;     // async_scope::spawn per operation (batch_size allocations)
  bool run_scoped_workers = false; // async_scope + repeat_effect_until (N allocations)

  // Queue type dimension
  bool run_nolock = true;
  bool run_mutex = true;
  bool run_tas = true;
  bool run_ttas = true;
  bool run_backoff = true;
  bool run_lockfree = true;

  // Benchmark parameters
  std::vector<size_t> batch_sizes = {1, 4, 16, 32};
  std::vector<size_t> msg_sizes = {256, 512, 1024, 2048, 4096, 8192, 16384};
  size_t total_bytes = 32ULL * 1024 * 1024;

  // Output configuration
  std::string csv_file = "dsa_benchmark_results.csv";
};

// Load configuration from TOML file
BenchmarkConfig load_config_from_toml(const std::string &filename) {
  BenchmarkConfig config;

  toml::table tbl;
  try {
    tbl = toml::parse_file(filename);
  } catch (const toml::parse_error &err) {
    fmt::println(stderr, "Failed to parse config file '{}': {}", filename, err.what());
    std::exit(1);
  }

  // Polling mode
  if (auto polling = tbl["polling"].as_table()) {
    config.run_inline = polling->get("inline")->value_or(true);
    config.run_threaded = polling->get("threaded")->value_or(true);
  }

  // Scheduling pattern
  if (auto scheduling = tbl["scheduling"].as_table()) {
    config.run_async_scope = scheduling->get("async_scope")->value_or(true);
    config.run_scoped_workers = scheduling->get("scoped_workers")->value_or(false);
  }

  // Queue types
  if (auto queues = tbl["queues"].as_table()) {
    config.run_nolock = queues->get("nolock")->value_or(true);
    config.run_mutex = queues->get("mutex")->value_or(true);
    config.run_tas = queues->get("tas")->value_or(true);
    config.run_ttas = queues->get("ttas")->value_or(true);
    config.run_backoff = queues->get("backoff")->value_or(true);
    config.run_lockfree = queues->get("lockfree")->value_or(true);
  }

  // Benchmark parameters
  if (auto params = tbl["parameters"].as_table()) {
    if (auto arr = params->get("batch_sizes")->as_array()) {
      config.batch_sizes.clear();
      for (const auto &elem : *arr) {
        if (auto val = elem.value<int64_t>()) {
          config.batch_sizes.push_back(static_cast<size_t>(*val));
        }
      }
    }
    if (auto arr = params->get("msg_sizes")->as_array()) {
      config.msg_sizes.clear();
      for (const auto &elem : *arr) {
        if (auto val = elem.value<int64_t>()) {
          config.msg_sizes.push_back(static_cast<size_t>(*val));
        }
      }
    }
    if (auto val = params->get("total_bytes")->value<int64_t>()) {
      config.total_bytes = static_cast<size_t>(*val);
    }
  }

  // Output configuration
  if (auto output = tbl["output"].as_table()) {
    if (auto val = output->get("csv_file")->value<std::string>()) {
      config.csv_file = *val;
    }
  }

  return config;
}

void print_usage(const char *prog) {
  fmt::println("Usage: {} [OPTIONS]", prog);
  fmt::println("");
  fmt::println("Options:");
  fmt::println("  --help, -h          Show this help message");
  fmt::println("  --config=<file>     Load configuration from TOML file");
  fmt::println("                      (command-line options override config file)");
  fmt::println("");
  fmt::println("Polling mode (can combine multiple):");
  fmt::println("  --inline            Run inline polling benchmarks (PollingRunLoop)");
  fmt::println("  --threaded          Run background thread polling benchmarks");
  fmt::println("");
  fmt::println("Scheduling pattern (can combine multiple):");
  fmt::println("  --async-scope       async_scope::spawn per op (batch_size allocations)");
  fmt::println("  --scoped-workers    N workers with repeat_effect_until (N allocations)");
  fmt::println("");
  fmt::println("Queue types:");
  fmt::println("  --queue=<type>      Run only specified queue type(s), comma-separated");
  fmt::println("                      Types: nolock, mutex, tas, ttas, backoff, lockfree");
  fmt::println("");
  fmt::println("Examples:");
  fmt::println("  {}                                  # Default: async-scope with inline+threaded", prog);
  fmt::println("  {} --config=benchmark_config.toml   # Load from TOML config file", prog);
  fmt::println("  {} --inline                         # Only inline polling", prog);
  fmt::println("  {} --threaded                       # Only background thread polling", prog);
  fmt::println("  {} --scoped-workers                 # Only scoped workers pattern", prog);
  fmt::println("  {} --scoped-workers --inline        # Scoped workers with inline only", prog);
  fmt::println("  {} --async-scope --scoped-workers   # Both patterns", prog);
  fmt::println("  {} --queue=mutex,lockfree           # Specific queue types", prog);
}

BenchmarkConfig parse_args(int argc, char **argv) {
  BenchmarkConfig config;
  bool polling_specified = false;
  bool pattern_specified = false;
  bool queue_specified = false;
  std::string config_file;

  // First pass: check for config file
  for (int i = 1; i < argc; ++i) {
    std::string arg = argv[i];
    if (arg.starts_with("--config=")) {
      config_file = arg.substr(9);
      break;
    }
  }

  // Load config from file if specified
  if (!config_file.empty()) {
    config = load_config_from_toml(config_file);
  }

  // Second pass: override with command-line options
  for (int i = 1; i < argc; ++i) {
    std::string arg = argv[i];

    if (arg == "--help" || arg == "-h") {
      print_usage(argv[0]);
      std::exit(0);
    } else if (arg.starts_with("--config=")) {
      // Already handled in first pass
      continue;
    } else if (arg == "--inline") {
      if (!polling_specified) {
        config.run_inline = false;
        config.run_threaded = false;
        polling_specified = true;
      }
      config.run_inline = true;
    } else if (arg == "--threaded") {
      if (!polling_specified) {
        config.run_inline = false;
        config.run_threaded = false;
        polling_specified = true;
      }
      config.run_threaded = true;
    } else if (arg == "--async-scope") {
      if (!pattern_specified) {
        config.run_async_scope = false;
        config.run_scoped_workers = false;
        pattern_specified = true;
      }
      config.run_async_scope = true;
    } else if (arg == "--scoped-workers") {
      if (!pattern_specified) {
        config.run_async_scope = false;
        config.run_scoped_workers = false;
        pattern_specified = true;
      }
      config.run_scoped_workers = true;
    } else if (arg.starts_with("--queue=")) {
      if (!queue_specified) {
        config.run_nolock = false;
        config.run_mutex = false;
        config.run_tas = false;
        config.run_ttas = false;
        config.run_backoff = false;
        config.run_lockfree = false;
        queue_specified = true;
      }
      std::string queues = arg.substr(8);
      size_t pos = 0;
      while (pos < queues.size()) {
        size_t end = queues.find(',', pos);
        if (end == std::string::npos) end = queues.size();
        std::string q = queues.substr(pos, end - pos);
        if (q == "nolock") config.run_nolock = true;
        else if (q == "mutex") config.run_mutex = true;
        else if (q == "tas") config.run_tas = true;
        else if (q == "ttas") config.run_ttas = true;
        else if (q == "backoff") config.run_backoff = true;
        else if (q == "lockfree") config.run_lockfree = true;
        else {
          fmt::println(stderr, "Unknown queue type: {}", q);
          std::exit(1);
        }
        pos = end + 1;
      }
    } else {
      fmt::println(stderr, "Unknown option: {}", arg);
      print_usage(argv[0]);
      std::exit(1);
    }
  }

  return config;
}

void benchmark_queues_with_dsa(const BenchmarkConfig &config) {
  fmt::println("=== DSA BENCHMARK WITH DIFFERENT TASK QUEUES ===\n");

  const std::vector<size_t> &batch_sizes = config.batch_sizes;
  const std::vector<size_t> &msg_sizes = config.msg_sizes;
  const size_t total_bytes_target = config.total_bytes;

  // Results organized by [pattern][polling_mode]
  // pattern: 0 = async_scope, 1 = scoped_workers
  // polling: 0 = inline, 1 = threaded
  std::vector<BenchmarkResult> async_scope_inline_results;
  std::vector<BenchmarkResult> async_scope_threaded_results;
  std::vector<BenchmarkResult> scoped_workers_inline_results;
  std::vector<BenchmarkResult> scoped_workers_threaded_results;

  std::vector<char> src(total_bytes_target);
  std::vector<char> dst(total_bytes_target);
  std::memset(src.data(), 1, total_bytes_target);
  std::memset(dst.data(), 0, total_bytes_target);

  // async_scope pattern + inline polling
  if (config.run_async_scope && config.run_inline) {
    fmt::println("Running async_scope + inline polling...");
    for (auto bs : batch_sizes) {
      for (auto ms : msg_sizes) {
        size_t batch_bytes = bs * ms;
        if (batch_bytes > 2ULL * 1024 * 1024 * 1024)
          continue;

        int iterations = static_cast<int>(total_bytes_target / batch_bytes);
        if (iterations < 1) iterations = 1;

        BenchmarkResult result{bs, ms, {}, {}, {}, {}, {}, {}};

        if (config.run_nolock) {
          exec::async_scope scope;
          DsaSingleThread dsa(false);
          result.single_thread = benchmark_scope_inline(
              dsa, scope, bs, ms, src, dst, iterations);
        }
        if (config.run_mutex) {
          exec::async_scope scope;
          Dsa dsa(false);
          result.mutex = benchmark_scope_inline(
              dsa, scope, bs, ms, src, dst, iterations);
        }
        if (config.run_tas) {
          exec::async_scope scope;
          DsaTasSpinlock dsa(false);
          result.tas_spinlock = benchmark_scope_inline(
              dsa, scope, bs, ms, src, dst, iterations);
        }
        if (config.run_ttas) {
          exec::async_scope scope;
          DsaSpinlock dsa(false);
          result.ttas_spinlock = benchmark_scope_inline(
              dsa, scope, bs, ms, src, dst, iterations);
        }
        if (config.run_backoff) {
          exec::async_scope scope;
          DsaBackoffSpinlock dsa(false);
          result.backoff_spinlock = benchmark_scope_inline(
              dsa, scope, bs, ms, src, dst, iterations);
        }
        if (config.run_lockfree) {
          exec::async_scope scope;
          DsaLockFree dsa(false);
          result.lockfree = benchmark_scope_inline(
              dsa, scope, bs, ms, src, dst, iterations);
        }
        async_scope_inline_results.push_back(result);
        fmt::println("  Batch {:>2}, Size {:>7}: done", bs, ms);
      }
    }
    fmt::println("");
  }

  // async_scope pattern + threaded polling
  if (config.run_async_scope && config.run_threaded) {
    fmt::println("Running async_scope + threaded polling...");
    for (auto bs : batch_sizes) {
      for (auto ms : msg_sizes) {
        size_t batch_bytes = bs * ms;
        if (batch_bytes > 2ULL * 1024 * 1024 * 1024)
          continue;

        int iterations = static_cast<int>(total_bytes_target / batch_bytes);
        if (iterations < 1) iterations = 1;

        BenchmarkResult result{bs, ms, {-1, 0, {}}, {}, {}, {}, {}, {}};

        if (config.run_mutex) {
          exec::async_scope scope;
          Dsa dsa(true);
          result.mutex = benchmark_scope_threaded(
              dsa, scope, bs, ms, src, dst, iterations);
        }
        if (config.run_tas) {
          exec::async_scope scope;
          DsaTasSpinlock dsa(true);
          result.tas_spinlock = benchmark_scope_threaded(
              dsa, scope, bs, ms, src, dst, iterations);
        }
        if (config.run_ttas) {
          exec::async_scope scope;
          DsaSpinlock dsa(true);
          result.ttas_spinlock = benchmark_scope_threaded(
              dsa, scope, bs, ms, src, dst, iterations);
        }
        if (config.run_backoff) {
          exec::async_scope scope;
          DsaBackoffSpinlock dsa(true);
          result.backoff_spinlock = benchmark_scope_threaded(
              dsa, scope, bs, ms, src, dst, iterations);
        }
        if (config.run_lockfree) {
          exec::async_scope scope;
          DsaLockFree dsa(true);
          result.lockfree = benchmark_scope_threaded(
              dsa, scope, bs, ms, src, dst, iterations);
        }

        async_scope_threaded_results.push_back(result);
        fmt::println("  Batch {:>2}, Size {:>7}: done", bs, ms);
      }
    }
    fmt::println("");
  }

  // scoped_workers pattern + inline polling
  if (config.run_scoped_workers && config.run_inline) {
    fmt::println("Running scoped_workers + inline polling...");
    for (auto bs : batch_sizes) {
      for (auto ms : msg_sizes) {
        size_t batch_bytes = bs * ms;
        if (batch_bytes > 2ULL * 1024 * 1024 * 1024)
          continue;

        int iterations = static_cast<int>(total_bytes_target / batch_bytes);
        if (iterations < 1) iterations = 1;

        BenchmarkResult result{bs, ms, {}, {}, {}, {}, {}, {}};

        if (config.run_nolock) {
          exec::async_scope scope;
          DsaSingleThread dsa(false);
          result.single_thread = benchmark_scoped_workers_inline(
              dsa, scope, bs, ms, src, dst, iterations);
        }
        if (config.run_mutex) {
          exec::async_scope scope;
          Dsa dsa(false);
          result.mutex = benchmark_scoped_workers_inline(
              dsa, scope, bs, ms, src, dst, iterations);
        }
        if (config.run_tas) {
          exec::async_scope scope;
          DsaTasSpinlock dsa(false);
          result.tas_spinlock = benchmark_scoped_workers_inline(
              dsa, scope, bs, ms, src, dst, iterations);
        }
        if (config.run_ttas) {
          exec::async_scope scope;
          DsaSpinlock dsa(false);
          result.ttas_spinlock = benchmark_scoped_workers_inline(
              dsa, scope, bs, ms, src, dst, iterations);
        }
        if (config.run_backoff) {
          exec::async_scope scope;
          DsaBackoffSpinlock dsa(false);
          result.backoff_spinlock = benchmark_scoped_workers_inline(
              dsa, scope, bs, ms, src, dst, iterations);
        }
        if (config.run_lockfree) {
          exec::async_scope scope;
          DsaLockFree dsa(false);
          result.lockfree = benchmark_scoped_workers_inline(
              dsa, scope, bs, ms, src, dst, iterations);
        }
        scoped_workers_inline_results.push_back(result);
        fmt::println("  Batch {:>2}, Size {:>7}: done", bs, ms);
      }
    }
    fmt::println("");
  }

  // scoped_workers pattern + threaded polling
  if (config.run_scoped_workers && config.run_threaded) {
    fmt::println("Running scoped_workers + threaded polling...");
    for (auto bs : batch_sizes) {
      for (auto ms : msg_sizes) {
        size_t batch_bytes = bs * ms;
        if (batch_bytes > 2ULL * 1024 * 1024 * 1024)
          continue;

        int iterations = static_cast<int>(total_bytes_target / batch_bytes);
        if (iterations < 1) iterations = 1;

        BenchmarkResult result{bs, ms, {-1, 0, {}}, {}, {}, {}, {}, {}};

        if (config.run_mutex) {
          exec::async_scope scope;
          Dsa dsa(true);
          result.mutex = benchmark_scoped_workers_threaded(
              dsa, scope, bs, ms, src, dst, iterations);
        }
        if (config.run_tas) {
          exec::async_scope scope;
          DsaTasSpinlock dsa(true);
          result.tas_spinlock = benchmark_scoped_workers_threaded(
              dsa, scope, bs, ms, src, dst, iterations);
        }
        if (config.run_ttas) {
          exec::async_scope scope;
          DsaSpinlock dsa(true);
          result.ttas_spinlock = benchmark_scoped_workers_threaded(
              dsa, scope, bs, ms, src, dst, iterations);
        }
        if (config.run_backoff) {
          exec::async_scope scope;
          DsaBackoffSpinlock dsa(true);
          result.backoff_spinlock = benchmark_scoped_workers_threaded(
              dsa, scope, bs, ms, src, dst, iterations);
        }
        if (config.run_lockfree) {
          exec::async_scope scope;
          DsaLockFree dsa(true);
          result.lockfree = benchmark_scoped_workers_threaded(
              dsa, scope, bs, ms, src, dst, iterations);
        }

        scoped_workers_threaded_results.push_back(result);
        fmt::println("  Batch {:>2}, Size {:>7}: done", bs, ms);
      }
    }
    fmt::println("");
  }

  // Print results tables
  fmt::println("==============================================================="
               "=================");
  fmt::println("                              BENCHMARK RESULTS");
  fmt::println("==============================================================="
               "=================\n");

  // Helper lambda to print a results table
  auto print_results_table = [](const char* title,
                                 const std::vector<BenchmarkResult>& results,
                                 bool include_nolock) {
    if (results.empty()) return;

    fmt::println("========== {} ==========\n", title);
    if (include_nolock) {
      fmt::println("{:>5} {:>10} {:>16} {:>16} {:>16} {:>16} {:>16} {:>16}",
                   "Batch", "Size", "NoLock", "Mutex", "TAS", "TTAS", "Backoff",
                   "LockFree");
      fmt::println(
          "{:-^5} {:-^10} {:-^16} {:-^16} {:-^16} {:-^16} {:-^16} {:-^16}",
          "", "", "", "", "", "", "", "");
      for (const auto &r : results) {
        fmt::println(
            "{:>5} {:>10} {:>16} {:>16} {:>16} {:>16} {:>16} {:>16}",
            r.batch_size, r.msg_size, format_metric(r.single_thread),
            format_metric(r.mutex), format_metric(r.tas_spinlock),
            format_metric(r.ttas_spinlock), format_metric(r.backoff_spinlock),
            format_metric(r.lockfree));
      }
    } else {
      fmt::println("{:>5} {:>10} {:>16} {:>16} {:>16} {:>16} {:>16}",
                   "Batch", "Size", "Mutex", "TAS", "TTAS", "Backoff", "LockFree");
      fmt::println("{:-^5} {:-^10} {:-^16} {:-^16} {:-^16} {:-^16} {:-^16}",
                   "", "", "", "", "", "", "");
      for (const auto &r : results) {
        fmt::println("{:>5} {:>10} {:>16} {:>16} {:>16} {:>16} {:>16}",
                     r.batch_size, r.msg_size, format_metric(r.mutex),
                     format_metric(r.tas_spinlock), format_metric(r.ttas_spinlock),
                     format_metric(r.backoff_spinlock), format_metric(r.lockfree));
      }
    }
    fmt::println("");
  };

  print_results_table("ASYNC_SCOPE + INLINE", async_scope_inline_results, true);
  print_results_table("ASYNC_SCOPE + THREADED", async_scope_threaded_results, false);
  print_results_table("SCOPED_WORKERS + INLINE", scoped_workers_inline_results, true);
  print_results_table("SCOPED_WORKERS + THREADED", scoped_workers_threaded_results, false);

  // Export results to CSV
  export_to_csv(config.csv_file,
                async_scope_inline_results, async_scope_threaded_results,
                scoped_workers_inline_results, scoped_workers_threaded_results);
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
