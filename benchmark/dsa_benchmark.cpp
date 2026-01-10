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
#include <functional>
#include <stdexec/execution.hpp>
#include <thread>
#include <utility>
#include <vector>

// Dynamic batch with inline polling
// base_offset is the starting offset for this iteration's batch
template <typename DsaType>
void run_dynamic_batch_inline(DsaType &dsa, exec::async_scope &scope,
                              size_t batch_size, size_t msg_size,
                              std::vector<char> &src, std::vector<char> &dst,
                              size_t base_offset) {
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });
  for (size_t i = 0; i < batch_size; ++i) {
    size_t offset = base_offset + i * msg_size;
    auto snd = dsa_stdexec::dsa_data_move(dsa, src.data() + offset,
                                          dst.data() + offset, msg_size);
    scope.spawn(snd);
  }
  dsa_stdexec::wait_start(scope.on_empty(), loop);
}

// Dynamic batch with background thread polling
// base_offset is the starting offset for this iteration's batch
template <typename DsaType>
void run_dynamic_batch_threaded(DsaType &dsa, exec::async_scope &scope,
                                size_t batch_size, size_t msg_size,
                                std::vector<char> &src,
                                std::vector<char> &dst,
                                size_t base_offset) {
  for (size_t i = 0; i < batch_size; ++i) {
    size_t offset = base_offset + i * msg_size;
    auto snd = dsa_stdexec::dsa_data_move(dsa, src.data() + offset,
                                          dst.data() + offset, msg_size);
    scope.spawn(snd);
  }
  stdexec::sync_wait(scope.on_empty());
}

struct DsaMetric {
  double bandwidth;
  uint64_t page_faults;
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

// Benchmark DSA dynamic batch with inline polling, returns bandwidth and page
// faults
template <typename DsaType>
DsaMetric benchmark_dynamic_inline(DsaType &dsa, exec::async_scope &scope,
                                   size_t batch_size, size_t msg_size,
                                   std::vector<char> &src,
                                   std::vector<char> &dst, int iterations,
                                   QueueTrackId track_id) {
  size_t batch_bytes = batch_size * msg_size;
  auto track = perfetto::Track(static_cast<uint64_t>(track_id));

  // Create slice name with batch/msg info
  std::string slice_name = fmt::format("b{}×{}B", batch_size, msg_size);

  TRACE_EVENT_BEGIN("dsa", "Warmup", track);
  run_dynamic_batch_inline(dsa, scope, batch_size, msg_size, src, dst, 0);
  TRACE_EVENT_END("dsa", track);

  dsa_stdexec::reset_page_fault_retries();

  auto start = std::chrono::high_resolution_clock::now();
  TRACE_EVENT_BEGIN("dsa", perfetto::DynamicString(slice_name), track,
                    "batch_size", batch_size, "msg_size", msg_size,
                    "mode", "inline");
  for (int i = 0; i < iterations; ++i) {
    size_t base_offset = static_cast<size_t>(i) * batch_bytes;
    TRACE_EVENT_BEGIN("dsa", "Iteration", track, "i", i);
    run_dynamic_batch_inline(dsa, scope, batch_size, msg_size, src, dst, base_offset);
    TRACE_EVENT_END("dsa", track);
  }
  TRACE_EVENT_END("dsa", track);
  auto end = std::chrono::high_resolution_clock::now();

  uint64_t page_faults = dsa_stdexec::get_page_fault_retries();
  std::chrono::duration<double> diff = end - start;
  double bw = (double)batch_bytes * iterations / (1024.0 * 1024.0 * 1024.0) /
              diff.count();
  return {bw, page_faults};
}

// Benchmark DSA dynamic batch with background thread polling, returns bandwidth
// and page faults
template <typename DsaType>
DsaMetric benchmark_dynamic_threaded(DsaType &dsa, exec::async_scope &scope,
                                     size_t batch_size, size_t msg_size,
                                     std::vector<char> &src,
                                     std::vector<char> &dst, int iterations,
                                     QueueTrackId track_id) {
  size_t batch_bytes = batch_size * msg_size;
  auto track = perfetto::Track(static_cast<uint64_t>(track_id));

  // Create slice name with batch/msg info
  std::string slice_name = fmt::format("b{}×{}B", batch_size, msg_size);

  TRACE_EVENT_BEGIN("dsa", "Warmup", track);
  run_dynamic_batch_threaded(dsa, scope, batch_size, msg_size, src, dst, 0);
  TRACE_EVENT_END("dsa", track);

  dsa_stdexec::reset_page_fault_retries();

  auto start = std::chrono::high_resolution_clock::now();
  TRACE_EVENT_BEGIN("dsa", perfetto::DynamicString(slice_name), track,
                    "batch_size", batch_size, "msg_size", msg_size,
                    "mode", "threaded");
  for (int i = 0; i < iterations; ++i) {
    size_t base_offset = static_cast<size_t>(i) * batch_bytes;
    TRACE_EVENT_BEGIN("dsa", "Iteration", track, "i", i);
    run_dynamic_batch_threaded(dsa, scope, batch_size, msg_size, src, dst, base_offset);
    TRACE_EVENT_END("dsa", track);
  }
  TRACE_EVENT_END("dsa", track);
  auto end = std::chrono::high_resolution_clock::now();

  uint64_t page_faults = dsa_stdexec::get_page_fault_retries();
  std::chrono::duration<double> diff = end - start;
  double bw = (double)batch_bytes * iterations / (1024.0 * 1024.0 * 1024.0) /
              diff.count();
  return {bw, page_faults};
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

      BenchmarkResult result{bs, ms, {-1, 0}, {}, {}, {}, {}, {}, {}};

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
