#include <algorithm>
#include <chrono>
#include <cstdio>
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
#include <exec/task.hpp>
#include <fmt/base.h>
#include <fmt/core.h>
#include <fmt/ranges.h>
#include <fstream>
#include <functional>
#include <memory>

#include <numeric>
#include <stdexec/execution.hpp>
#include <thread>
#include <toml++/toml.hpp>
#include <unistd.h>
#include <utility>
#include <vector>

// Progress bar with time-based throttling to minimize performance impact
class ProgressBar {
public:
  ProgressBar(size_t total, std::string_view label = "")
      : total_(total), current_(0), label_(label), bar_width_(30) {
    is_tty_ = isatty(STDERR_FILENO);
    last_update_ = std::chrono::steady_clock::now();
  }

  void set_label(std::string_view label) { label_ = label; }

  void update(size_t current) {
    current_ = current;
    auto now = std::chrono::steady_clock::now();
    auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(now - last_update_);
    // Throttle updates to max ~10Hz to minimize overhead
    if (elapsed.count() >= 100 || current >= total_) {
      render();
      last_update_ = now;
    }
  }

  void increment() { update(current_ + 1); }

  void finish() {
    current_ = total_;
    render();
    if (is_tty_) {
      fmt::print(stderr, "\n");
    }
  }

private:
  void render() {
    if (!is_tty_) return;

    double pct = total_ > 0 ? static_cast<double>(current_) / total_ : 0.0;
    size_t filled = static_cast<size_t>(pct * bar_width_);

    std::string bar(filled, '=');
    if (filled < bar_width_) {
      bar += '>';
      bar += std::string(bar_width_ - filled - 1, ' ');
    }

    fmt::print(stderr, "\r\033[K{} [{}] {:3.0f}% ({}/{})",
               label_, bar, pct * 100, current_, total_);
    std::fflush(stderr);
  }

  size_t total_;
  size_t current_;
  std::string label_;
  size_t bar_width_;
  bool is_tty_;
  std::chrono::steady_clock::time_point last_update_;
};

// Latency collector (single-threaded, no locking needed)
class LatencyCollector {
public:
  void record(double latency_ns) {
    samples_.push_back(latency_ns);
  }

  void clear() {
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
  std::vector<double> samples_;
};

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


struct DsaMetric {
  double bandwidth;      // GB/s
  double msg_rate;       // Million messages/second
  uint64_t page_faults;
  LatencyCollector::Stats latency;
};

// Generic benchmark runner using a run function
// RunFn signature: void(DsaType&, async_scope&, concurrency, msg_size, total_bytes, src, dst, latency)
template <typename DsaType, typename RunFn>
DsaMetric run_benchmark(DsaType &dsa, size_t concurrency, size_t msg_size,
                        size_t total_bytes, int iterations,
                        std::vector<char> &src, std::vector<char> &dst,
                        RunFn run_fn, ProgressBar *progress = nullptr) {
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


struct BenchmarkResult {
  size_t concurrency;
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

// Benchmark configuration from command-line options or TOML file
struct BenchmarkConfig {
  // Polling mode dimension
  bool run_inline = true;
  bool run_threaded = true;

  // Scheduling pattern dimension
  bool run_sliding_window = true;  // Semaphore-like: spawn new op as one completes
  bool run_batch = false;          // Spawn N ops, wait all, repeat
  bool run_scoped_workers = false; // N workers processing sequentially (N allocations)

  // Queue type dimension
  bool run_nolock = true;
  bool run_mutex = true;
  bool run_tas = true;
  bool run_ttas = true;
  bool run_backoff = true;
  bool run_lockfree = true;

  // Benchmark parameters
  std::vector<size_t> concurrency_levels = {1, 4, 16, 32};  // Max operations in-flight
  std::vector<size_t> msg_sizes = {256, 512, 1024, 2048, 4096, 8192, 16384};
  size_t total_bytes = 32ULL * 1024 * 1024;  // Total bytes to copy per iteration
  int iterations = 10;  // Number of times to repeat the full copy

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
    config.run_sliding_window = scheduling->get("sliding_window")->value_or(true);
    config.run_batch = scheduling->get("batch")->value_or(false);
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
    if (auto arr = params->get("concurrency_levels")->as_array()) {
      config.concurrency_levels.clear();
      for (const auto &elem : *arr) {
        if (auto val = elem.value<int64_t>()) {
          config.concurrency_levels.push_back(static_cast<size_t>(*val));
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
    if (auto val = params->get("iterations")->value<int64_t>()) {
      config.iterations = static_cast<int>(*val);
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
  fmt::println("  --sliding-window    Semaphore-like: spawn new op as one completes (default)");
  fmt::println("  --batch             Spawn N ops, wait all complete, repeat");
  fmt::println("  --scoped-workers    N worker coroutines processing sequentially");
  fmt::println("");
  fmt::println("Queue types:");
  fmt::println("  --queue=<type>      Run only specified queue type(s), comma-separated");
  fmt::println("                      Types: nolock, mutex, tas, ttas, backoff, lockfree");
  fmt::println("");
  fmt::println("Examples:");
  fmt::println("  {}                                  # Default: sliding-window with inline+threaded", prog);
  fmt::println("  {} --config=benchmark_config.toml   # Load from TOML config file", prog);
  fmt::println("  {} --inline                         # Only inline polling", prog);
  fmt::println("  {} --threaded                       # Only background thread polling", prog);
  fmt::println("  {} --batch                          # Only batch pattern", prog);
  fmt::println("  {} --scoped-workers --inline        # Scoped workers with inline only", prog);
  fmt::println("  {} --sliding-window --batch         # Both sliding-window and batch", prog);
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
    } else if (arg == "--sliding-window") {
      if (!pattern_specified) {
        config.run_sliding_window = false;
        config.run_batch = false;
        config.run_scoped_workers = false;
        pattern_specified = true;
      }
      config.run_sliding_window = true;
    } else if (arg == "--batch") {
      if (!pattern_specified) {
        config.run_sliding_window = false;
        config.run_batch = false;
        config.run_scoped_workers = false;
        pattern_specified = true;
      }
      config.run_batch = true;
    } else if (arg == "--scoped-workers") {
      if (!pattern_specified) {
        config.run_sliding_window = false;
        config.run_batch = false;
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

// Helper to run benchmarks for all queue types with a given run function
template <typename RunFn>
std::vector<BenchmarkResult> run_all_queues(
    const BenchmarkConfig &config,
    std::vector<char> &src, std::vector<char> &dst,
    bool use_threaded_polling,
    RunFn run_fn,
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

  for (auto concurrency : config.concurrency_levels) {
    for (auto msg_size : config.msg_sizes) {
      BenchmarkResult result{concurrency, msg_size, {}, {}, {}, {}, {}, {}};

      progress.set_label(fmt::format("{} c={} sz={}", pattern_name, concurrency, msg_size));

      if (!use_threaded_polling && config.run_nolock) {
        DsaSingleThread dsa(false);
        result.single_thread = run_benchmark(dsa, concurrency, msg_size,
            config.total_bytes, config.iterations, src, dst, run_fn, &progress);
      }
      if (config.run_mutex) {
        Dsa dsa(use_threaded_polling);
        result.mutex = run_benchmark(dsa, concurrency, msg_size,
            config.total_bytes, config.iterations, src, dst, run_fn, &progress);
      }
      if (config.run_tas) {
        DsaTasSpinlock dsa(use_threaded_polling);
        result.tas_spinlock = run_benchmark(dsa, concurrency, msg_size,
            config.total_bytes, config.iterations, src, dst, run_fn, &progress);
      }
      if (config.run_ttas) {
        DsaSpinlock dsa(use_threaded_polling);
        result.ttas_spinlock = run_benchmark(dsa, concurrency, msg_size,
            config.total_bytes, config.iterations, src, dst, run_fn, &progress);
      }
      if (config.run_backoff) {
        DsaBackoffSpinlock dsa(use_threaded_polling);
        result.backoff_spinlock = run_benchmark(dsa, concurrency, msg_size,
            config.total_bytes, config.iterations, src, dst, run_fn, &progress);
      }
      if (config.run_lockfree) {
        DsaLockFree dsa(use_threaded_polling);
        result.lockfree = run_benchmark(dsa, concurrency, msg_size,
            config.total_bytes, config.iterations, src, dst, run_fn, &progress);
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
        [](auto &dsa, auto &scope, size_t c, size_t m, size_t t, auto &s, auto &d, auto &l) {
          run_sliding_window_inline(dsa, scope, c, m, t, s, d, l);
        }, "sliding_window");
    all_results.emplace_back("sliding_window_inline", std::move(results));
    fmt::println("");
  }

  if (config.run_sliding_window && config.run_threaded) {
    fmt::println("Running sliding_window + threaded polling...");
    auto results = run_all_queues(config, src, dst, true,
        [](auto &dsa, auto &scope, size_t c, size_t m, size_t t, auto &s, auto &d, auto &l) {
          run_sliding_window_threaded(dsa, scope, c, m, t, s, d, l);
        }, "sliding_window");
    all_results.emplace_back("sliding_window_threaded", std::move(results));
    fmt::println("");
  }

  // Batch pattern
  if (config.run_batch && config.run_inline) {
    fmt::println("Running batch + inline polling...");
    auto results = run_all_queues(config, src, dst, false,
        [](auto &dsa, auto &scope, size_t c, size_t m, size_t t, auto &s, auto &d, auto &l) {
          run_batch_inline(dsa, scope, c, m, t, s, d, l);
        }, "batch");
    all_results.emplace_back("batch_inline", std::move(results));
    fmt::println("");
  }

  if (config.run_batch && config.run_threaded) {
    fmt::println("Running batch + threaded polling...");
    auto results = run_all_queues(config, src, dst, true,
        [](auto &dsa, auto &scope, size_t c, size_t m, size_t t, auto &s, auto &d, auto &l) {
          run_batch_threaded(dsa, scope, c, m, t, s, d, l);
        }, "batch");
    all_results.emplace_back("batch_threaded", std::move(results));
    fmt::println("");
  }

  // Scoped workers pattern
  if (config.run_scoped_workers && config.run_inline) {
    fmt::println("Running scoped_workers + inline polling...");
    auto results = run_all_queues(config, src, dst, false,
        [](auto &dsa, auto &scope, size_t c, size_t m, size_t t, auto &s, auto &d, auto &l) {
          run_scoped_workers_inline(dsa, scope, c, m, t, s, d, l);
        }, "scoped_workers");
    all_results.emplace_back("scoped_workers_inline", std::move(results));
    fmt::println("");
  }

  if (config.run_scoped_workers && config.run_threaded) {
    fmt::println("Running scoped_workers + threaded polling...");
    auto results = run_all_queues(config, src, dst, true,
        [](auto &dsa, auto &scope, size_t c, size_t m, size_t t, auto &s, auto &d, auto &l) {
          run_scoped_workers_threaded(dsa, scope, c, m, t, s, d, l);
        }, "scoped_workers");
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
