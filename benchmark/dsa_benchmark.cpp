#include <algorithm>
#include <chrono>
#include <cstdlib>
#include <cstring>
#include <dsa/dsa.hpp>
#include <dsa/task_queue.hpp>
#include <dsa_stdexec/data_move.hpp>
#include <dsa_stdexec/run_loop.hpp>
#include <dsa_stdexec/sync_wait.hpp>
#include <dsa_stdexec/trace.hpp>
#include <exec/async_scope.hpp>
#include <fmt/base.h>
#include <fmt/core.h>
#include <fmt/ranges.h>
#include <fstream>
#include <functional>
#include <mutex>
#include <numeric>
#include <stdexec/execution.hpp>
#include <thread>
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

// Dynamic batch with inline polling (with latency measurement using then)
// base_offset is the starting offset for this iteration's batch
template <typename DsaType>
void run_dynamic_batch_inline(DsaType &dsa, exec::async_scope &scope,
                              size_t batch_size, size_t msg_size,
                              std::vector<char> &src, std::vector<char> &dst,
                              size_t base_offset, LatencyCollector &latency) {
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });
  for (size_t i = 0; i < batch_size; ++i) {
    size_t offset = base_offset + i * msg_size;
    auto start_time = std::chrono::high_resolution_clock::now();
    auto snd = dsa_stdexec::dsa_data_move(dsa, src.data() + offset,
                                          dst.data() + offset, msg_size)
             | stdexec::then([&latency, start_time]() {
                 auto end_time = std::chrono::high_resolution_clock::now();
                 auto duration = std::chrono::duration<double, std::nano>(
                     end_time - start_time).count();
                 latency.record(duration);
               });
    scope.spawn(std::move(snd));
  }
  dsa_stdexec::wait_start(scope.on_empty(), loop);
}

// Dynamic batch with background thread polling (with latency measurement using then)
// base_offset is the starting offset for this iteration's batch
template <typename DsaType>
void run_dynamic_batch_threaded(DsaType &dsa, exec::async_scope &scope,
                                size_t batch_size, size_t msg_size,
                                std::vector<char> &src,
                                std::vector<char> &dst,
                                size_t base_offset, LatencyCollector &latency) {
  for (size_t i = 0; i < batch_size; ++i) {
    size_t offset = base_offset + i * msg_size;
    auto start_time = std::chrono::high_resolution_clock::now();
    auto snd = dsa_stdexec::dsa_data_move(dsa, src.data() + offset,
                                          dst.data() + offset, msg_size)
             | stdexec::then([&latency, start_time]() {
                 auto end_time = std::chrono::high_resolution_clock::now();
                 auto duration = std::chrono::duration<double, std::nano>(
                     end_time - start_time).count();
                 latency.record(duration);
               });
    scope.spawn(std::move(snd));
  }
  stdexec::sync_wait(scope.on_empty());
}

struct DsaMetric {
  double bandwidth;
  uint64_t page_faults;
  LatencyCollector::Stats latency;
};

// Track UUIDs for each queue type (fixed so they appear as separate rows)
enum class QueueTrackId : uint64_t {
  SingleThread = 1000,
  Mutex = 1001,
  TasSpinlock = 1002,
  TtasSpinlock = 1003,
  BackoffSpinlock = 1004,
  LockFree = 1005,
  RingBuffer = 1006,
};

// Initialize tracks upfront so they appear vertically aligned in Perfetto
inline void init_benchmark_tracks() {
  auto register_track = [](QueueTrackId id, const char* name) {
    auto track = perfetto::Track(static_cast<uint64_t>(id));
    auto desc = track.Serialize();
    desc.set_name(name);
    perfetto::TrackEvent::SetTrackDescriptor(track, desc);
  };

  register_track(QueueTrackId::SingleThread, "SingleThread (NoLock)");
  register_track(QueueTrackId::Mutex, "Mutex");
  register_track(QueueTrackId::TasSpinlock, "TAS Spinlock");
  register_track(QueueTrackId::TtasSpinlock, "TTAS Spinlock");
  register_track(QueueTrackId::BackoffSpinlock, "Backoff Spinlock");
  register_track(QueueTrackId::LockFree, "Lock-Free");
  register_track(QueueTrackId::RingBuffer, "Ring Buffer");
}

// Benchmark DSA dynamic batch with inline polling, returns bandwidth, page
// faults, and latency stats
template <typename DsaType>
DsaMetric benchmark_dynamic_inline(DsaType &dsa, exec::async_scope &scope,
                                   size_t batch_size, size_t msg_size,
                                   std::vector<char> &src,
                                   std::vector<char> &dst, int iterations,
                                   QueueTrackId track_id) {
  size_t batch_bytes = batch_size * msg_size;
  auto track = perfetto::Track(static_cast<uint64_t>(track_id));
  LatencyCollector warmup_latency;
  LatencyCollector latency;

  // Create slice name with batch/msg info
  std::string slice_name = fmt::format("b{}×{}B", batch_size, msg_size);

  TRACE_EVENT_BEGIN("dsa", "Warmup", track);
  run_dynamic_batch_inline(dsa, scope, batch_size, msg_size, src, dst, 0, warmup_latency);
  TRACE_EVENT_END("dsa", track);

  dsa_stdexec::reset_page_fault_retries();

  auto start = std::chrono::high_resolution_clock::now();
  TRACE_EVENT_BEGIN("dsa", perfetto::DynamicString(slice_name), track,
                    "batch_size", batch_size, "msg_size", msg_size,
                    "mode", "inline");
  for (int i = 0; i < iterations; ++i) {
    size_t base_offset = static_cast<size_t>(i) * batch_bytes;
    TRACE_EVENT_BEGIN("dsa", "Iteration", track, "i", i);
    run_dynamic_batch_inline(dsa, scope, batch_size, msg_size, src, dst, base_offset, latency);
    TRACE_EVENT_END("dsa", track);
  }
  TRACE_EVENT_END("dsa", track);
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
DsaMetric benchmark_dynamic_threaded(DsaType &dsa, exec::async_scope &scope,
                                     size_t batch_size, size_t msg_size,
                                     std::vector<char> &src,
                                     std::vector<char> &dst, int iterations,
                                     QueueTrackId track_id) {
  size_t batch_bytes = batch_size * msg_size;
  auto track = perfetto::Track(static_cast<uint64_t>(track_id));
  LatencyCollector warmup_latency;
  LatencyCollector latency;

  // Create slice name with batch/msg info
  std::string slice_name = fmt::format("b{}×{}B", batch_size, msg_size);

  TRACE_EVENT_BEGIN("dsa", "Warmup", track);
  run_dynamic_batch_threaded(dsa, scope, batch_size, msg_size, src, dst, 0, warmup_latency);
  TRACE_EVENT_END("dsa", track);

  dsa_stdexec::reset_page_fault_retries();

  auto start = std::chrono::high_resolution_clock::now();
  TRACE_EVENT_BEGIN("dsa", perfetto::DynamicString(slice_name), track,
                    "batch_size", batch_size, "msg_size", msg_size,
                    "mode", "threaded");
  for (int i = 0; i < iterations; ++i) {
    size_t base_offset = static_cast<size_t>(i) * batch_bytes;
    TRACE_EVENT_BEGIN("dsa", "Iteration", track, "i", i);
    run_dynamic_batch_threaded(dsa, scope, batch_size, msg_size, src, dst, base_offset, latency);
    TRACE_EVENT_END("dsa", track);
  }
  TRACE_EVENT_END("dsa", track);
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
  DsaMetric ringbuffer;
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
                   const std::vector<BenchmarkResult> &inline_results,
                   const std::vector<BenchmarkResult> &threaded_results) {
  std::ofstream file(filename);
  if (!file.is_open()) {
    fmt::println(stderr, "Failed to open {} for writing", filename);
    return;
  }

  // Write CSV header
  file << "mode,queue_type,batch_size,msg_size,bandwidth_gbps,page_faults,"
       << "latency_min_ns,latency_max_ns,latency_avg_ns,latency_p50_ns,latency_p99_ns,latency_count\n";

  // Helper to write one metric row
  auto write_row = [&file](const char *mode, const char *queue_type,
                            size_t batch_size, size_t msg_size,
                            const DsaMetric &m) {
    file << mode << "," << queue_type << "," << batch_size << "," << msg_size
         << "," << m.bandwidth << "," << m.page_faults << ","
         << m.latency.min_ns << "," << m.latency.max_ns << ","
         << m.latency.avg_ns << "," << m.latency.p50_ns << ","
         << m.latency.p99_ns << "," << m.latency.count << "\n";
  };

  // Write inline polling results
  for (const auto &r : inline_results) {
    write_row("inline", "NoLock", r.batch_size, r.msg_size, r.single_thread);
    write_row("inline", "Mutex", r.batch_size, r.msg_size, r.mutex);
    write_row("inline", "TAS", r.batch_size, r.msg_size, r.tas_spinlock);
    write_row("inline", "TTAS", r.batch_size, r.msg_size, r.ttas_spinlock);
    write_row("inline", "Backoff", r.batch_size, r.msg_size, r.backoff_spinlock);
    write_row("inline", "LockFree", r.batch_size, r.msg_size, r.lockfree);
    write_row("inline", "RingBuffer", r.batch_size, r.msg_size, r.ringbuffer);
  }

  // Write threaded polling results
  for (const auto &r : threaded_results) {
    write_row("threaded", "Mutex", r.batch_size, r.msg_size, r.mutex);
    write_row("threaded", "TAS", r.batch_size, r.msg_size, r.tas_spinlock);
    write_row("threaded", "TTAS", r.batch_size, r.msg_size, r.ttas_spinlock);
    write_row("threaded", "Backoff", r.batch_size, r.msg_size, r.backoff_spinlock);
    write_row("threaded", "LockFree", r.batch_size, r.msg_size, r.lockfree);
    write_row("threaded", "RingBuffer", r.batch_size, r.msg_size, r.ringbuffer);
  }

  file.close();
  fmt::println("Results exported to {}", filename);
}

void benchmark_queues_with_dsa() {
  fmt::println("=== DSA BENCHMARK WITH DIFFERENT TASK QUEUES ===\n");

  // Initialize tracks upfront for vertical alignment in Perfetto
  init_benchmark_tracks();

  std::vector<size_t> batch_sizes = {1, 4, 16, 32};
  std::vector<size_t> msg_sizes = {512,  1024,      2048,
                                   4096, 64 * 1024, 1024 * 1024};
  constexpr size_t total_bytes_target = 32ULL * 1024 * 1024;  // 512MB total data to copy

  std::vector<BenchmarkResult> inline_results;
  std::vector<BenchmarkResult> threaded_results;

  // Allocate buffers at total size once (reused across all benchmarks)
  std::vector<char> src(total_bytes_target);
  std::vector<char> dst(total_bytes_target);
  std::memset(src.data(), 1, total_bytes_target);
  std::memset(dst.data(), 0, total_bytes_target);

  // Collect inline polling results
  fmt::println("Running inline polling benchmarks...");
  for (auto bs : batch_sizes) {
    for (auto ms : msg_sizes) {
      size_t batch_bytes = bs * ms;
      if (batch_bytes > 2ULL * 1024 * 1024 * 1024)
        continue;

      int iterations = static_cast<int>(total_bytes_target / batch_bytes);
      if (iterations < 1) iterations = 1;

      BenchmarkResult result{bs, ms, {}, {}, {}, {}, {}, {}, {}};

      {
        exec::async_scope scope;
        DsaSingleThread dsa(false);
        result.single_thread = benchmark_dynamic_inline(
            dsa, scope, bs, ms, src, dst, iterations, QueueTrackId::SingleThread);
      }
      {
        exec::async_scope scope;
        Dsa dsa(false);
        result.mutex = benchmark_dynamic_inline(
            dsa, scope, bs, ms, src, dst, iterations, QueueTrackId::Mutex);
      }
      {
        exec::async_scope scope;
        DsaTasSpinlock dsa(false);
        result.tas_spinlock = benchmark_dynamic_inline(
            dsa, scope, bs, ms, src, dst, iterations, QueueTrackId::TasSpinlock);
      }
      {
        exec::async_scope scope;
        DsaSpinlock dsa(false);
        result.ttas_spinlock = benchmark_dynamic_inline(
            dsa, scope, bs, ms, src, dst, iterations, QueueTrackId::TtasSpinlock);
      }
      {
        exec::async_scope scope;
        DsaBackoffSpinlock dsa(false);
        result.backoff_spinlock = benchmark_dynamic_inline(
            dsa, scope, bs, ms, src, dst, iterations, QueueTrackId::BackoffSpinlock);
      }
      {
        exec::async_scope scope;
        DsaLockFree dsa(false);
        result.lockfree = benchmark_dynamic_inline(
            dsa, scope, bs, ms, src, dst, iterations, QueueTrackId::LockFree);
      }
      {
        exec::async_scope scope;
        DsaRingBuffer dsa(false);
        result.ringbuffer = benchmark_dynamic_inline(
            dsa, scope, bs, ms, src, dst, iterations, QueueTrackId::RingBuffer);
      }

      inline_results.push_back(result);
      fmt::println("  Batch {:>2}, Size {:>7}: done", bs, ms);
    }
  }
  fmt::println("");

  // Collect threaded polling results
  fmt::println("Running background thread polling benchmarks...");
  for (auto bs : batch_sizes) {
    for (auto ms : msg_sizes) {
      size_t batch_bytes = bs * ms;
      if (batch_bytes > 2ULL * 1024 * 1024 * 1024)
        continue;

      int iterations = static_cast<int>(total_bytes_target / batch_bytes);
      if (iterations < 1) iterations = 1;

      BenchmarkResult result{bs, ms, {-1, 0, {}}, {}, {}, {}, {}, {}, {}};

      {
        exec::async_scope scope;
        Dsa dsa(true);
        result.mutex = benchmark_dynamic_threaded(
            dsa, scope, bs, ms, src, dst, iterations, QueueTrackId::Mutex);
      }
      {
        exec::async_scope scope;
        DsaTasSpinlock dsa(true);
        result.tas_spinlock = benchmark_dynamic_threaded(
            dsa, scope, bs, ms, src, dst, iterations, QueueTrackId::TasSpinlock);
      }
      {
        exec::async_scope scope;
        DsaSpinlock dsa(true);
        result.ttas_spinlock = benchmark_dynamic_threaded(
            dsa, scope, bs, ms, src, dst, iterations, QueueTrackId::TtasSpinlock);
      }
      {
        exec::async_scope scope;
        DsaBackoffSpinlock dsa(true);
        result.backoff_spinlock = benchmark_dynamic_threaded(
            dsa, scope, bs, ms, src, dst, iterations, QueueTrackId::BackoffSpinlock);
      }
      {
        exec::async_scope scope;
        DsaLockFree dsa(true);
        result.lockfree = benchmark_dynamic_threaded(
            dsa, scope, bs, ms, src, dst, iterations, QueueTrackId::LockFree);
      }
      {
        exec::async_scope scope;
        DsaRingBuffer dsa(true);
        result.ringbuffer = benchmark_dynamic_threaded(
            dsa, scope, bs, ms, src, dst, iterations, QueueTrackId::RingBuffer);
      }

      threaded_results.push_back(result);
      fmt::println("  Batch {:>2}, Size {:>7}: done", bs, ms);
    }
  }
  fmt::println("");

  // Print results tables
  fmt::println("==============================================================="
               "=================");
  fmt::println("                              BENCHMARK RESULTS");
  fmt::println("==============================================================="
               "=================\n");

  // Inline polling
  fmt::println("========== INLINE POLLING ==========\n");
  fmt::println("{:>5} {:>10} {:>16} {:>16} {:>16} {:>16} {:>16} {:>16} {:>16}",
               "Batch", "Size", "NoLock", "Mutex", "TAS", "TTAS", "Backoff",
               "LockFree", "RingBuf");
  fmt::println(
      "{:-^5} {:-^10} {:-^16} {:-^16} {:-^16} {:-^16} {:-^16} {:-^16} {:-^16}",
      "", "", "", "", "", "", "", "", "");
  for (const auto &r : inline_results) {
    fmt::println(
        "{:>5} {:>10} {:>16} {:>16} {:>16} {:>16} {:>16} {:>16} {:>16}",
        r.batch_size, r.msg_size, format_metric(r.single_thread),
        format_metric(r.mutex), format_metric(r.tas_spinlock),
        format_metric(r.ttas_spinlock), format_metric(r.backoff_spinlock),
        format_metric(r.lockfree), format_metric(r.ringbuffer));
  }
  fmt::println("");

  // Threaded polling
  fmt::println("========== BACKGROUND THREAD POLLING ==========\n");
  fmt::println("{:>5} {:>10} {:>16} {:>16} {:>16} {:>16} {:>16} {:>16}",
               "Batch", "Size", "Mutex", "TAS", "TTAS", "Backoff", "LockFree",
               "RingBuf");
  fmt::println("{:-^5} {:-^10} {:-^16} {:-^16} {:-^16} {:-^16} {:-^16} {:-^16}",
               "", "", "", "", "", "", "", "");
  for (const auto &r : threaded_results) {
    fmt::println("{:>5} {:>10} {:>16} {:>16} {:>16} {:>16} {:>16} {:>16}",
                 r.batch_size, r.msg_size, format_metric(r.mutex),
                 format_metric(r.tas_spinlock), format_metric(r.ttas_spinlock),
                 format_metric(r.backoff_spinlock), format_metric(r.lockfree),
                 format_metric(r.ringbuffer));
  }
  fmt::println("");

  // Export results to CSV
  export_to_csv("dsa_benchmark_results.csv", inline_results, threaded_results);
}

int main(int argc, char **argv) {
  ::init_tracing();
  std::system("stty opost onlcr");
  try {
    benchmark_queues_with_dsa();

    fmt::println("");
    fmt::println("Benchmark completed.");

  } catch (const std::exception &e) {
    fmt::println(stderr, "Error: {}", e.what());
    perfetto::TrackEvent::Flush();
    perfetto::Tracing::ActivateTriggers({"app_finished"}, 0);
    std::this_thread::sleep_for(std::chrono::seconds(1));
    return 1;
  }

  // Flush pending track events and fire trigger to stop tracing
  perfetto::TrackEvent::Flush();
  perfetto::Tracing::ActivateTriggers({"app_finished"}, 0);
  fmt::println("Trigger 'app_finished' sent.");

  // Wait for perfetto to process trigger and save trace
  std::this_thread::sleep_for(std::chrono::seconds(1));
  return 0;
}
