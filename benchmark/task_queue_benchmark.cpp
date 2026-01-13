#include <atomic>
#include <chrono>
#include <cstddef>
#include <fmt/core.h>
#include <functional>
#include <numbers>
#include <stdexec/execution.hpp>
#include <exec/async_scope.hpp>
#include <thread>
#include <vector>

#include <dsa/mock_dsa.hpp>
#include <dsa_stdexec/operation_base.hpp>
#include <dsa_stdexec/run_loop.hpp>
#include <dsa_stdexec/sync_wait.hpp>

struct BenchmarkResult {
  double ops_per_sec;
  double avg_latency_ns;
  std::size_t total_ops;
};

void print_result(const char *name, const BenchmarkResult &r) {
  fmt::println("  {:20} {:>12.2f} Mops/s  {:>10.1f} ns/op", name,
               r.ops_per_sec / 1e6, r.avg_latency_ns);
}

// Mock DSA Scheduler - works with MockDsaBase instead of real Dsa
template <template <typename> class QueueTemplate>
class MockDsaScheduler;

template <template <typename> class QueueTemplate, class ReceiverId>
class MockScheduleOperation : public dsa_stdexec::OperationBase {
  using Receiver = stdexec::__t<ReceiverId>;

public:
  using operation_state_concept = stdexec::operation_state_t;

  struct Wrapper {
    MockScheduleOperation *op;
    bool check_completion() { return op->check_completion(); }
    void notify() { op->notify(); }
    dsa_hw_desc *get_descriptor() { return nullptr; }
  };

  MockScheduleOperation(MockDsaBase<QueueTemplate> &dsa, Receiver r)
      : dsa_(dsa), r_(std::move(r)) {
    this->proxy = pro::make_proxy<dsa_stdexec::OperationFacade>(Wrapper{this});
  }

  MockScheduleOperation(MockScheduleOperation &&other) noexcept
      : dsa_stdexec::OperationBase(), dsa_(other.dsa_), r_(std::move(other.r_)) {
    this->proxy = pro::make_proxy<dsa_stdexec::OperationFacade>(Wrapper{this});
  }

  void start() noexcept { dsa_.submit(this); }

  bool check_completion() { return true; }

  void notify() { stdexec::set_value(std::move(r_)); }

private:
  MockDsaBase<QueueTemplate> &dsa_;
  Receiver r_;
};

template <template <typename> class QueueTemplate>
class MockScheduleSender {
public:
  using sender_concept = stdexec::sender_t;
  using completion_signatures =
      stdexec::completion_signatures<stdexec::set_value_t(),
                                     stdexec::set_error_t(std::exception_ptr)>;

  explicit MockScheduleSender(MockDsaBase<QueueTemplate> &dsa) : dsa_(dsa) {}

  template <stdexec::receiver Receiver>
  auto connect(Receiver &&r) && {
    return MockScheduleOperation<QueueTemplate, stdexec::__id<Receiver>>(
        dsa_, std::forward<Receiver>(r));
  }

  template <stdexec::receiver Receiver>
  auto connect(Receiver &&r) const & {
    return MockScheduleOperation<QueueTemplate, stdexec::__id<Receiver>>(
        dsa_, std::forward<Receiver>(r));
  }

private:
  MockDsaBase<QueueTemplate> &dsa_;
};

template <template <typename> class QueueTemplate>
class MockDsaScheduler {
public:
  using scheduler_concept = stdexec::scheduler_t;
  explicit MockDsaScheduler(MockDsaBase<QueueTemplate> &dsa) : dsa_(dsa) {}

  MockScheduleSender<QueueTemplate> schedule() const noexcept {
    return MockScheduleSender<QueueTemplate>(dsa_);
  }

  bool operator==(const MockDsaScheduler &other) const noexcept {
    return &dsa_ == &other.dsa_;
  }

private:
  MockDsaBase<QueueTemplate> &dsa_;
};

// Benchmark scheduling operations through stdexec scheduler interface
// Uses async_scope to track all spawned work
template <template <typename> class QueueTemplate>
BenchmarkResult benchmark_stdexec_schedule(std::size_t num_ops) {
  MockDsaBase<QueueTemplate> mock_dsa(false);
  MockDsaScheduler<QueueTemplate> scheduler(mock_dsa);


  exec::async_scope scope;
  std::atomic<std::size_t> completed{0};

  auto start = std::chrono::high_resolution_clock::now();

  // Spawn all operations using async_scope
  for (std::size_t i = 0; i < num_ops; ++i) {
    scope.spawn(
        scheduler.schedule()
        | stdexec::then([&completed] {
            completed.fetch_add(1, std::memory_order_relaxed);
          }));
  }

  // Poll until all complete
  while (completed.load(std::memory_order_relaxed) < num_ops) {
    mock_dsa.poll();
  }

  // Wait for scope to be empty (should be immediate since all work completed)
  dsa_stdexec::PollingRunLoop loop([&mock_dsa] { mock_dsa.poll(); });
  dsa_stdexec::wait_start(scope.on_empty(), loop);

  auto end = std::chrono::high_resolution_clock::now();
  auto duration = std::chrono::duration<double, std::nano>(end - start).count();

  return {static_cast<double>(num_ops) / (duration / 1e9),
          duration / static_cast<double>(num_ops), num_ops};
}

// Benchmark using PollingRunLoop with schedule operations
template <template <typename> class QueueTemplate>
BenchmarkResult benchmark_polling_run_loop(std::size_t num_ops) {
  MockDsaBase<QueueTemplate> mock_dsa(false);
  std::atomic<std::size_t> completed{0};

  // Create a polling run loop that polls the mock DSA
  dsa_stdexec::PollingRunLoop loop([&mock_dsa] { mock_dsa.poll(); });

  exec::async_scope scope;

  auto start = std::chrono::high_resolution_clock::now();

  // Spawn work on the run loop scheduler
  for (std::size_t i = 0; i < num_ops; ++i) {
    scope.spawn(
        loop.get_scheduler().schedule()
        | stdexec::then([&completed] {
            completed.fetch_add(1, std::memory_order_relaxed);
          }));
  }

  // Wait for all work to complete
  dsa_stdexec::wait_start(scope.on_empty(), loop);

  auto end = std::chrono::high_resolution_clock::now();
  auto duration = std::chrono::duration<double, std::nano>(end - start).count();

  return {static_cast<double>(num_ops) / (duration / 1e9),
          duration / static_cast<double>(num_ops), num_ops};
}

// Benchmark chained/composed stdexec operations (schedule | then | then)
template <template <typename> class QueueTemplate>
BenchmarkResult benchmark_stdexec_then_chain(std::size_t num_ops) {
  MockDsaBase<QueueTemplate> mock_dsa(false);
  MockDsaScheduler<QueueTemplate> scheduler(mock_dsa);

  exec::async_scope scope;
  std::atomic<std::size_t> completed{0};
  std::atomic<std::size_t> work_done{0};

  auto start = std::chrono::high_resolution_clock::now();

  // Spawn chained operations
  for (std::size_t i = 0; i < num_ops; ++i) {
    scope.spawn(
        scheduler.schedule()
        | stdexec::then([&work_done] {
            work_done.fetch_add(1, std::memory_order_relaxed);
            return 42;
          })
        | stdexec::then([](int x) { return x * 2; })
        | stdexec::then([&completed](int) {
            completed.fetch_add(1, std::memory_order_relaxed);
          }));
  }

  // Poll until all complete
  while (completed.load(std::memory_order_relaxed) < num_ops) {
    mock_dsa.poll();
  }

  // Wait for scope to be empty
  dsa_stdexec::PollingRunLoop loop([&mock_dsa] { mock_dsa.poll(); });
  dsa_stdexec::wait_start(scope.on_empty(), loop);

  auto end = std::chrono::high_resolution_clock::now();
  auto duration = std::chrono::duration<double, std::nano>(end - start).count();

  return {static_cast<double>(num_ops) / (duration / 1e9),
          duration / static_cast<double>(num_ops), num_ops};
}

// Benchmark multi-threaded stdexec scheduling
template <template <typename> class QueueTemplate>
BenchmarkResult benchmark_stdexec_multi_producer(std::size_t num_ops,
                                                  std::size_t num_threads) {
  MockDsaBase<QueueTemplate> mock_dsa(false);
  MockDsaScheduler<QueueTemplate> scheduler(mock_dsa);

  exec::async_scope scope;
  std::atomic<std::size_t> op_index{0};
  std::atomic<std::size_t> completed{0};

  auto start = std::chrono::high_resolution_clock::now();

  // Producer threads
  std::vector<std::thread> producers;
  for (std::size_t t = 0; t < num_threads; ++t) {
    producers.emplace_back([&] {
      while (true) {
        std::size_t idx = op_index.fetch_add(1, std::memory_order_relaxed);
        if (idx >= num_ops) {
          break;
        }
        scope.spawn(
            scheduler.schedule()
            | stdexec::then([&completed] {
                completed.fetch_add(1, std::memory_order_relaxed);
              }));
      }
    });
  }

  // Poll in main thread until all complete
  while (completed.load(std::memory_order_relaxed) < num_ops) {
    mock_dsa.poll();
  }

  for (auto &t : producers) {
    t.join();
  }

  // Wait for scope to be empty
  dsa_stdexec::PollingRunLoop loop([&mock_dsa] { mock_dsa.poll(); });
  dsa_stdexec::wait_start(scope.on_empty(), loop);

  auto end = std::chrono::high_resolution_clock::now();
  auto duration = std::chrono::duration<double, std::nano>(end - start).count();

  return {static_cast<double>(num_ops) / (duration / 1e9),
          duration / static_cast<double>(num_ops), num_ops};
}

// Benchmark with background poller using stdexec interface
template <template <typename> class QueueTemplate>
BenchmarkResult benchmark_stdexec_background_poller(std::size_t num_ops,
                                                     std::size_t num_threads) {
  MockDsaBase<QueueTemplate> mock_dsa(true); // Start background poller
  MockDsaScheduler<QueueTemplate> scheduler(mock_dsa);

  exec::async_scope scope;
  std::atomic<std::size_t> op_index{0};
  std::atomic<std::size_t> completed{0};

  auto start = std::chrono::high_resolution_clock::now();

  // Producer threads
  std::vector<std::thread> producers;
  for (std::size_t t = 0; t < num_threads; ++t) {
    producers.emplace_back([&] {
      while (true) {
        std::size_t idx = op_index.fetch_add(1, std::memory_order_relaxed);
        if (idx >= num_ops) {
          break;
        }
        scope.spawn(
            scheduler.schedule()
            | stdexec::then([&completed] {
                completed.fetch_add(1, std::memory_order_relaxed);
              }));
      }
    });
  }

  for (auto &t : producers) {
    t.join();
  }

  // Wait for all notifications (background poller handles completion)
  while (completed.load(std::memory_order_relaxed) < num_ops) {
    std::this_thread::yield();
  }

  // Wait for scope to be empty
  dsa_stdexec::PollingRunLoop loop([&mock_dsa] { mock_dsa.poll(); });
  dsa_stdexec::wait_start(scope.on_empty(), loop);

  auto end = std::chrono::high_resolution_clock::now();
  auto duration = std::chrono::duration<double, std::nano>(end - start).count();

  return {static_cast<double>(num_ops) / (duration / 1e9),
          duration / static_cast<double>(num_ops), num_ops};
}

int main() {
  constexpr std::size_t NUM_OPS = 1000000;
  constexpr std::size_t NUM_THREADS = 4;
  constexpr std::size_t WARMUP_OPS = 10000;

  fmt::println("=== TASK QUEUE BENCHMARK (stdexec Interface) ===\n");
  fmt::println("Operations: {}, Threads: {}\n", NUM_OPS, NUM_THREADS);

  // Warmup phase
  fmt::println("--- Warmup ({} ops each) ---", WARMUP_OPS);
  benchmark_stdexec_schedule<dsa::MutexTaskQueue>(WARMUP_OPS);
  benchmark_stdexec_schedule<dsa::TasSpinlockTaskQueue>(WARMUP_OPS);
  benchmark_stdexec_schedule<dsa::SpinlockTaskQueue>(WARMUP_OPS);
  benchmark_stdexec_schedule<dsa::BackoffSpinlockTaskQueue>(WARMUP_OPS);
  benchmark_stdexec_schedule<dsa::LockFreeTaskQueue>(WARMUP_OPS);
  benchmark_polling_run_loop<dsa::MutexTaskQueue>(WARMUP_OPS);
  benchmark_polling_run_loop<dsa::LockFreeTaskQueue>(WARMUP_OPS);
  benchmark_stdexec_then_chain<dsa::MutexTaskQueue>(WARMUP_OPS);
  benchmark_stdexec_then_chain<dsa::LockFreeTaskQueue>(WARMUP_OPS);
  benchmark_stdexec_multi_producer<dsa::MutexTaskQueue>(WARMUP_OPS, NUM_THREADS);
  benchmark_stdexec_multi_producer<dsa::LockFreeTaskQueue>(WARMUP_OPS, NUM_THREADS);
  benchmark_stdexec_background_poller<dsa::MutexTaskQueue>(WARMUP_OPS, NUM_THREADS);
  benchmark_stdexec_background_poller<dsa::LockFreeTaskQueue>(WARMUP_OPS, NUM_THREADS);
  fmt::println("Warmup complete.\n");

  // Single-threaded stdexec schedule benchmark
  fmt::println("--- Single-Threaded stdexec::schedule() ---");
  print_result("Mutex",
               benchmark_stdexec_schedule<dsa::MutexTaskQueue>(NUM_OPS));
  print_result("TAS Spinlock",
               benchmark_stdexec_schedule<dsa::TasSpinlockTaskQueue>(NUM_OPS));
  print_result("TTAS Spinlock",
               benchmark_stdexec_schedule<dsa::SpinlockTaskQueue>(NUM_OPS));
  print_result("Backoff Spinlock",
               benchmark_stdexec_schedule<dsa::BackoffSpinlockTaskQueue>(NUM_OPS));
  print_result("Lock-Free",
               benchmark_stdexec_schedule<dsa::LockFreeTaskQueue>(NUM_OPS));
  fmt::println("");

  // PollingRunLoop benchmark
  fmt::println("--- Single-Threaded PollingRunLoop::schedule() ---");
  print_result("Mutex",
               benchmark_polling_run_loop<dsa::MutexTaskQueue>(NUM_OPS));
  print_result("TAS Spinlock",
               benchmark_polling_run_loop<dsa::TasSpinlockTaskQueue>(NUM_OPS));
  print_result("TTAS Spinlock",
               benchmark_polling_run_loop<dsa::SpinlockTaskQueue>(NUM_OPS));
  print_result("Backoff Spinlock",
               benchmark_polling_run_loop<dsa::BackoffSpinlockTaskQueue>(NUM_OPS));
  print_result("Lock-Free",
               benchmark_polling_run_loop<dsa::LockFreeTaskQueue>(NUM_OPS));
  fmt::println("");

  // Single-threaded with sender composition (then chain)
  fmt::println("--- Single-Threaded schedule() | then() | then() ---");
  print_result("Mutex",
               benchmark_stdexec_then_chain<dsa::MutexTaskQueue>(NUM_OPS));
  print_result("TAS Spinlock",
               benchmark_stdexec_then_chain<dsa::TasSpinlockTaskQueue>(NUM_OPS));
  print_result("TTAS Spinlock",
               benchmark_stdexec_then_chain<dsa::SpinlockTaskQueue>(NUM_OPS));
  print_result("Backoff Spinlock",
               benchmark_stdexec_then_chain<dsa::BackoffSpinlockTaskQueue>(NUM_OPS));
  print_result("Lock-Free",
               benchmark_stdexec_then_chain<dsa::LockFreeTaskQueue>(NUM_OPS));
  fmt::println("");

  // Multi-producer with inline poll
  fmt::println("--- Multi-Producer stdexec, Inline Poll ({} threads) ---",
               NUM_THREADS);
  print_result("Mutex",
               benchmark_stdexec_multi_producer<dsa::MutexTaskQueue>(NUM_OPS, NUM_THREADS));
  print_result("TAS Spinlock",
               benchmark_stdexec_multi_producer<dsa::TasSpinlockTaskQueue>(NUM_OPS, NUM_THREADS));
  print_result("TTAS Spinlock",
               benchmark_stdexec_multi_producer<dsa::SpinlockTaskQueue>(NUM_OPS, NUM_THREADS));
  print_result("Backoff Spinlock",
               benchmark_stdexec_multi_producer<dsa::BackoffSpinlockTaskQueue>(NUM_OPS, NUM_THREADS));
  print_result("Lock-Free",
               benchmark_stdexec_multi_producer<dsa::LockFreeTaskQueue>(NUM_OPS, NUM_THREADS));
  fmt::println("");

  // Multi-producer with background poller
  fmt::println("--- Multi-Producer stdexec, Background Poller ({} threads) ---",
               NUM_THREADS);
  print_result("Mutex",
               benchmark_stdexec_background_poller<dsa::MutexTaskQueue>(NUM_OPS, NUM_THREADS));
  print_result("TAS Spinlock",
               benchmark_stdexec_background_poller<dsa::TasSpinlockTaskQueue>(NUM_OPS, NUM_THREADS));
  print_result("TTAS Spinlock",
               benchmark_stdexec_background_poller<dsa::SpinlockTaskQueue>(NUM_OPS, NUM_THREADS));
  print_result("Backoff Spinlock",
               benchmark_stdexec_background_poller<dsa::BackoffSpinlockTaskQueue>(NUM_OPS, NUM_THREADS));
  print_result("Lock-Free",
               benchmark_stdexec_background_poller<dsa::LockFreeTaskQueue>(NUM_OPS, NUM_THREADS));
  fmt::println("");

  fmt::println("Benchmark completed.");
  return 0;
}
