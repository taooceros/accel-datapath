#include <atomic>
#include <chrono>
#include <cstddef>
#include <fmt/core.h>
#include <thread>
#include <vector>

#include <dsa/mock_dsa.hpp>
#include <dsa/task_queue.hpp>
#include <dsa_stdexec/operation_base.hpp>

// Benchmark operation that wraps MockOperation for use with OperationBase
class BenchmarkOperation : public MockOperation {
public:
  using MockOperation::MockOperation;

  void init_base() {
    base.proxy = pro::make_proxy<dsa_stdexec::OperationFacade>(Wrapper{this});
  }

  dsa_stdexec::OperationBase base;

private:
  struct Wrapper {
    BenchmarkOperation *op;
    bool check_completion() { return op->MockOperation::check_completion(); }
    void notify() { op->MockOperation::notify(); }
    dsa_hw_desc *get_descriptor() { return op->MockOperation::get_descriptor(); }
  };
};

struct BenchmarkResult {
  double ops_per_sec;
  double avg_latency_ns;
  std::size_t total_ops;
};

// Benchmark single-threaded push/poll throughput
template <template <typename> class QueueTemplate>
BenchmarkResult benchmark_single_thread(std::size_t num_ops) {
  MockDsaBase<QueueTemplate> mock_dsa(false);
  std::vector<BenchmarkOperation> ops(num_ops);
  for (auto &op : ops) {
    op.init_base();
  }

  auto start = std::chrono::high_resolution_clock::now();

  // Push all operations
  for (auto &op : ops) {
    mock_dsa.submit(&op.base);
  }

  // Poll until all complete
  std::size_t completed = 0;
  while (completed < num_ops) {
    completed += mock_dsa.poll();
  }

  auto end = std::chrono::high_resolution_clock::now();
  auto duration = std::chrono::duration<double, std::nano>(end - start).count();

  return {static_cast<double>(num_ops) / (duration / 1e9),
          duration / static_cast<double>(num_ops), num_ops};
}

// Benchmark with multiple producer threads and single poller
template <template <typename> class QueueTemplate>
BenchmarkResult benchmark_multi_producer(std::size_t num_ops,
                                         std::size_t num_threads) {
  MockDsaBase<QueueTemplate> mock_dsa(false);
  std::vector<BenchmarkOperation> ops(num_ops);
  for (auto &op : ops) {
    op.init_base();
  }
  std::atomic<std::size_t> push_index{0};
  std::atomic<std::size_t> completed{0};

  auto start = std::chrono::high_resolution_clock::now();

  // Start producer threads
  std::vector<std::thread> producers;
  for (std::size_t t = 0; t < num_threads; ++t) {
    producers.emplace_back([&] {
      while (true) {
        std::size_t idx = push_index.fetch_add(1, std::memory_order_relaxed);
        if (idx >= num_ops) {
          break;
        }
        mock_dsa.submit(&ops[idx].base);
      }
    });
  }

  // Poll in main thread until all complete
  while (completed.load(std::memory_order_relaxed) < num_ops) {
    std::size_t n = mock_dsa.poll();
    completed.fetch_add(n, std::memory_order_relaxed);
  }

  for (auto &t : producers) {
    t.join();
  }

  auto end = std::chrono::high_resolution_clock::now();
  auto duration = std::chrono::duration<double, std::nano>(end - start).count();

  return {static_cast<double>(num_ops) / (duration / 1e9),
          duration / static_cast<double>(num_ops), num_ops};
}

// Benchmark with background poller thread
template <template <typename> class QueueTemplate>
BenchmarkResult benchmark_background_poller(std::size_t num_ops,
                                            std::size_t num_threads) {
  MockDsaBase<QueueTemplate> mock_dsa(true); // Start background poller
  std::vector<BenchmarkOperation> ops(num_ops);
  for (auto &op : ops) {
    op.init_base();
  }
  std::atomic<std::size_t> push_index{0};
  std::atomic<std::size_t> notified{0};

  // Set callbacks to count notifications
  for (auto &op : ops) {
    op.set_callback(
        [&notified] { notified.fetch_add(1, std::memory_order_relaxed); });
  }

  auto start = std::chrono::high_resolution_clock::now();

  // Start producer threads
  std::vector<std::thread> producers;
  for (std::size_t t = 0; t < num_threads; ++t) {
    producers.emplace_back([&] {
      while (true) {
        std::size_t idx = push_index.fetch_add(1, std::memory_order_relaxed);
        if (idx >= num_ops) {
          break;
        }
        mock_dsa.submit(&ops[idx].base);
      }
    });
  }

  for (auto &t : producers) {
    t.join();
  }

  // Wait for all notifications
  while (notified.load(std::memory_order_relaxed) < num_ops) {
    std::this_thread::yield();
  }

  auto end = std::chrono::high_resolution_clock::now();
  auto duration = std::chrono::duration<double, std::nano>(end - start).count();

  return {static_cast<double>(num_ops) / (duration / 1e9),
          duration / static_cast<double>(num_ops), num_ops};
}

void print_result(const char *name, const BenchmarkResult &r) {
  fmt::println("  {:20} {:>12.2f} Mops/s  {:>10.1f} ns/op", name,
               r.ops_per_sec / 1e6, r.avg_latency_ns);
}

int main() {
  constexpr std::size_t NUM_OPS = 1000000;
  constexpr std::size_t NUM_THREADS = 4;

  fmt::println("=== TASK QUEUE BENCHMARK (Mock DSA - No Hardware) ===\n");
  fmt::println("Operations: {}, Threads: {}\n", NUM_OPS, NUM_THREADS);

  // Single-threaded benchmark
  fmt::println("--- Single-Threaded Push/Poll ---");
  print_result("NoLock",
               benchmark_single_thread<dsa::SingleThreadTaskQueue>(NUM_OPS));
  print_result("Mutex",
               benchmark_single_thread<dsa::MutexTaskQueue>(NUM_OPS));
  print_result("TAS Spinlock",
               benchmark_single_thread<dsa::TasSpinlockTaskQueue>(NUM_OPS));
  print_result("TTAS Spinlock",
               benchmark_single_thread<dsa::SpinlockTaskQueue>(NUM_OPS));
  print_result("Backoff Spinlock",
               benchmark_single_thread<dsa::BackoffSpinlockTaskQueue>(NUM_OPS));
  print_result("Lock-Free",
               benchmark_single_thread<dsa::LockFreeTaskQueue>(NUM_OPS));
  fmt::println("");

  // Multi-producer, single consumer (inline poll)
  fmt::println("--- Multi-Producer, Inline Poll ({} threads) ---", NUM_THREADS);
  print_result("Mutex",
               benchmark_multi_producer<dsa::MutexTaskQueue>(NUM_OPS, NUM_THREADS));
  print_result("TAS Spinlock",
               benchmark_multi_producer<dsa::TasSpinlockTaskQueue>(NUM_OPS, NUM_THREADS));
  print_result("TTAS Spinlock",
               benchmark_multi_producer<dsa::SpinlockTaskQueue>(NUM_OPS, NUM_THREADS));
  print_result("Backoff Spinlock",
               benchmark_multi_producer<dsa::BackoffSpinlockTaskQueue>(NUM_OPS, NUM_THREADS));
  print_result("Lock-Free",
               benchmark_multi_producer<dsa::LockFreeTaskQueue>(NUM_OPS, NUM_THREADS));
  fmt::println("");

  // Multi-producer with background poller
  fmt::println("--- Multi-Producer, Background Poller ({} threads) ---",
               NUM_THREADS);
  print_result("Mutex",
               benchmark_background_poller<dsa::MutexTaskQueue>(NUM_OPS, NUM_THREADS));
  print_result("TAS Spinlock",
               benchmark_background_poller<dsa::TasSpinlockTaskQueue>(NUM_OPS, NUM_THREADS));
  print_result("TTAS Spinlock",
               benchmark_background_poller<dsa::SpinlockTaskQueue>(NUM_OPS, NUM_THREADS));
  print_result("Backoff Spinlock",
               benchmark_background_poller<dsa::BackoffSpinlockTaskQueue>(NUM_OPS, NUM_THREADS));
  print_result("Lock-Free",
               benchmark_background_poller<dsa::LockFreeTaskQueue>(NUM_OPS, NUM_THREADS));
  fmt::println("");

  fmt::println("Benchmark completed.");
  return 0;
}
