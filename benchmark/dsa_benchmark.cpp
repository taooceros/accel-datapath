#include <chrono>
#include <cstdlib>
#include <cstring>
#include <dsa/dsa.hpp>
#include <dsa/task_queue.hpp>
#include <dsa_stdexec/data_move.hpp>
#include <dsa_stdexec/run_loop.hpp>
#include <dsa_stdexec/sync_wait.hpp>
#include <exec/async_scope.hpp>
#include <fmt/base.h>
#include <fmt/core.h>
#include <fmt/ranges.h>
#include <stdexec/execution.hpp>
#include <utility>
#include <vector>
#include <thread>

// Dynamic batch with inline polling
template <typename DsaType>
void run_dynamic_batch_inline(DsaType &dsa, size_t batch_size, size_t msg_size,
                              std::vector<char> &src, std::vector<char> &dst) {
  dsa_stdexec::PollingRunLoop loop([&dsa] { dsa.poll(); });
  exec::async_scope scope;
  for (size_t i = 0; i < batch_size; ++i) {
    auto snd = dsa_stdexec::dsa_data_move(dsa, src.data() + i * msg_size,
                                          dst.data() + i * msg_size, msg_size);
    scope.spawn(snd);
  }
  dsa_stdexec::wait_start(scope.on_empty(), loop);
  loop.reset();
}

// Dynamic batch with background thread polling
template <typename DsaType>
void run_dynamic_batch_threaded(DsaType &dsa, size_t batch_size, size_t msg_size,
                                std::vector<char> &src, std::vector<char> &dst) {
  exec::async_scope scope;
  for (size_t i = 0; i < batch_size; ++i) {
    auto snd = dsa_stdexec::dsa_data_move(dsa, src.data() + i * msg_size,
                                          dst.data() + i * msg_size, msg_size);
    scope.spawn(snd);
  }
  stdexec::sync_wait(scope.on_empty());
}

// Benchmark DSA dynamic batch with inline polling
template <typename DsaType>
double benchmark_dynamic_inline(DsaType &dsa, size_t batch_size, size_t msg_size,
                                std::vector<char> &src, std::vector<char> &dst,
                                int iterations) {
  size_t total_size = batch_size * msg_size;

  run_dynamic_batch_inline(dsa, batch_size, msg_size, src, dst); // Warmup

  auto start = std::chrono::high_resolution_clock::now();
  for (int i = 0; i < iterations; ++i) {
    run_dynamic_batch_inline(dsa, batch_size, msg_size, src, dst);
  }
  auto end = std::chrono::high_resolution_clock::now();

  std::chrono::duration<double> diff = end - start;
  return (double)total_size * iterations / (1024.0 * 1024.0 * 1024.0) / diff.count();
}

// Benchmark DSA dynamic batch with background thread polling
template <typename DsaType>
double benchmark_dynamic_threaded(DsaType &dsa, size_t batch_size, size_t msg_size,
                                  std::vector<char> &src, std::vector<char> &dst,
                                  int iterations) {
  size_t total_size = batch_size * msg_size;

  run_dynamic_batch_threaded(dsa, batch_size, msg_size, src, dst); // Warmup

  auto start = std::chrono::high_resolution_clock::now();
  for (int i = 0; i < iterations; ++i) {
    run_dynamic_batch_threaded(dsa, batch_size, msg_size, src, dst);
  }
  auto end = std::chrono::high_resolution_clock::now();

  std::chrono::duration<double> diff = end - start;
  return (double)total_size * iterations / (1024.0 * 1024.0 * 1024.0) / diff.count();
}

struct BenchmarkResult {
  size_t batch_size;
  size_t msg_size;
  double single_thread_bw;
  double mutex_bw;
  double tas_spinlock_bw;
  double ttas_spinlock_bw;
  double backoff_spinlock_bw;
  double lockfree_bw;
};

void benchmark_queues_with_dsa() {
  fmt::println("=== DSA BENCHMARK WITH DIFFERENT TASK QUEUES ===\n");

  std::vector<size_t> batch_sizes = {1, 4, 16, 32};
  std::vector<size_t> msg_sizes = {4096, 64 * 1024, 1024 * 1024};
  constexpr int iterations = 100;

  std::vector<BenchmarkResult> inline_results;
  std::vector<BenchmarkResult> threaded_results;

  // Collect inline polling results
  fmt::println("Running inline polling benchmarks...");
  for (auto bs : batch_sizes) {
    for (auto ms : msg_sizes) {
      size_t total_size = bs * ms;
      if (total_size > 2ULL * 1024 * 1024 * 1024) continue;

      std::vector<char> src(total_size);
      std::vector<char> dst(total_size);
      std::memset(src.data(), 1, total_size);
      std::memset(dst.data(), 0, total_size);

      BenchmarkResult result{bs, ms, 0, 0, 0, 0, 0, 0};

      { DsaSingleThread dsa(false); result.single_thread_bw = benchmark_dynamic_inline(dsa, bs, ms, src, dst, iterations); }
      { Dsa dsa(false); result.mutex_bw = benchmark_dynamic_inline(dsa, bs, ms, src, dst, iterations); }
      { DsaTasSpinlock dsa(false); result.tas_spinlock_bw = benchmark_dynamic_inline(dsa, bs, ms, src, dst, iterations); }
      { DsaSpinlock dsa(false); result.ttas_spinlock_bw = benchmark_dynamic_inline(dsa, bs, ms, src, dst, iterations); }
      { DsaBackoffSpinlock dsa(false); result.backoff_spinlock_bw = benchmark_dynamic_inline(dsa, bs, ms, src, dst, iterations); }
      { DsaLockFree dsa(false); result.lockfree_bw = benchmark_dynamic_inline(dsa, bs, ms, src, dst, iterations); }

      inline_results.push_back(result);
      fmt::println("  Batch {:>2}, Size {:>7}: done", bs, ms);
    }
  }
  fmt::println("");

  // Collect threaded polling results
  fmt::println("Running background thread polling benchmarks...");
  for (auto bs : batch_sizes) {
    for (auto ms : msg_sizes) {
      size_t total_size = bs * ms;
      if (total_size > 2ULL * 1024 * 1024 * 1024) continue;

      std::vector<char> src(total_size);
      std::vector<char> dst(total_size);
      std::memset(src.data(), 1, total_size);
      std::memset(dst.data(), 0, total_size);

      BenchmarkResult result{bs, ms, -1, 0, 0, 0, 0, 0};

      { Dsa dsa(true); result.mutex_bw = benchmark_dynamic_threaded(dsa, bs, ms, src, dst, iterations); }
      { DsaTasSpinlock dsa(true); result.tas_spinlock_bw = benchmark_dynamic_threaded(dsa, bs, ms, src, dst, iterations); }
      { DsaSpinlock dsa(true); result.ttas_spinlock_bw = benchmark_dynamic_threaded(dsa, bs, ms, src, dst, iterations); }
      { DsaBackoffSpinlock dsa(true); result.backoff_spinlock_bw = benchmark_dynamic_threaded(dsa, bs, ms, src, dst, iterations); }
      { DsaLockFree dsa(true); result.lockfree_bw = benchmark_dynamic_threaded(dsa, bs, ms, src, dst, iterations); }

      threaded_results.push_back(result);
      fmt::println("  Batch {:>2}, Size {:>7}: done", bs, ms);
    }
  }
  fmt::println("");

  // Print results tables
  fmt::println("================================================================================");
  fmt::println("                              BENCHMARK RESULTS");
  fmt::println("================================================================================\n");

  // Inline polling
  fmt::println("========== INLINE POLLING ==========\n");
  fmt::println("{:>5} {:>10} | {:>8} | {:>8} | {:>8} | {:>8} | {:>8} | {:>8}",
               "Batch", "Size", "NoLock", "Mutex", "TAS", "TTAS", "Backoff", "LockFree");
  fmt::println("{:-^5}-{:-^10}-+-{:-^8}-+-{:-^8}-+-{:-^8}-+-{:-^8}-+-{:-^8}-+-{:-^8}",
               "", "", "", "", "", "", "", "");
  for (const auto &r : inline_results) {
    fmt::println("{:>5} {:>10} | {:>5.2f} GB | {:>5.2f} GB | {:>5.2f} GB | {:>5.2f} GB | {:>5.2f} GB | {:>5.2f} GB",
                 r.batch_size, r.msg_size, r.single_thread_bw,
                 r.mutex_bw, r.tas_spinlock_bw, r.ttas_spinlock_bw, r.backoff_spinlock_bw, r.lockfree_bw);
  }
  fmt::println("");

  // Threaded polling
  fmt::println("========== BACKGROUND THREAD POLLING ==========\n");
  fmt::println("{:>5} {:>10} | {:>8} | {:>8} | {:>8} | {:>8} | {:>8}",
               "Batch", "Size", "Mutex", "TAS", "TTAS", "Backoff", "LockFree");
  fmt::println("{:-^5}-{:-^10}-+-{:-^8}-+-{:-^8}-+-{:-^8}-+-{:-^8}-+-{:-^8}",
               "", "", "", "", "", "", "");
  for (const auto &r : threaded_results) {
    fmt::println("{:>5} {:>10} | {:>5.2f} GB | {:>5.2f} GB | {:>5.2f} GB | {:>5.2f} GB | {:>5.2f} GB",
                 r.batch_size, r.msg_size,
                 r.mutex_bw, r.tas_spinlock_bw, r.ttas_spinlock_bw, r.backoff_spinlock_bw, r.lockfree_bw);
  }
}

int main(int argc, char **argv) {
  std::system("stty opost onlcr");
  try {
    benchmark_queues_with_dsa();

    fmt::println("");
    fmt::println("Benchmark completed.");
  } catch (const std::exception &e) {
    fmt::println(stderr, "Error: {}", e.what());
    return 1;
  }

  return 0;
}
