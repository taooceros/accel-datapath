#include <atomic>
#include <chrono>
#include <cstddef>
#include <cstdio>
#include <fmt/core.h>
#include <stdexec/execution.hpp>
#include <exec/async_scope.hpp>
#include <exec/repeat_effect_until.hpp>
#include <pthread.h>
#include <string>
#include <utility>

#include <dsa/mock_dsa.hpp>
#include <dsa_stdexec/operation_base.hpp>
#include <dsa_stdexec/run_loop.hpp>
#include <dsa_stdexec/sync_wait.hpp>



// Helper to set thread name (max 15 chars + null terminator on Linux)
inline void set_thread_name(const char* name) {
  pthread_setname_np(pthread_self(), name);
}

// Helper to launch N workers in parallel using when_all (zero allocation)
template <std::size_t N, typename F, std::size_t... Is>
auto when_all_n_impl(F&& make_worker, std::index_sequence<Is...>) {
    return stdexec::when_all(make_worker(Is)...);
}

template <std::size_t N, typename F>
auto when_all_n(F&& make_worker) {
    return when_all_n_impl<N>(std::forward<F>(make_worker),
                               std::make_index_sequence<N>{});
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

  MockScheduleOperation(MockDsaBase<QueueTemplate> &dsa, Receiver r)
      : dsa_(dsa), r_(std::move(r)) {
    this->notify_fn = [](dsa_stdexec::OperationBase *base) {
      static_cast<MockScheduleOperation *>(base)->notify();
    };
    this->get_descriptor_fn = [](dsa_stdexec::OperationBase *) -> dsa_hw_desc * { return nullptr; };
  }

  MockScheduleOperation(MockScheduleOperation &&other) noexcept
      : dsa_stdexec::OperationBase(), dsa_(other.dsa_), r_(std::move(other.r_)) {
    this->notify_fn = [](dsa_stdexec::OperationBase *base) {
      static_cast<MockScheduleOperation *>(base)->notify();
    };
    this->get_descriptor_fn = [](dsa_stdexec::OperationBase *) -> dsa_hw_desc * { return nullptr; };
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

// Benchmark: Single-threaded schedule operations using PollingRunLoop
// All operations start from DSA scheduler and run on DSA context thread
template <template <typename> class QueueTemplate>
BenchmarkResult benchmark_schedule(std::size_t num_ops) {
  MockDsaBase<QueueTemplate> mock_dsa(false);
  MockDsaScheduler<QueueTemplate> scheduler(mock_dsa);

  // Create a polling run loop that polls the mock DSA
  dsa_stdexec::PollingRunLoop loop([&mock_dsa] { mock_dsa.poll(); });

  auto start = std::chrono::high_resolution_clock::now();

  // Run each operation synchronously using wait_start
  for (std::size_t i = 0; i < num_ops; ++i) {
    dsa_stdexec::wait_start(scheduler.schedule(), loop);
  }

  auto end = std::chrono::high_resolution_clock::now();
  auto duration = std::chrono::duration<double, std::nano>(end - start).count();

  return {static_cast<double>(num_ops) / (duration / 1e9),
          duration / static_cast<double>(num_ops), num_ops};
}

// Benchmark: Schedule with then chain (schedule | then | then)
// All operations start from DSA scheduler
template <template <typename> class QueueTemplate>
BenchmarkResult benchmark_then_chain(std::size_t num_ops) {
  MockDsaBase<QueueTemplate> mock_dsa(false);
  MockDsaScheduler<QueueTemplate> scheduler(mock_dsa);

  dsa_stdexec::PollingRunLoop loop([&mock_dsa] { mock_dsa.poll(); });

  auto start = std::chrono::high_resolution_clock::now();

  // Run chained operations synchronously
  for (std::size_t i = 0; i < num_ops; ++i) {
    dsa_stdexec::wait_start(
        scheduler.schedule()
        | stdexec::then([] { return 42; })
        | stdexec::then([](int x) { return x * 2; }),
        loop);
  }

  auto end = std::chrono::high_resolution_clock::now();
  auto duration = std::chrono::duration<double, std::nano>(end - start).count();

  return {static_cast<double>(num_ops) / (duration / 1e9),
          duration / static_cast<double>(num_ops), num_ops};
}

// Benchmark: Schedule with async_scope (for comparison)
// Uses async_scope to track spawned work - allows batching
template <template <typename> class QueueTemplate>
BenchmarkResult benchmark_async_scope(std::size_t num_ops) {
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

  // Wait for scope to be empty
  dsa_stdexec::PollingRunLoop loop([&mock_dsa] { mock_dsa.poll(); });
  dsa_stdexec::wait_start(scope.on_empty(), loop);

  auto end = std::chrono::high_resolution_clock::now();
  auto duration = std::chrono::duration<double, std::nano>(end - start).count();

  return {static_cast<double>(num_ops) / (duration / 1e9),
          duration / static_cast<double>(num_ops), num_ops};
}

// Benchmark: Static workers using when_all + repeat_effect_until
// N workers run in parallel, each processing items sequentially
// Zero allocation - avoids async_scope::spawn overhead
// Worker count must be known at compile time
template <template <typename> class QueueTemplate, std::size_t NumWorkers = 4>
BenchmarkResult benchmark_static_workers(std::size_t num_ops) {
  MockDsaBase<QueueTemplate> mock_dsa(false);
  MockDsaScheduler<QueueTemplate> scheduler(mock_dsa);

  dsa_stdexec::PollingRunLoop loop([&mock_dsa] { mock_dsa.poll(); });


  // Create a worker that processes items: worker_id, worker_id + NumWorkers, ...
  auto make_worker = [&](std::size_t worker_id) {
    return scheduler.schedule()
         | stdexec::let_value([&, worker_id, current_idx = worker_id]() mutable {
             // Use repeat_effect_until to loop until all items processed
             return exec::repeat_effect_until(
                 scheduler.schedule()
               | stdexec::then([num_ops, &current_idx]() mutable {

                   // Move to next item for this worker
                   current_idx += NumWorkers;

                   // Return true when done (no more items for this worker)
                   return current_idx >= num_ops;
                 })
             );
           });
  };

  auto start = std::chrono::high_resolution_clock::now();


  // Launch all workers in parallel (compile-time count = no allocation)
  dsa_stdexec::wait_start(when_all_n<NumWorkers>(make_worker), loop);

  auto end = std::chrono::high_resolution_clock::now();
  auto duration = std::chrono::duration<double, std::nano>(end - start).count();

  return {static_cast<double>(num_ops) / (duration / 1e9),
          duration / static_cast<double>(num_ops), num_ops};
}

// Benchmark: Scoped workers using async_scope + repeat_effect_until
// Workers are spawned via async_scope (N allocations), but each worker
// processes its chunk sequentially with repeat_effect_until (no per-item alloc)
// This allows runtime-determined worker count while still avoiding per-op allocation
template <template <typename> class QueueTemplate>
BenchmarkResult benchmark_scoped_workers(std::size_t num_ops, std::size_t num_workers = 16) {
  MockDsaBase<QueueTemplate> mock_dsa(false);
  MockDsaScheduler<QueueTemplate> scheduler(mock_dsa);

  exec::async_scope scope;

  auto start = std::chrono::high_resolution_clock::now();

  // Spawn N workers using async_scope (N allocations total)
  for (std::size_t worker_id = 0; worker_id < num_workers; ++worker_id) {
    scope.spawn(
        scheduler.schedule()
      | stdexec::let_value([&, worker_id, current_idx = worker_id]() mutable {
          // Each worker loops with repeat_effect_until (no allocation per iteration)
          return exec::repeat_effect_until(
              scheduler.schedule()
            | stdexec::then([num_ops, num_workers, &current_idx]() mutable {
                // Move to next item for this worker
                current_idx += num_workers;

                // Return true when done (no more items for this worker)
                return current_idx >= num_ops;
              })
          );
        })
    );
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
  fmt::println("  all        - Run all benchmarks (default)");
  fmt::println("  schedule   - Single-threaded schedule (no async_scope)");
  fmt::println("  chain      - schedule | then | then chain");
  fmt::println("  async_scope- Schedule with async_scope overhead");
  fmt::println("  static     - Static workers with when_all (zero alloc, compile-time N)");
  fmt::println("  scoped     - Scoped workers with async_scope (N allocs, runtime N)");
  fmt::println("");
  fmt::println("Examples:");
  fmt::println("  {} mutex schedule   # Profile mutex queue schedule", prog);
  fmt::println("  {} lockfree         # Profile lock-free queue, all benchmarks", prog);
  fmt::println("  samply record {} mutex schedule", prog);
}

// Run benchmark for a specific queue type
template <template <typename> class QueueTemplate>
void run_benchmarks_for_queue(const char* queue_name,
                               const std::string& benchmark_type,
                               std::size_t num_ops,
                               std::size_t num_threads,
                               std::size_t warmup_ops) {
  (void)num_threads; // No longer used

  // Set main thread name
  char main_name[16];
  snprintf(main_name, sizeof(main_name), "%s_main", queue_name);
  set_thread_name(main_name);

  bool run_all = (benchmark_type == "all");

  // Warmup
  fmt::println("--- Warmup ({} ops) for {} ---", warmup_ops, queue_name);
  if (run_all || benchmark_type == "schedule") {
    benchmark_schedule<QueueTemplate>(warmup_ops);
  }
  if (run_all || benchmark_type == "chain") {
    benchmark_then_chain<QueueTemplate>(warmup_ops);
  }
  if (run_all || benchmark_type == "async_scope") {
    benchmark_async_scope<QueueTemplate>(warmup_ops);
  }
  if (run_all || benchmark_type == "static") {
    benchmark_static_workers<QueueTemplate>(warmup_ops);
  }
  if (run_all || benchmark_type == "scoped") {
    benchmark_scoped_workers<QueueTemplate>(warmup_ops);
  }
  fmt::println("Warmup complete.\n");

  // Run actual benchmarks
  if (run_all || benchmark_type == "schedule") {
    fmt::println("--- Single-Threaded schedule() ---");
    print_result(queue_name, benchmark_schedule<QueueTemplate>(num_ops));
    fmt::println("");
  }

  if (run_all || benchmark_type == "chain") {
    fmt::println("--- Single-Threaded schedule() | then() | then() ---");
    print_result(queue_name, benchmark_then_chain<QueueTemplate>(num_ops));
    fmt::println("");
  }

  if (run_all || benchmark_type == "async_scope") {
    fmt::println("--- Single-Threaded with async_scope ---");
    print_result(queue_name, benchmark_async_scope<QueueTemplate>(num_ops));
    fmt::println("");
  }

  if (run_all || benchmark_type == "static") {
    fmt::println("--- Static Workers (when_all + repeat_effect_until) ---");
    print_result(queue_name, benchmark_static_workers<QueueTemplate>(num_ops));
    fmt::println("");
  }

  if (run_all || benchmark_type == "scoped") {
    fmt::println("--- Scoped Workers (async_scope + repeat_effect_until) ---");
    print_result(queue_name, benchmark_scoped_workers<QueueTemplate>(num_ops));
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
