#include <atomic>
#include <chrono>
#include <cstddef>
#include <fmt/core.h>
#include <functional>
#include <numbers>
#include <stdexec/execution.hpp>
#include <exec/async_scope.hpp>
#include <pthread.h>
#include <thread>
#include <vector>

#include <dsa/mock_dsa.hpp>
#include <dsa_stdexec/operation_base.hpp>
#include <dsa_stdexec/run_loop.hpp>
#include <dsa_stdexec/sync_wait.hpp>

// Helper to set thread name (max 15 chars + null terminator on Linux)
inline void set_thread_name(const char* name) {
  pthread_setname_np(pthread_self(), name);
}

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
                                                  std::size_t num_threads,
                                                  const char* thread_name = "mp_producer") {
  MockDsaBase<QueueTemplate> mock_dsa(false);
  MockDsaScheduler<QueueTemplate> scheduler(mock_dsa);

  exec::async_scope scope;
  std::atomic<std::size_t> op_index{0};
  std::atomic<std::size_t> completed{0};

  auto start = std::chrono::high_resolution_clock::now();

  // Producer threads
  std::vector<std::thread> producers;
  for (std::size_t t = 0; t < num_threads; ++t) {
    producers.emplace_back([&, t, thread_name] {
      // Set thread name with index (e.g., "mp_mutex_0")
      char name[16];
      snprintf(name, sizeof(name), "%s_%zu", thread_name, t);
      set_thread_name(name);
      
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
                                                     std::size_t num_threads,
                                                     const char* thread_name = "bg_producer") {
  MockDsaBase<QueueTemplate> mock_dsa(true); // Start background poller
  MockDsaScheduler<QueueTemplate> scheduler(mock_dsa);

  exec::async_scope scope;
  std::atomic<std::size_t> op_index{0};
  std::atomic<std::size_t> completed{0};

  auto start = std::chrono::high_resolution_clock::now();

  // Producer threads
  std::vector<std::thread> producers;
  for (std::size_t t = 0; t < num_threads; ++t) {
    producers.emplace_back([&, t, thread_name] {
      // Set thread name with index (e.g., "bg_mutex_0")
      char name[16];
      snprintf(name, sizeof(name), "%s_%zu", thread_name, t);
      set_thread_name(name);
      
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

void print_usage(const char* prog) {
  fmt::println("Usage: {} [queue_type] [benchmark_type]", prog);
  fmt::println("");
  fmt::println("Queue types:");
  fmt::println("  all       - Run all queue types (default)");
  fmt::println("  mutex     - std::mutex based queue");
  fmt::println("  tas       - Test-and-set spinlock queue");
  fmt::println("  ttas      - Test-and-test-and-set spinlock queue");
  fmt::println("  backoff   - Backoff spinlock queue");
  fmt::println("  lockfree  - Lock-free queue");
  fmt::println("");
  fmt::println("Benchmark types:");
  fmt::println("  all       - Run all benchmarks (default)");
  fmt::println("  schedule  - Single-threaded schedule");
  fmt::println("  runloop   - PollingRunLoop schedule");
  fmt::println("  chain     - schedule | then | then chain");
  fmt::println("  multi     - Multi-producer inline poll");
  fmt::println("  background- Multi-producer background poller");
  fmt::println("");
  fmt::println("Examples:");
  fmt::println("  {} mutex multi      # Profile mutex queue with multi-producer", prog);
  fmt::println("  {} lockfree         # Profile lock-free queue, all benchmarks", prog);
  fmt::println("  samply record {} mutex multi", prog);
}

// Run benchmark for a specific queue type
template <template <typename> class QueueTemplate>
void run_benchmarks_for_queue(const char* queue_name, 
                               const std::string& benchmark_type,
                               std::size_t num_ops,
                               std::size_t num_threads,
                               std::size_t warmup_ops) {
  // Set main thread name
  char main_name[16];
  snprintf(main_name, sizeof(main_name), "%s_main", queue_name);
  set_thread_name(main_name);

  bool run_all = (benchmark_type == "all");
  
  // Warmup
  fmt::println("--- Warmup ({} ops) for {} ---", warmup_ops, queue_name);
  if (run_all || benchmark_type == "schedule") {
    benchmark_stdexec_schedule<QueueTemplate>(warmup_ops);
  }
  if (run_all || benchmark_type == "runloop") {
    benchmark_polling_run_loop<QueueTemplate>(warmup_ops);
  }
  if (run_all || benchmark_type == "chain") {
    benchmark_stdexec_then_chain<QueueTemplate>(warmup_ops);
  }
  if (run_all || benchmark_type == "multi") {
    benchmark_stdexec_multi_producer<QueueTemplate>(warmup_ops, num_threads, queue_name);
  }
  if (run_all || benchmark_type == "background") {
    benchmark_stdexec_background_poller<QueueTemplate>(warmup_ops, num_threads, queue_name);
  }
  fmt::println("Warmup complete.\n");

  // Run actual benchmarks
  if (run_all || benchmark_type == "schedule") {
    fmt::println("--- Single-Threaded stdexec::schedule() ---");
    print_result(queue_name, benchmark_stdexec_schedule<QueueTemplate>(num_ops));
    fmt::println("");
  }
  
  if (run_all || benchmark_type == "runloop") {
    fmt::println("--- Single-Threaded PollingRunLoop::schedule() ---");
    print_result(queue_name, benchmark_polling_run_loop<QueueTemplate>(num_ops));
    fmt::println("");
  }
  
  if (run_all || benchmark_type == "chain") {
    fmt::println("--- Single-Threaded schedule() | then() | then() ---");
    print_result(queue_name, benchmark_stdexec_then_chain<QueueTemplate>(num_ops));
    fmt::println("");
  }
  
  if (run_all || benchmark_type == "multi") {
    fmt::println("--- Multi-Producer stdexec, Inline Poll ({} threads) ---", num_threads);
    print_result(queue_name, benchmark_stdexec_multi_producer<QueueTemplate>(num_ops, num_threads, queue_name));
    fmt::println("");
  }
  
  if (run_all || benchmark_type == "background") {
    fmt::println("--- Multi-Producer stdexec, Background Poller ({} threads) ---", num_threads);
    print_result(queue_name, benchmark_stdexec_background_poller<QueueTemplate>(num_ops, num_threads, queue_name));
    fmt::println("");
  }
}

int main(int argc, char* argv[]) {
  constexpr std::size_t NUM_OPS = 1000000;
  constexpr std::size_t NUM_THREADS = 4;
  constexpr std::size_t WARMUP_OPS = 10000;

  std::string queue_type = "all";
  std::string benchmark_type = "all";

  if (argc > 1) {
    std::string arg1 = argv[1];
    if (arg1 == "-h" || arg1 == "--help") {
      print_usage(argv[0]);
      return 0;
    }
    queue_type = arg1;
  }
  if (argc > 2) {
    benchmark_type = argv[2];
  }

  fmt::println("=== TASK QUEUE BENCHMARK (stdexec Interface) ===\n");
  fmt::println("Operations: {}, Threads: {}", NUM_OPS, NUM_THREADS);
  fmt::println("Queue type: {}, Benchmark: {}\n", queue_type, benchmark_type);

  bool run_all_queues = (queue_type == "all");

  if (run_all_queues || queue_type == "mutex") {
    run_benchmarks_for_queue<dsa::MutexTaskQueue>("mutex", benchmark_type, NUM_OPS, NUM_THREADS, WARMUP_OPS);
  }
  if (run_all_queues || queue_type == "tas") {
    run_benchmarks_for_queue<dsa::TasSpinlockTaskQueue>("tas", benchmark_type, NUM_OPS, NUM_THREADS, WARMUP_OPS);
  }
  if (run_all_queues || queue_type == "ttas") {
    run_benchmarks_for_queue<dsa::SpinlockTaskQueue>("ttas", benchmark_type, NUM_OPS, NUM_THREADS, WARMUP_OPS);
  }
  if (run_all_queues || queue_type == "backoff") {
    run_benchmarks_for_queue<dsa::BackoffSpinlockTaskQueue>("backoff", benchmark_type, NUM_OPS, NUM_THREADS, WARMUP_OPS);
  }
  if (run_all_queues || queue_type == "lockfree") {
    run_benchmarks_for_queue<dsa::LockFreeTaskQueue>("lockfree", benchmark_type, NUM_OPS, NUM_THREADS, WARMUP_OPS);
  }

  fmt::println("Benchmark completed.");
  return 0;
}
